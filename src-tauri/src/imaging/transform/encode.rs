#[cfg(test)]
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use color_quant::NeuQuant;
use image::DynamicImage;
use jpeg_encoder::{
    ColorType as JpegColorType, Encoder as JpegEncoder, PixelDensity, PixelDensityUnit,
    SamplingFactor,
};

use super::{
    resolution::{PhysicalResolution, ResolutionUnit},
    settings::OutputFormat,
    TransformError,
};

const WEBP_MAX_DIMENSION: u32 = 16_383;
const MAXIMUM_PNG_COLORS: usize = 256;
const MAXIMUM_PNG_SAMPLE_FACTOR: i32 = 10;

#[cfg(test)]
pub fn encode_image(
    image: &DynamicImage,
    format: OutputFormat,
    quality: u8,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    encode_image_with_resolution(image, format, quality, None, should_cancel)
}

pub fn encode_image_with_resolution(
    image: &DynamicImage,
    format: OutputFormat,
    quality: u8,
    physical_resolution: Option<PhysicalResolution>,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    match format {
        OutputFormat::Jpeg => encode_jpeg_with_resolution(image, quality, physical_resolution),
        OutputFormat::Png => {
            encode_png_with_resolution(image, quality, physical_resolution, should_cancel)
        }
        OutputFormat::Webp => encode_webp(image, quality, should_cancel),
        OutputFormat::Original => Err(TransformError::UnsupportedOutputFormat),
    }
}

#[cfg(test)]
fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, TransformError> {
    encode_jpeg_with_resolution(image, quality, None)
}

fn encode_jpeg_with_resolution(
    image: &DynamicImage,
    quality: u8,
    physical_resolution: Option<PhysicalResolution>,
) -> Result<Vec<u8>, TransformError> {
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
    if let Some(resolution) = jpeg_density(physical_resolution) {
        encoder.set_density(resolution);
    }
    encoder.set_sampling_factor(SamplingFactor::F_1_1);
    encoder.set_progressive(true);
    encoder.set_optimized_huffman_tables(true);
    encoder
        .encode(&rgb, width, height, JpegColorType::Rgb)
        .map_err(|_| TransformError::EncodeFailed)?;
    Ok(bytes)
}

#[cfg(test)]
fn encode_png(
    image: &DynamicImage,
    quality: u8,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    encode_png_with_resolution(image, quality, None, should_cancel)
}

fn encode_png_with_resolution(
    image: &DynamicImage,
    quality: u8,
    physical_resolution: Option<PhysicalResolution>,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    let rgba = image.to_rgba8();
    let encoded = if quality >= 82 {
        encode_lossless_png(&rgba, image.width(), image.height(), physical_resolution)?
    } else {
        encode_indexed_png(
            &rgba,
            image.width(),
            image.height(),
            physical_resolution,
            should_cancel,
        )?
    };
    if should_cancel() {
        return Err(TransformError::Cancelled);
    }
    let mut options = oxipng::Options::from_preset(2);
    options.strip = if png_dimensions(physical_resolution).is_some() {
        oxipng::StripChunks::Keep([*b"pHYs"].into_iter().collect())
    } else {
        oxipng::StripChunks::All
    };
    options.optimize_alpha = false;
    oxipng::optimize_from_memory(&encoded, &options).map_err(|_| TransformError::EncodeFailed)
}

fn encode_lossless_png(
    rgba: &image::RgbaImage,
    width: u32,
    height: u32,
    physical_resolution: Option<PhysicalResolution>,
) -> Result<Vec<u8>, TransformError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_pixel_dims(png_dimensions(physical_resolution));
        let mut writer = encoder
            .write_header()
            .map_err(|_| TransformError::EncodeFailed)?;
        writer
            .write_image_data(rgba.as_raw())
            .map_err(|_| TransformError::EncodeFailed)?;
    }
    Ok(bytes)
}

fn encode_indexed_png(
    rgba: &image::RgbaImage,
    width: u32,
    height: u32,
    physical_resolution: Option<PhysicalResolution>,
    should_cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, TransformError> {
    if should_cancel() {
        return Err(TransformError::Cancelled);
    }

    let quantizer = NeuQuant::new(MAXIMUM_PNG_SAMPLE_FACTOR, MAXIMUM_PNG_COLORS, rgba.as_raw());
    if should_cancel() {
        return Err(TransformError::Cancelled);
    }

    let rgba_palette = quantizer.color_map_rgba();
    let rgb_palette = rgba_palette
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|color| color[..3].iter().copied())
        .collect::<Vec<_>>();
    let alpha_palette = quantizer.color_map_alpha();
    let mut indices = Vec::with_capacity((u64::from(width) * u64::from(height)) as usize);
    for (index, pixel) in rgba.as_raw().as_chunks::<4>().0.iter().enumerate() {
        if index % 16_384 == 0 && should_cancel() {
            return Err(TransformError::Cancelled);
        }
        indices.push(quantizer.index_of(pixel) as u8);
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_pixel_dims(png_dimensions(physical_resolution));
        encoder.set_palette(rgb_palette);
        if alpha_palette.iter().any(|alpha| *alpha != u8::MAX) {
            encoder.set_trns(alpha_palette);
        }
        let mut writer = encoder
            .write_header()
            .map_err(|_| TransformError::EncodeFailed)?;
        writer
            .write_image_data(&indices)
            .map_err(|_| TransformError::EncodeFailed)?;
    }
    Ok(bytes)
}

