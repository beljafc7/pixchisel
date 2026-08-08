use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

use crate::imaging::transform::{
    detect_input_format, transform_image, BatchSettings, OutputFormat, TransformError,
    TransformationMetadata,
};

const MAX_COPY_ATTEMPTS: u32 = 10_000;
const MAX_TEMP_ATTEMPTS: u32 = 10_000;
static NEXT_TEMP_NAME: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConflictPolicy {
    Overwrite,
    CreateCopy,
    Skip,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WriteImageRequest {
    pub source_path: PathBuf,
    pub output_directory: PathBuf,
    pub settings: BatchSettings,
    pub conflict_policy: ConflictPolicy,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum WriteImageResult {
    Written {
        #[serde(rename = "sourcePath")]
        source_path: String,
        #[serde(rename = "outputPath")]
        output_path: String,
        #[serde(flatten)]
        transformation: TransformationMetadata,
        #[serde(rename = "originalSizeBytes")]
        original_size_bytes: u64,
        #[serde(rename = "outputSizeBytes")]
        output_size_bytes: u64,
    },
    Skipped {
        #[serde(rename = "sourcePath")]
        source_path: String,
        #[serde(rename = "outputPath")]
        output_path: String,
        #[serde(rename = "outputFormat")]
        output_format: OutputFormat,
        #[serde(rename = "originalSizeBytes")]
        original_size_bytes: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteImageErrorCode {
    OutputDirectoryMissing,
    OutputDirectoryNotWritable,
    DestinationConflict,
    TempFileCreationFailed,
    WriteFailed,
    FinalizeFailed,
    CleanupFailed,
    UnsafeSourceDestination,
    TransformFailed,
    OpenOutputFolderFailed,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteImageError {
    pub code: WriteImageErrorCode,
    pub message: String,
}

impl WriteImageError {
    pub(crate) fn new(code: WriteImageErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn internal() -> Self {
        Self::new(
            WriteImageErrorCode::WriteFailed,
            "The output operation could not be completed.",
        )
    }
}

pub fn write_transformed_image(
    request: WriteImageRequest,
) -> Result<WriteImageResult, WriteImageError> {
    validate_output_directory(&request.output_directory)?;
    let output_format = match request.settings.output_format {
        OutputFormat::Original => {
            detect_input_format(&request.source_path).map_err(map_transform_error)?
        }
        selected => selected,
    };
    let base_destination = output_path(
        &request.source_path,
        &request.output_directory,
        output_format,
    )?;
    let original_size_bytes = fs::metadata(&request.source_path)
        .map_err(|_| transform_failed())?
        .len();

    if base_destination.exists() && request.conflict_policy == ConflictPolicy::Skip {
        return Ok(WriteImageResult::Skipped {
            source_path: display_path(&request.source_path),
            output_path: display_path(&base_destination),
            output_format,
            original_size_bytes,
        });
    }

    let destination = match request.conflict_policy {
        ConflictPolicy::CreateCopy => available_copy_path(&base_destination)?,
        ConflictPolicy::Overwrite | ConflictPolicy::Skip => base_destination,
    };
    let transformed =
        transform_image(&request.source_path, &request.settings).map_err(map_transform_error)?;
    let mut temporary = TemporaryOutput::create(&request.output_directory)?;
    temporary.write_all(&transformed.bytes)?;
    temporary.finalize(&destination, request.conflict_policy)?;

    let output_size_bytes = fs::metadata(&destination)
        .map_err(|_| {
            WriteImageError::new(
                WriteImageErrorCode::FinalizeFailed,
                "The completed output could not be verified.",
            )
        })?
        .len();

    Ok(WriteImageResult::Written {
        source_path: display_path(&request.source_path),
        output_path: display_path(&destination),
        transformation: transformed.metadata,
        original_size_bytes,
        output_size_bytes,
    })
}

pub fn output_path(
    source: &Path,
    output_directory: &Path,
    format: OutputFormat,
) -> Result<PathBuf, WriteImageError> {
    let stem = source
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| {
            WriteImageError::new(
                WriteImageErrorCode::UnsafeSourceDestination,
                "The source filename cannot be used for output.",
            )
        })?;
    let mut filename = OsString::from(stem);
    filename.push(".");
    filename.push(extension(format));
    Ok(output_directory.join(filename))
}

fn extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Original => unreachable!("original is resolved before naming"),
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
    }
}

fn available_copy_path(destination: &Path) -> Result<PathBuf, WriteImageError> {
    if !destination.exists() {
        return Ok(destination.to_path_buf());
    }

    let stem = destination.file_stem().ok_or_else(|| {
        WriteImageError::new(
            WriteImageErrorCode::UnsafeSourceDestination,
            "The destination filename cannot be made unique.",
        )
    })?;
    let extension = destination.extension();
    for number in 1..=MAX_COPY_ATTEMPTS {
        let mut filename = OsString::from(stem);
        filename.push(format!(" ({number})"));
        if let Some(extension) = extension {
            filename.push(".");
            filename.push(extension);
        }
        let candidate = destination.with_file_name(filename);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(WriteImageError::new(
        WriteImageErrorCode::DestinationConflict,
        "No available copy filename could be found.",
    ))
}

pub(crate) fn validate_output_directory(directory: &Path) -> Result<(), WriteImageError> {
    match fs::metadata(directory) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) | Err(_) => Err(WriteImageError::new(
            WriteImageErrorCode::OutputDirectoryMissing,
            "The selected output folder is no longer available.",
        )),
    }
}

struct TemporaryOutput {
    path: PathBuf,
    file: Option<File>,
}

impl TemporaryOutput {
    fn create(directory: &Path) -> Result<Self, WriteImageError> {
        for _ in 0..MAX_TEMP_ATTEMPTS {
            let number = NEXT_TEMP_NAME.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!(".pixchisel-{}-{number}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                    })
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    return Err(WriteImageError::new(
                        WriteImageErrorCode::OutputDirectoryNotWritable,
                        "PixChisel cannot write to the selected output folder.",
                    ))
                }
                Err(error) => {
                    return Err(WriteImageError::new(
                        WriteImageErrorCode::TempFileCreationFailed,
                        if error.kind() == io::ErrorKind::StorageFull {
                            "The output drive does not have enough free space."
                        } else {
                            "A temporary output file could not be created. Check that the output folder is still available."
                        },
                    ))
                }
            }
        }
        Err(WriteImageError::new(
            WriteImageErrorCode::TempFileCreationFailed,
            "A unique temporary output file could not be created.",
        ))
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), WriteImageError> {
        let file = self.file.as_mut().expect("temporary file remains open");
        file.write_all(bytes)
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                WriteImageError::new(
                    WriteImageErrorCode::WriteFailed,
                    if error.kind() == io::ErrorKind::StorageFull {
                        "The output drive ran out of space before the image was written."
                    } else if error.kind() == io::ErrorKind::PermissionDenied {
                        "PixChisel lost permission to write to the output folder."
                    } else {
                        "The image could not be written completely. Check the output folder and try again."
                    },
                )
            })
    }

    fn finalize(
        &mut self,
        destination: &Path,
        policy: ConflictPolicy,
    ) -> Result<(), WriteImageError> {
        self.file.take();
        let result = match policy {
            ConflictPolicy::CreateCopy | ConflictPolicy::Skip => {
                finalize_without_overwrite(&self.path, destination)
            }
            ConflictPolicy::Overwrite => finalize_overwrite(&self.path, destination),
        };
        if result.is_ok() {
            self.path.clear();
        }
        result
    }
}

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        self.file.take();
        if !self.path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn finalize_without_overwrite(temp: &Path, destination: &Path) -> Result<(), WriteImageError> {
    match fs::hard_link(temp, destination) {
        Ok(()) => fs::remove_file(temp).map_err(|_| cleanup_failed()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(WriteImageError::new(
            WriteImageErrorCode::DestinationConflict,
            "An output with that name appeared before the write completed.",
        )),
        Err(_) => copy_without_overwrite(temp, destination),
    }
}

fn copy_without_overwrite(temp: &Path, destination: &Path) -> Result<(), WriteImageError> {
    let mut source = File::open(temp).map_err(|_| finalize_failed())?;
    let mut output = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
    {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(WriteImageError::new(
                WriteImageErrorCode::DestinationConflict,
                "An output with that name appeared before the write completed.",
            ))
        }
        Err(_) => return Err(finalize_failed()),
    };

    if io::copy(&mut source, &mut output)
        .and_then(|_| output.flush())
        .and_then(|_| output.sync_all())
        .is_err()
    {
        drop(output);
        let _ = fs::remove_file(destination);
        return Err(finalize_failed());
    }
    drop(output);
    fs::remove_file(temp).map_err(|_| cleanup_failed())
}

