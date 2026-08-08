mod encode;
mod resize;
pub mod settings;

use std::{fs::File, io::BufReader, path::Path};

use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use serde::Serialize;

use encode::encode_image;
use resize::{calculate_target, Dimensions, ResizeError};
pub use settings::{
    BatchSettings, OutputFormat, ResizeMode, ResizeSettings, ValidationError, ValidationErrorCode,
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MetadataDisposition {
    Removed,
    DiscardedUnsupported,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformationMetadata {
    pub input_format: OutputFormat,
    pub output_format: OutputFormat,
    pub original_width: u32,
    pub original_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub encoded_size_bytes: u64,
    pub metadata_disposition: MetadataDisposition,
}

#[derive(Debug)]
pub struct EncodedTransformation {
    pub bytes: Vec<u8>,
    pub metadata: TransformationMetadata,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TransformError {
    InvalidSettings(Vec<settings::ValidationError>),
    FileNotFound,
    PermissionDenied,
    UnsupportedInputFormat,
    UnsupportedOutputFormat,
    DecodeFailed,
    InvalidSourceDimensions,
    DimensionLimitExceeded,
    ArithmeticOverflow,
    WebpDimensionLimitExceeded,
    EncodeFailed,
}

pub fn transform_image(
    path: &Path,
    settings: &BatchSettings,
) -> Result<EncodedTransformation, TransformError> {
    settings
        .validate()
        .map_err(TransformError::InvalidSettings)?;

    let file = File::open(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => TransformError::FileNotFound,
        std::io::ErrorKind::PermissionDenied => TransformError::PermissionDenied,
        _ => TransformError::DecodeFailed,
    })?;
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|_| TransformError::DecodeFailed)?;
    let input_format = supported_format(reader.format())?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| TransformError::DecodeFailed)?;
    let orientation = decoder
        .orientation()
        .map_err(|_| TransformError::DecodeFailed)?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| TransformError::DecodeFailed)?;
    image.apply_orientation(orientation);

    let original = Dimensions {
        width: image.width(),
        height: image.height(),
    };
    let target = calculate_target(original, settings).map_err(map_resize_error)?;
    if target != original {
        image = image.resize_exact(target.width, target.height, FilterType::Lanczos3);
    }

    let output_format = match settings.output_format {
        OutputFormat::Original => input_format,
        selected => selected,
    };
    let quality = if matches!(
        settings.output_format,
        OutputFormat::Jpeg | OutputFormat::Webp
    ) {
        u8::try_from(settings.quality).map_err(|_| TransformError::EncodeFailed)?
    } else {
        82
    };
    let bytes = encode_image(&image, output_format, quality)?;
    let metadata = TransformationMetadata {
        input_format,
        output_format,
        original_width: original.width,
        original_height: original.height,
        output_width: target.width,
        output_height: target.height,
        encoded_size_bytes: bytes.len() as u64,
        metadata_disposition: if settings.remove_metadata {
            MetadataDisposition::Removed
        } else {
            MetadataDisposition::DiscardedUnsupported
        },
    };

    Ok(EncodedTransformation { bytes, metadata })
}

fn supported_format(format: Option<ImageFormat>) -> Result<OutputFormat, TransformError> {
    match format {
        Some(ImageFormat::Jpeg) => Ok(OutputFormat::Jpeg),
        Some(ImageFormat::Png) => Ok(OutputFormat::Png),
        Some(ImageFormat::WebP) => Ok(OutputFormat::Webp),
        _ => Err(TransformError::UnsupportedInputFormat),
    }
}

