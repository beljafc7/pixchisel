use std::io::Cursor;

use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    DynamicImage, ExtendedColorType, ImageEncoder,
};

use super::{settings::OutputFormat, TransformError};

const WEBP_MAX_DIMENSION: u32 = 16_383;

pub fn encode_image(
    image: &DynamicImage,
    format: OutputFormat,
    quality: u8,
) -> Result<Vec<u8>, TransformError> {
    match format {
        OutputFormat::Jpeg => encode_jpeg(image, quality),
        OutputFormat::Png => encode_png(image),
        OutputFormat::Webp => encode_webp(image, quality),
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
    JpegEncoder::new_with_quality(&mut bytes, quality)
        .write_image(&rgb, image.width(), image.height(), ExtendedColorType::Rgb8)
        .map_err(|_| TransformError::EncodeFailed)?;
    Ok(bytes)
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, TransformError> {
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
    Ok(bytes.into_inner())
}

fn encode_webp(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, TransformError> {
    if image.width() > WEBP_MAX_DIMENSION || image.height() > WEBP_MAX_DIMENSION {
        return Err(TransformError::WebpDimensionLimitExceeded);
    }

    let rgba = image.to_rgba8();
    Ok(
        webp::Encoder::from_rgba(rgba.as_raw(), image.width(), image.height())
            .encode(quality as f32)
            .to_vec(),
    )
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
}
