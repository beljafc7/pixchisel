use std::{
    fs, io,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, atomic::Ordering, Mutex},
    thread,
};

use image::{error::ImageError, ImageFormat, ImageReader};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SupportedImageFormat {
    Jpeg,
    Png,
    Webp,
}

impl TryFrom<ImageFormat> for SupportedImageFormat {
    type Error = ();

    fn try_from(format: ImageFormat) -> Result<Self, Self::Error> {
        match format {
            ImageFormat::Jpeg => Ok(Self::Jpeg),
            ImageFormat::Png => Ok(Self::Png),
            ImageFormat::WebP => Ok(Self::Webp),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspection {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub format: SupportedImageFormat,
    pub width: u32,
    pub height: u32,
    pub file_size_bytes: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ImageInspectionResult {
    Ready {
        image: ImageInspection,
    },
    Error {
        path: String,
        filename: String,
        extension: String,
        error: InspectImageError,
    },
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InspectImageErrorCode {
    FileNotFound,
    PermissionDenied,
    ReadFailed,
    UnsupportedFormat,
    InvalidImage,
    InvalidPath,
    Internal,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectImageError {
    pub code: InspectImageErrorCode,
    pub message: String,
}

impl InspectImageError {
    fn new(code: InspectImageErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn internal() -> Self {
        Self::new(
            InspectImageErrorCode::Internal,
            "The image inspection task could not be completed.",
        )
    }
}

pub fn inspect_image_file(path: &Path) -> Result<ImageInspection, InspectImageError> {
    let file = fs::File::open(path).map_err(map_io_error)?;
    let metadata = file.metadata().map_err(map_io_error)?;

    if !metadata.is_file() {
        return Err(InspectImageError::new(
            InspectImageErrorCode::ReadFailed,
            "The selected path is not a file.",
        ));
    }

    let reader = ImageReader::new(BufReader::new(file));
    let reader = reader.with_guessed_format().map_err(map_io_error)?;
    let format = reader
        .format()
        .ok_or_else(unsupported_format)
        .and_then(|format| {
            SupportedImageFormat::try_from(format).map_err(|_| unsupported_format())
        })?;
    let (width, height) = reader.into_dimensions().map_err(map_image_error)?;

    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            InspectImageError::new(
                InspectImageErrorCode::InvalidPath,
                "The selected file name cannot be represented safely.",
            )
        })?;
    let path_string = path.to_str().ok_or_else(|| {
        InspectImageError::new(
            InspectImageErrorCode::InvalidPath,
            "The selected path cannot be represented safely.",
        )
    })?;

    Ok(ImageInspection {
        path: path_string.to_owned(),
        filename: filename.to_owned(),
        extension: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase(),
        format,
        width,
        height,
        file_size_bytes: metadata.len(),
    })
}

pub fn inspect_image_files(paths: Vec<PathBuf>) -> Vec<ImageInspectionResult> {
    if paths.is_empty() {
        return Vec::new();
    }

    let worker_count = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
        .min(4)
        .min(paths.len());
    let next_index = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(paths.len())
            .collect::<Vec<Option<ImageInspectionResult>>>(),
    );

    thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(index) else {
                    break;
                };
                let result = inspection_result(path);
                results.lock().expect("inspection results lock")[index] = Some(result);
            });
        }
    });

    results
        .into_inner()
        .expect("inspection results lock")
        .into_iter()
        .map(|result| result.expect("every inspection produces a result"))
        .collect()
}