fn map_resize_error(error: ResizeError) -> TransformError {
    match error {
        ResizeError::InvalidSourceDimensions => TransformError::InvalidSourceDimensions,
        ResizeError::DimensionLimitExceeded => TransformError::DimensionLimitExceeded,
        ResizeError::ArithmeticOverflow => TransformError::ArithmeticOverflow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::jpeg::JpegEncoder, ImageDecoder, ImageEncoder, Rgba, RgbaImage};
    use settings::{ResizeMode, ResizeSettings};
    use std::{
        fs,
        io::Cursor,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    struct TestFile(PathBuf);
    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn settings(output_format: OutputFormat) -> BatchSettings {
        BatchSettings {
            output_format,
            quality: 82,
            resize: ResizeSettings {
                mode: ResizeMode::None,
                width: 1920,
                height: 1080,
                max_width: 1920,
                max_height: 1080,
                percentage: 50,
            },
            allow_upscaling: false,
            remove_metadata: false,
        }
    }

    fn test_path(extension: &str) -> TestFile {
        let number = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
        TestFile(std::env::temp_dir().join(format!(
            "pixchisel-transform-{}-{number}.{extension}",
            std::process::id()
        )))
    }

    fn write_source(format: OutputFormat, image: &RgbaImage) -> TestFile {
        let extension = match format {
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
            OutputFormat::Webp => "webp",
            OutputFormat::Original => unreachable!(),
        };
        let path = test_path(extension);
        let bytes = match format {
            OutputFormat::Png => {
                let mut bytes = Cursor::new(Vec::new());
                image::codecs::png::PngEncoder::new(&mut bytes)
                    .write_image(
                        image.as_raw(),
                        image.width(),
                        image.height(),
                        image::ExtendedColorType::Rgba8,
                    )
                    .unwrap();
                bytes.into_inner()
            }
            OutputFormat::Jpeg => encode_image(
                &DynamicImage::ImageRgba8(image.clone()),
                OutputFormat::Jpeg,
                90,
            )
            .unwrap(),
            OutputFormat::Webp => encode_image(
                &DynamicImage::ImageRgba8(image.clone()),
                OutputFormat::Webp,
                90,
            )
            .unwrap(),
            OutputFormat::Original => unreachable!(),
        };
        fs::write(&path.0, bytes).unwrap();
        path
    }

    fn write_oriented_jpeg(image: &RgbaImage) -> TestFile {
        // Little-endian TIFF with one Orientation=6 (rotate 90° clockwise) entry.
        let exif = vec![
            b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0,
            0,
        ];
        let path = test_path("jpg");
        let rgb = DynamicImage::ImageRgba8(image.clone()).to_rgb8();
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
        encoder.set_exif_metadata(exif).unwrap();
        encoder
            .write_image(
                rgb.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        fs::write(&path.0, bytes).unwrap();
        path
    }

    #[test]
    fn jpeg_to_jpeg_encodes_valid_requested_dimensions() {
        let source = write_source(
            OutputFormat::Jpeg,
            &RgbaImage::from_pixel(8, 4, Rgba([20, 80, 160, 255])),
        );
        let mut options = settings(OutputFormat::Jpeg);
        options.quality = 55;
        options.resize.mode = ResizeMode::Width;
        options.resize.width = 4;
        let result = transform_image(&source.0, &options).unwrap();
        let decoded =
            image::load_from_memory_with_format(&result.bytes, ImageFormat::Jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (4, 2));
        assert_eq!(
            (result.metadata.output_width, result.metadata.output_height),
            (4, 2)
        );
    }

    #[test]
    fn png_output_preserves_alpha_and_decodes() {
        let source = write_source(
            OutputFormat::Png,
            &RgbaImage::from_pixel(3, 2, Rgba([30, 40, 50, 40])),
        );
        let result = transform_image(&source.0, &settings(OutputFormat::Png)).unwrap();
        let decoded = image::load_from_memory_with_format(&result.bytes, ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (3, 2));
        assert_eq!(decoded.get_pixel(0, 0)[3], 40);
    }

    #[test]
    fn transparent_png_to_jpeg_uses_white_background() {
        let source = write_source(
            OutputFormat::Png,
            &RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0])),
        );
        let result = transform_image(&source.0, &settings(OutputFormat::Jpeg)).unwrap();
        let decoded = image::load_from_memory_with_format(&result.bytes, ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        assert!(decoded
            .get_pixel(0, 0)
            .0
            .iter()
            .all(|channel| *channel >= 250));
    }

    #[test]
    fn webp_input_decodes_and_lossy_webp_output_uses_quality() {
        let source = write_source(
            OutputFormat::Webp,
            &RgbaImage::from_pixel(4, 3, Rgba([90, 120, 180, 200])),
        );
        let result = transform_image(&source.0, &settings(OutputFormat::Webp)).unwrap();
        let decoded =
            image::load_from_memory_with_format(&result.bytes, ImageFormat::WebP).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (4, 3));
        assert_eq!(result.metadata.output_format, OutputFormat::Webp);
    }

    #[test]
    fn original_reencodes_to_the_detected_input_format() {
        let source = write_source(
            OutputFormat::Png,
            &RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 4])),
        );
        let result = transform_image(&source.0, &settings(OutputFormat::Original)).unwrap();
        assert_eq!(result.metadata.output_format, OutputFormat::Png);
        assert_eq!(
            image::guess_format(&result.bytes).unwrap(),
            ImageFormat::Png
        );
        assert_eq!(
            result.metadata.metadata_disposition,
            MetadataDisposition::DiscardedUnsupported
        );
    }

    #[test]
    fn remove_metadata_reports_intentional_removal() {
        let source = write_source(
            OutputFormat::Png,
            &RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255])),
        );
        let mut options = settings(OutputFormat::Png);
        options.remove_metadata = true;
        assert_eq!(
            transform_image(&source.0, &options)
                .unwrap()
                .metadata
                .metadata_disposition,
            MetadataDisposition::Removed
        );
    }

    #[test]
    fn orientation_is_applied_before_resize_and_metadata_is_not_copied() {
        let source = write_oriented_jpeg(&RgbaImage::from_pixel(2, 3, Rgba([40, 80, 120, 255])));
        let result = transform_image(&source.0, &settings(OutputFormat::Jpeg)).unwrap();
        assert_eq!(
            (
                result.metadata.original_width,
                result.metadata.original_height
            ),
            (3, 2)
        );

        let mut decoder = image::ImageReader::new(Cursor::new(&result.bytes))
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap();
        assert!(decoder.exif_metadata().unwrap().is_none());
        assert_eq!(
            result.metadata.metadata_disposition,
            MetadataDisposition::DiscardedUnsupported
        );
    }
}