fn jpeg_density(resolution: Option<PhysicalResolution>) -> Option<PixelDensity> {
    let resolution = resolution?;
    let horizontal = u16::try_from(resolution.horizontal).ok()?;
    let vertical = u16::try_from(resolution.vertical).ok()?;
    let unit = match resolution.unit {
        ResolutionUnit::Unspecified => PixelDensityUnit::PixelAspectRatio,
        ResolutionUnit::Inches => PixelDensityUnit::Inches,
        ResolutionUnit::Centimeters => PixelDensityUnit::Centimeters,
        ResolutionUnit::Meters => return None,
    };
    Some(PixelDensity {
        density: (horizontal, vertical),
        unit,
    })
}

fn png_dimensions(resolution: Option<PhysicalResolution>) -> Option<png::PixelDimensions> {
    let resolution = resolution?;
    let unit = match resolution.unit {
        ResolutionUnit::Unspecified => png::Unit::Unspecified,
        ResolutionUnit::Meters => png::Unit::Meter,
        ResolutionUnit::Inches | ResolutionUnit::Centimeters => return None,
    };
    Some(png::PixelDimensions {
        xppu: resolution.horizontal,
        yppu: resolution.vertical,
        unit,
    })
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
    let config = webp_config(quality)?;
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

fn webp_config(quality: u8) -> Result<libwebp_sys::WebPConfig, TransformError> {
    let mut config = libwebp_sys::WebPConfig::new().map_err(|_| TransformError::EncodeFailed)?;
    config.lossless = 0;
    config.alpha_compression = 1;
    config.quality = quality as f32;
    config.method = 2;
    Ok(config)
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
        let outputs = [82, 45].map(|quality| encode_jpeg(&image, quality).unwrap());
        for output in &outputs {
            let decoded =
                image::load_from_memory_with_format(output, image::ImageFormat::Jpeg).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (256, 192));
        }
        assert!(outputs[0].len() > outputs[1].len());
        assert!(outputs[0].len() - outputs[1].len() > outputs[0].len() / 10);
    }

    #[test]
    fn standard_png_preserves_every_alpha_level_exactly() {
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
        let encoded = encode_png(&image, 82, &|| false).unwrap();
        let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded, source);
    }

    #[test]
    fn standard_png_preserves_representative_opaque_content_exactly() {
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
            let encoded = encode_png(&image, 82, &|| false).unwrap();
            let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
            assert_eq!(decoded, source);
        }
    }

    #[test]
    fn maximum_png_is_indexed_smaller_and_decodable() {
        let source = RgbaImage::from_fn(384, 256, |x, y| {
            let mut detail = x.wrapping_add(y.wrapping_mul(384));
            detail ^= detail >> 16;
            detail = detail.wrapping_mul(0x7feb_352d);
            detail ^= detail >> 15;
            detail = detail.wrapping_mul(0x846c_a68b);
            detail ^= detail >> 16;
            Rgba([
                detail as u8,
                detail.rotate_left(9) as u8,
                detail.rotate_left(17) as u8,
                if x % 11 == 0 { 96 } else { 255 },
            ])
        });
        let image = DynamicImage::ImageRgba8(source.clone());
        let standard = encode_png(&image, 82, &|| false).unwrap();
        let maximum = encode_png(&image, 45, &|| false).unwrap();

        let reader = png::Decoder::new(Cursor::new(&maximum))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Indexed);
        let decoded = image::load_from_memory_with_format(&maximum, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded.dimensions(), source.dimensions());
        assert!(decoded.pixels().any(|pixel| pixel[3] < u8::MAX));
        assert!(decoded.pixels().any(|pixel| pixel[3] == u8::MAX));
        assert!(
            maximum.len() < standard.len(),
            "maximum PNG should be smaller: standard={}, maximum={}",
            standard.len(),
            maximum.len()
        );
        assert_ne!(decoded, source);
    }

    #[test]
    fn maximum_png_honors_cancellation_before_quantizing() {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(32, 32, Rgba([120, 80, 40, 255])));
        assert_eq!(
            encode_png(&image, 45, &|| true).unwrap_err(),
            TransformError::Cancelled
        );
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

    #[test]
    fn webp_uses_the_benchmarked_fast_method_without_changing_quality() {
        let config = webp_config(92).unwrap();
        assert_eq!(config.method, 2);
        assert_eq!(config.quality, 92.0);
        assert_eq!(config.lossless, 0);
        assert_eq!(config.alpha_compression, 1);
    }
}