#[cfg(unix)]
fn finalize_overwrite(temp: &Path, destination: &Path) -> Result<(), WriteImageError> {
    fs::rename(temp, destination).map_err(|_| finalize_failed())
}

#[cfg(windows)]
fn finalize_overwrite(temp: &Path, destination: &Path) -> Result<(), WriteImageError> {
    if !destination.exists() {
        return finalize_without_overwrite(temp, destination);
    }

    let backup = unique_backup_path(destination)?;
    fs::rename(destination, &backup).map_err(|_| finalize_failed())?;
    if fs::rename(temp, destination).is_err() {
        let restored = fs::rename(&backup, destination).is_ok();
        return Err(if restored {
            finalize_failed()
        } else {
            cleanup_failed()
        });
    }
    fs::remove_file(backup).map_err(|_| cleanup_failed())
}

#[cfg(windows)]
fn unique_backup_path(destination: &Path) -> Result<PathBuf, WriteImageError> {
    for number in 0..MAX_TEMP_ATTEMPTS {
        let candidate = destination.with_file_name(format!(
            ".pixchisel-backup-{}-{number}.tmp",
            std::process::id()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(WriteImageError::new(
        WriteImageErrorCode::TempFileCreationFailed,
        "A safe replacement backup could not be prepared.",
    ))
}

fn map_transform_error(_error: TransformError) -> WriteImageError {
    transform_failed()
}

fn transform_failed() -> WriteImageError {
    WriteImageError::new(
        WriteImageErrorCode::TransformFailed,
        "The source image could not be transformed.",
    )
}

fn finalize_failed() -> WriteImageError {
    WriteImageError::new(
        WriteImageErrorCode::FinalizeFailed,
        "The completed output could not be moved into place safely.",
    )
}

fn cleanup_failed() -> WriteImageError {
    WriteImageError::new(
        WriteImageErrorCode::CleanupFailed,
        "The output completed, but a temporary file could not be cleaned up.",
    )
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::transform::{ResizeMode, ResizeSettings};
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let number = NEXT_TEMP_NAME.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pixchisel-output-tests-{}-{number}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn settings(format: OutputFormat) -> BatchSettings {
        BatchSettings {
            output_format: format,
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
            remove_metadata: true,
        }
    }

    fn write_source(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 4, Rgba([40, 90, 160, 180])))
            .save(&path)
            .unwrap();
        path
    }

    fn request(
        source: PathBuf,
        output_directory: PathBuf,
        format: OutputFormat,
        conflict_policy: ConflictPolicy,
    ) -> WriteImageRequest {
        WriteImageRequest {
            source_path: source,
            output_directory,
            settings: settings(format),
            conflict_policy,
        }
    }

    fn written_path(result: &WriteImageResult) -> PathBuf {
        match result {
            WriteImageResult::Written { output_path, .. } => PathBuf::from(output_path),
            WriteImageResult::Skipped { .. } => panic!("expected written output"),
        }
    }

    #[test]
    fn naming_normalizes_extensions_and_preserves_unicode_stems() {
        let output = Path::new("output");
        assert_eq!(
            output_path(Path::new("photo.PNG"), output, OutputFormat::Jpeg).unwrap(),
            output.join("photo.jpg")
        );
        assert_eq!(
            output_path(Path::new("photo.jpg"), output, OutputFormat::Png).unwrap(),
            output.join("photo.png")
        );
        assert_eq!(
            output_path(Path::new("photo.jpg"), output, OutputFormat::Webp).unwrap(),
            output.join("photo.webp")
        );
        assert_eq!(
            output_path(Path::new("café.png"), output, OutputFormat::Png).unwrap(),
            output.join("café.png")
        );
    }

    #[test]
    fn request_contract_and_conflict_policy_use_typescript_spelling() {
        let value = serde_json::json!({
            "sourcePath": "/images/photo.png",
            "outputDirectory": "/images/output",
            "settings": {
                "outputFormat": "png",
                "quality": 82,
                "resize": {
                    "mode": "none",
                    "width": 1920,
                    "height": 1080,
                    "maxWidth": 1920,
                    "maxHeight": 1080,
                    "percentage": 50
                },
                "allowUpscaling": false,
                "removeMetadata": true
            },
            "conflictPolicy": "createCopy"
        });
        let parsed: WriteImageRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(parsed.conflict_policy, ConflictPolicy::CreateCopy);
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        assert!(serde_json::from_value::<ConflictPolicy>(serde_json::json!("rename")).is_err());
    }

    #[test]
    fn original_jpeg_output_normalizes_extension_to_jpg() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "photo.jpeg");
        let result = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();
        assert_eq!(written_path(&result), directory.0.join("photo.jpg"));
    }

    #[test]
    fn create_copy_uses_first_available_number_without_overwriting() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "photo.png");
        let first = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();
        fs::write(directory.0.join("photo (1).jpg"), b"keep-one").unwrap();
        let second = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();

        assert_eq!(written_path(&first), directory.0.join("photo.jpg"));
        assert_eq!(written_path(&second), directory.0.join("photo (2).jpg"));
        assert_eq!(
            fs::read(directory.0.join("photo (1).jpg")).unwrap(),
            b"keep-one"
        );
    }

    #[test]
    fn skip_returns_before_transform_and_preserves_destination() {
        let directory = TestDirectory::new();
        let source = directory.0.join("broken.png");
        fs::write(&source, b"not an image").unwrap();
        let destination = directory.0.join("broken.jpg");
        fs::write(&destination, b"existing").unwrap();

        let result = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Skip,
        ));

        assert!(matches!(result.unwrap(), WriteImageResult::Skipped { .. }));
        assert_eq!(fs::read(destination).unwrap(), b"existing");
    }

    #[test]
    fn skip_existing_valid_output_is_not_an_error() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "photo.png");
        let destination = directory.0.join("photo.jpg");
        fs::write(&destination, b"existing").unwrap();
        let result = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Skip,
        ))
        .unwrap();
        assert!(matches!(result, WriteImageResult::Skipped { .. }));
        assert_eq!(fs::read(destination).unwrap(), b"existing");
    }

    #[test]
    fn overwrite_replaces_only_with_complete_decodable_output() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "source.png");
        let destination = directory.0.join("source.jpg");
        fs::write(&destination, b"old destination").unwrap();
        let result = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Overwrite,
        ))
        .unwrap();
        let path = written_path(&result);
        assert_eq!(image::open(&path).unwrap().dimensions(), (8, 4));
        match result {
            WriteImageResult::Written {
                output_size_bytes,
                transformation,
                ..
            } => {
                assert_eq!(output_size_bytes, fs::metadata(path).unwrap().len());
                assert_eq!(
                    (transformation.output_width, transformation.output_height),
                    (8, 4)
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn source_equals_destination_is_safe_for_every_policy() {
        let directory = TestDirectory::new();

        let overwrite_source = write_source(&directory.0, "overwrite.png");
        let overwrite = write_transformed_image(request(
            overwrite_source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        ))
        .unwrap();
        assert_eq!(written_path(&overwrite), overwrite_source);
        assert_eq!(image::open(&overwrite_source).unwrap().dimensions(), (8, 4));

        let copy_source = write_source(&directory.0, "copy.png");
        let copy = write_transformed_image(request(
            copy_source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();
        assert_eq!(written_path(&copy), directory.0.join("copy (1).png"));
        assert!(copy_source.exists());

        let skip_source = write_source(&directory.0, "skip.png");
        let before = fs::read(&skip_source).unwrap();
        let skipped = write_transformed_image(request(
            skip_source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Skip,
        ))
        .unwrap();
        assert!(matches!(skipped, WriteImageResult::Skipped { .. }));
        assert_eq!(fs::read(skip_source).unwrap(), before);
    }

    #[test]
    fn failed_transform_preserves_destination_and_leaves_no_temp_file() {
        let directory = TestDirectory::new();
        let source = directory.0.join("broken.png");
        fs::write(&source, b"not an image").unwrap();
        let destination = directory.0.join("broken.jpg");
        fs::write(&destination, b"existing").unwrap();
        let result = write_transformed_image(request(
            source,
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Overwrite,
        ));
        assert_eq!(
            result.unwrap_err().code,
            WriteImageErrorCode::TransformFailed
        );
        assert_eq!(fs::read(destination).unwrap(), b"existing");
        assert!(fs::read_dir(&directory.0).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".pixchisel-")));
    }

    #[test]
    fn portable_copy_finalization_never_overwrites_and_removes_its_temp_file() {
        let directory = TestDirectory::new();
        let temporary = directory.0.join("complete.tmp");
        let destination = directory.0.join("photo.png");
        fs::write(&temporary, b"complete output").unwrap();

        copy_without_overwrite(&temporary, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"complete output");
        assert!(!temporary.exists());

        let second_temporary = directory.0.join("second.tmp");
        fs::write(&second_temporary, b"must not replace").unwrap();
        let error = copy_without_overwrite(&second_temporary, &destination).unwrap_err();
        assert_eq!(error.code, WriteImageErrorCode::DestinationConflict);
        assert_eq!(fs::read(&destination).unwrap(), b"complete output");
    }

    #[test]
    fn writes_each_output_format_and_reports_actual_size() {
        for (format, extension) in [
            (OutputFormat::Jpeg, "jpg"),
            (OutputFormat::Png, "png"),
            (OutputFormat::Webp, "webp"),
        ] {
            let directory = TestDirectory::new();
            let source = write_source(&directory.0, "photo.png");
            let mut request = request(
                source,
                directory.0.clone(),
                format,
                ConflictPolicy::CreateCopy,
            );
            request.settings.resize.mode = ResizeMode::Width;
            request.settings.resize.width = 4;
            let result = write_transformed_image(request).unwrap();
            let path = written_path(&result);
            assert_eq!(path.extension().unwrap(), extension);
            assert_eq!(image::open(&path).unwrap().dimensions(), (4, 2));
            match result {
                WriteImageResult::Written {
                    output_size_bytes, ..
                } => assert_eq!(output_size_bytes, fs::metadata(path).unwrap().len()),
                _ => unreachable!(),
            }
        }
    }
}
