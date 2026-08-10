use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use image::{codecs::png::PngEncoder, DynamicImage, ExtendedColorType, ImageEncoder};
use jpeg_encoder::{ColorType as JpegColorType, Encoder as JpegEncoder, SamplingFactor};

use super::{settings::OutputFormat, TransformError};

const WEBP_MAX_DIMENSION: u32 = 16_383;

pub fn encode_image(
    image: &DynamicImage,
    format: OutputFormat,
    quality: u8,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    match format {
        OutputFormat::Jpeg => encode_jpeg(image, quality),
        OutputFormat::Png => encode_png(image, quality),
        OutputFormat::Webp => encode_webp(image, quality, should_cancel),
        OutputFormat::Original => Err(TransformError::UnsupportedOutputFormat),
    }
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, TransformError> {
    let rgba = image.to_rgba8();
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for pixel in rgba.pixels() {
        let alpha = u32::from(pixel[3]);
        for channel in &pixel.0[..3] {
            let composited = (u32::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255;
            rgb.push(composited as u8);
        }
    }

    let mut bytes = Vec::new();
    let width = u16::try_from(image.width()).map_err(|_| TransformError::EncodeFailed)?;
    let height = u16::try_from(image.height()).map_err(|_| TransformError::EncodeFailed)?;
    let mut encoder = JpegEncoder::new(&mut bytes, quality);
    encoder.set_sampling_factor(SamplingFactor::F_1_1);
    encoder.set_progressive(true);
    encoder.set_optimized_huffman_tables(true);
    encoder
        .encode(&rgb, width, height, JpegColorType::Rgb)
        .map_err(|_| TransformError::EncodeFailed)?;
    Ok(bytes)
}

fn encode_png(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, TransformError> {
    let rgba = image.to_rgba8();
    let mut bytes = Cursor::new(Vec::new());
    PngEncoder::new(&mut bytes)
        .write_image(
            rgba.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|_| TransformError::EncodeFailed)?;
    let encoded = bytes.into_inner();
    let preset = png_optimization_preset(quality);
    let mut options = oxipng::Options::from_preset(preset);
    options.strip = oxipng::StripChunks::All;
    options.optimize_alpha = false;
    oxipng::optimize_from_memory(&encoded, &options).map_err(|_| TransformError::EncodeFailed)
}

fn png_optimization_preset(quality: u8) -> u8 {
    match quality {
        82..=u8::MAX => 2,
        60..=81 => 4,
        _ => 6,
    }
}

fn encode_webp(
    image: &DynamicImage,
    quality: u8,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    if image.width() > WEBP_MAX_DIMENSION || image.height() > WEBP_MAX_DIMENSION {
        return Err(TransformError::WebpDimensionLimitExceeded);
    }

    let rgba = image.to_rgba8();
    if should_cancel() {
        return Err(TransformError::Cancelled);
    }
    let mut config = libwebp_sys::WebPConfig::new().map_err(|_| TransformError::EncodeFailed)?;
    config.lossless = 0;
    config.alpha_compression = 1;
    config.quality = quality as f32;
    if unsafe { libwebp_sys::WebPValidateConfig(&config) } == 0 {
        return Err(TransformError::EncodeFailed);
    }

    struct PictureGuard(libwebp_sys::WebPPicture);
    impl Drop for PictureGuard {
        fn drop(&mut self) {
            unsafe { libwebp_sys::WebPPictureFree(&mut self.0) };
        }
    }

    struct CallbackContext<'a> {
        output: Vec<u8>,
        should_cancel: &'a dyn Fn() -> bool,
    }

    unsafe extern "C" fn write_output(
        data: *const u8,
        data_size: usize,
        picture: *const libwebp_sys::WebPPicture,
    ) -> std::ffi::c_int {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let context = &mut *((*picture).custom_ptr as *mut CallbackContext<'_>);
            context
                .output
                .extend_from_slice(std::slice::from_raw_parts(data, data_size));
        }));
        if result.is_ok() {
            1
        } else {
            0
        }
    }

    unsafe extern "C" fn report_progress(
        _percent: std::ffi::c_int,
        picture: *const libwebp_sys::WebPPicture,
    ) -> std::ffi::c_int {
        let cancelled = catch_unwind(AssertUnwindSafe(|| {
            let context = &*((*picture).user_data as *const CallbackContext<'_>);
            (context.should_cancel)()
        }))
        .unwrap_or(true);
        if cancelled {
            0
        } else {
            1
        }
    }

    let mut picture =
        PictureGuard(libwebp_sys::WebPPicture::new().map_err(|_| TransformError::EncodeFailed)?);
    picture.0.use_argb = 1;
    picture.0.width = image.width() as i32;
    picture.0.height = image.height() as i32;
    if unsafe {
        libwebp_sys::WebPPictureImportRGBA(&mut picture.0, rgba.as_ptr(), image.width() as i32 * 4)
    } == 0
    {
        return Err(TransformError::EncodeFailed);
    }
    if should_cancel() {
        return Err(TransformError::Cancelled);
    }

    let mut context = CallbackContext {
        output: Vec::new(),
        should_cancel,
    };
    picture.0.writer = Some(write_output);
    picture.0.progress_hook = Some(report_progress);
    picture.0.custom_ptr = &mut context as *mut _ as *mut std::ffi::c_void;
    picture.0.user_data = &mut context as *mut _ as *mut std::ffi::c_void;
    let encoded = unsafe { libwebp_sys::WebPEncode(&config, &mut picture.0) };
    if encoded == 0 {
        return if should_cancel() {
            Err(TransformError::Cancelled)
        } else {
            Err(TransformError::EncodeFailed)
        };
    }
    Ok(context.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn transparent_pixels_are_white_in_jpeg() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([9, 20, 30, 0])));
        let encoded = encode_jpeg(&image, 100).unwrap();
        let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        assert!(decoded
            .get_pixel(0, 0)
            .0
            .iter()
            .all(|channel| *channel >= 250));
    }

    #[test]
    fn opaque_pixels_are_not_alpha_blended() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([220, 40, 10, 255])));
        let encoded = encode_jpeg(&image, 100).unwrap();
        let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        let pixel = decoded.get_pixel(0, 0);
        assert!(pixel[0] > 190 && pixel[1] < 70 && pixel[2] < 40);
    }

    #[test]
    fn jpeg_quality_boundaries_encode_decodable_images() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(3, 2, Rgba([120, 80, 40, 255])));
        for quality in [1, 100] {
            let encoded = encode_jpeg(&image, quality).unwrap();
            let decoded =
                image::load_from_memory_with_format(&encoded, image::ImageFormat::Jpeg).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (3, 2));
        }
    }

    #[test]
    fn jpeg_compression_presets_are_decodable_and_meaningfully_differentiated() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(256, 192, |x, y| {
            let detail = x.wrapping_mul(37).wrapping_add(y.wrapping_mul(73)) as u8;
            Rgba([detail, detail.rotate_left(2), detail.rotate_left(5), 255])
        }));
        let outputs = [82, 65, 45].map(|quality| encode_jpeg(&image, quality).unwrap());
        for output in &outputs {
            let decoded =
                image::load_from_memory_with_format(output, image::ImageFormat::Jpeg).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (256, 192));
        }
        assert!(outputs[0].len() > outputs[1].len());
        assert!(outputs[1].len() > outputs[2].len());
        assert!(outputs[0].len() - outputs[2].len() > outputs[0].len() / 10);
    }

    #[test]
    fn png_presets_are_distinct_and_preserve_every_alpha_level_exactly() {
        assert_eq!(png_optimization_preset(82), 2);
        assert_eq!(png_optimization_preset(65), 4);
        assert_eq!(png_optimization_preset(45), 6);
        let source = RgbaImage::from_fn(96, 64, |x, y| {
            let alpha = match x % 4 {
                0 => 0,
                1 => 64,
                2 => 160,
                _ => 255,
            };
            Rgba([(x * 7) as u8, (y * 11) as u8, ((x + y) * 3) as u8, alpha])
        });
        let image = DynamicImage::ImageRgba8(source.clone());
        for quality in [82, 65, 45] {
            let encoded = encode_png(&image, quality).unwrap();
            let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
            assert_eq!(decoded.dimensions(), source.dimensions());
            assert_eq!(decoded, source);
        }
    }

    #[test]
    fn png_optimization_preserves_representative_opaque_content_exactly() {
        let cases = [
            RgbaImage::from_fn(128, 96, |x, y| {
                let band = if (x / 16 + y / 16) % 2 == 0 { 24 } else { 232 };
                Rgba([band, 80, 190, 255])
            }),
            RgbaImage::from_fn(128, 96, |x, y| {
                let value = ((x * 255) / 127) as u8;
                let vertical = ((y * 255) / 95) as u8;
                Rgba([value, vertical, value.saturating_add(vertical) / 2, 255])
            }),
            RgbaImage::from_fn(128, 96, |x, y| {
                let noise = x
                    .wrapping_mul(1_664_525)
                    .wrapping_add(y.wrapping_mul(1_013_904_223));
                Rgba([
                    noise as u8,
                    noise.rotate_left(9) as u8,
                    noise.rotate_left(17) as u8,
                    255,
                ])
            }),
        ];

        for source in cases {
            let image = DynamicImage::ImageRgba8(source.clone());
            for quality in [82, 65, 45] {
                let encoded = encode_png(&image, quality).unwrap();
                let decoded =
                    image::load_from_memory_with_format(&encoded, image::ImageFormat::Png)
                        .unwrap()
                        .to_rgba8();
                assert_eq!(decoded.dimensions(), source.dimensions());
                assert_eq!(decoded, source);
            }
        }
    }

    #[test]
    fn webp_progress_hook_aborts_an_active_encode() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(1024, 768, |x, y| {
            let detail = x
                .wrapping_mul(1_664_525)
                .wrapping_add(y.wrapping_mul(1_013_904_223));
            Rgba([
                detail as u8,
                detail.rotate_left(9) as u8,
                detail.rotate_left(17) as u8,
                255,
            ])
        }));

        assert_eq!(
            encode_webp(&image, 82, &|| true).unwrap_err(),
            TransformError::Cancelled
        );
    }
}