fn inspection_result(path: &Path) -> ImageInspectionResult {
    match inspect_image_file(path) {
        Ok(image) => ImageInspectionResult::Ready { image },
        Err(error) => ImageInspectionResult::Error {
            path: path.to_string_lossy().into_owned(),
            filename: path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Unknown file".to_owned()),
            extension: path
                .extension()
                .map(|value| value.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
            error,
        },
    }
}

fn unsupported_format() -> InspectImageError {
    InspectImageError::new(
        InspectImageErrorCode::UnsupportedFormat,
        "Select a JPEG, PNG, or WebP image.",
    )
}

fn map_io_error(error: io::Error) -> InspectImageError {
    match error.kind() {
        io::ErrorKind::NotFound => InspectImageError::new(
            InspectImageErrorCode::FileNotFound,
            "The selected file could not be found.",
        ),
        io::ErrorKind::PermissionDenied => InspectImageError::new(
            InspectImageErrorCode::PermissionDenied,
            "PixChisel does not have permission to read the selected file.",
        ),
        _ => InspectImageError::new(
            InspectImageErrorCode::ReadFailed,
            "The selected file could not be read.",
        ),
    }
}

fn map_image_error(error: ImageError) -> InspectImageError {
    match error {
        ImageError::Unsupported(_) => unsupported_format(),
        ImageError::IoError(error) => map_io_error(error),
        _ => InspectImageError::new(
            InspectImageErrorCode::InvalidImage,
            "The selected file is corrupt or is not a valid image.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use image::{DynamicImage, ImageFormat};

    use super::{
        inspect_image_file, inspect_image_files, ImageInspectionResult, InspectImageErrorCode,
        SupportedImageFormat,
    };

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pixchisel-inspection-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn inspects_supported_formats_from_file_content() {
        let directory = TestDirectory::new();
        let cases = [
            (
                "sample-jpeg.dat",
                ImageFormat::Jpeg,
                SupportedImageFormat::Jpeg,
            ),
            (
                "sample-png.dat",
                ImageFormat::Png,
                SupportedImageFormat::Png,
            ),
            (
                "sample-webp.dat",
                ImageFormat::WebP,
                SupportedImageFormat::Webp,
            ),
        ];

        for (filename, encoded_format, expected_format) in cases {
            let path = directory.path().join(filename);
            DynamicImage::new_rgb8(3, 2)
                .save_with_format(&path, encoded_format)
                .expect("write tiny image fixture");

            let result = inspect_image_file(&path).expect("inspect supported image");

            assert_eq!(result.format, expected_format);
            assert_eq!((result.width, result.height), (3, 2));
            assert_eq!(result.extension, "dat");
            assert!(result.file_size_bytes > 0);
        }
    }

    #[test]
    fn rejects_unsupported_files() {
        let directory = TestDirectory::new();
        let path = directory.path().join("notes.txt");
        fs::write(&path, b"not an image").expect("write unsupported fixture");

        let error = inspect_image_file(&path).expect_err("reject unsupported fixture");

        assert_eq!(error.code, InspectImageErrorCode::UnsupportedFormat);
    }

    #[test]
    fn rejects_corrupt_supported_images() {
        let directory = TestDirectory::new();
        let path = directory.path().join("broken.jpg");
        fs::write(&path, [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]).expect("write corrupt JPEG fixture");

        let error = inspect_image_file(&path).expect_err("reject corrupt fixture");

        assert_eq!(error.code, InspectImageErrorCode::InvalidImage);
    }

    #[test]
    fn reports_missing_files() {
        let directory = TestDirectory::new();
        let path = directory.path().join("missing.png");

        let error = inspect_image_file(&path).expect_err("report missing fixture");

        assert_eq!(error.code, InspectImageErrorCode::FileNotFound);
    }

    #[test]
    fn multi_file_inspection_preserves_input_order() {
        let directory = TestDirectory::new();
        let paths = ["first.png", "second.jpg", "third.webp"].map(|filename| {
            let path = directory.path().join(filename);
            DynamicImage::new_rgb8(3, 2)
                .save(&path)
                .expect("write ordered image fixture");
            path
        });

        let results = inspect_image_files(paths.to_vec());
        let filenames = results
            .into_iter()
            .map(|result| match result {
                ImageInspectionResult::Ready { image } => image.filename,
                ImageInspectionResult::Error { .. } => panic!("expected ready result"),
            })
            .collect::<Vec<_>>();

        assert_eq!(filenames, ["first.png", "second.jpg", "third.webp"]);
    }

    #[test]
    fn multi_file_inspection_keeps_successes_and_errors() {
        let directory = TestDirectory::new();
        let valid_path = directory.path().join("valid.png");
        let invalid_path = directory.path().join("invalid.png");
        DynamicImage::new_rgb8(1, 1)
            .save(&valid_path)
            .expect("write valid fixture");
        fs::write(&invalid_path, b"not an image").expect("write invalid fixture");

        let results = inspect_image_files(vec![valid_path, invalid_path]);

        assert!(matches!(results[0], ImageInspectionResult::Ready { .. }));
        assert!(matches!(
            results[1],
            ImageInspectionResult::Error {
                error: super::InspectImageError {
                    code: InspectImageErrorCode::UnsupportedFormat,
                    ..
                },
                ..
            }
        ));
    }
}
