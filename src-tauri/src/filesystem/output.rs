use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

use crate::imaging::transform::{
    detect_input_format, transform_image_with_progress_and_cancellation, BatchSettings,
    OutputFormat, ProcessingStage, TransformError, TransformationMetadata,
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
    pub settings: BatchSettings,
    pub destination: WriteDestination,
    pub operation: WriteOperation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteOperation {
    Compress,
    Convert,
    Resize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum WriteDestination {
    Directory { path: PathBuf },
    AutomaticDirectory { path: PathBuf },
    ReplaceOriginal,
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
    NotSmaller {
        #[serde(rename = "sourcePath")]
        source_path: String,
        #[serde(rename = "outputFormat")]
        output_format: OutputFormat,
        #[serde(rename = "originalSizeBytes")]
        original_size_bytes: u64,
        #[serde(rename = "candidateSizeBytes")]
        candidate_size_bytes: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteImageErrorCode {
    Cancelled,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingProgress {
    pub path: String,
    pub stage: ProcessingStage,
    pub percent: u8,
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
    write_transformed_image_with_progress(request, |_| {})
}

pub fn write_transformed_image_with_progress(
    request: WriteImageRequest,
    on_progress: impl FnMut(ProcessingProgress),
) -> Result<WriteImageResult, WriteImageError> {
    write_transformed_image_with_progress_and_cancellation(request, on_progress, || false)
}

pub fn write_transformed_image_with_progress_and_cancellation(
    request: WriteImageRequest,
    mut on_progress: impl FnMut(ProcessingProgress),
    should_cancel: impl Fn() -> bool,
) -> Result<WriteImageResult, WriteImageError> {
    let source_path = display_path(&request.source_path);
    let mut report = |stage: ProcessingStage| {
        on_progress(ProcessingProgress {
            path: source_path.clone(),
            stage,
            percent: stage.percent(),
        });
    };
    report(ProcessingStage::Preparing);
    ensure_not_cancelled(&should_cancel)?;
    let output_format = match request.settings.output_format {
        OutputFormat::Original => {
            detect_input_format(&request.source_path).map_err(map_transform_error)?
        }
        selected => selected,
    };
    let original_size_bytes = fs::metadata(&request.source_path)
        .map_err(|_| transform_failed())?
        .len();
    let (directory, mut destination, finalization, remove_source_after, create_directory) =
        match &request.destination {
            WriteDestination::Directory { path } => {
                validate_output_directory(path)?;
                let base_destination = output_path(&request.source_path, path, output_format)?;
                (
                    path.clone(),
                    Some(available_copy_path(&base_destination)?),
                    ConflictPolicy::CreateCopy,
                    false,
                    false,
                )
            }
            WriteDestination::AutomaticDirectory { path } => {
                (path.clone(), None, ConflictPolicy::CreateCopy, false, true)
            }
            WriteDestination::ReplaceOriginal => {
                let directory = request
                    .source_path
                    .parent()
                    .filter(|path| !path.as_os_str().is_empty())
                    .ok_or_else(|| {
                        WriteImageError::new(
                            WriteImageErrorCode::UnsafeSourceDestination,
                            "The source image does not have a safe parent folder.",
                        )
                    })?
                    .to_path_buf();
                validate_output_directory(&directory)?;
                let destination = if request.settings.output_format == OutputFormat::Original {
                    request.source_path.clone()
                } else {
                    output_path(&request.source_path, &directory, output_format)?
                };
                let changes_path = destination != request.source_path;
                if changes_path && destination.exists() {
                    return Err(WriteImageError::new(
                    WriteImageErrorCode::DestinationConflict,
                    "A different file already uses the converted filename. The original was not changed.",
                ));
                }
                (
                    directory,
                    Some(destination),
                    if changes_path {
                        ConflictPolicy::CreateCopy
                    } else {
                        ConflictPolicy::Overwrite
                    },
                    changes_path,
                    false,
                )
            }
        };
    ensure_not_cancelled(&should_cancel)?;
    let transformed = transform_image_with_progress_and_cancellation(
        &request.source_path,
        &request.settings,
        &mut report,
        &should_cancel,
    )
    .map_err(map_transform_error)?;
    ensure_not_cancelled(&should_cancel)?;
    if is_not_smaller(
        request.operation,
        transformed.bytes.len(),
        original_size_bytes,
    ) {
        return Ok(WriteImageResult::NotSmaller {
            source_path: display_path(&request.source_path),
            output_format,
            original_size_bytes,
            candidate_size_bytes: transformed.bytes.len() as u64,
        });
    }
    if create_directory {
        create_automatic_output_directory(&directory)?;
        let base_destination = output_path(&request.source_path, &directory, output_format)?;
        destination = Some(available_copy_path(&base_destination)?);
    }
    let destination = destination.expect("every writable destination is resolved before saving");
    report(ProcessingStage::Saving);
    let mut temporary = TemporaryOutput::create(&directory)?;
    temporary.write_all(&transformed.bytes)?;
    ensure_not_cancelled(&should_cancel)?;
    temporary.finalize(&destination, finalization)?;
    if remove_source_after {
        fs::remove_file(&request.source_path).map_err(|_| {
            WriteImageError::new(
                WriteImageErrorCode::CleanupFailed,
                "The converted image was saved, but the original could not be removed.",
            )
        })?;
    }

    let output_size_bytes = fs::metadata(&destination)
        .map_err(|_| {
            WriteImageError::new(
                WriteImageErrorCode::FinalizeFailed,
                "The completed output could not be verified.",
            )
        })?
        .len();

    let result = WriteImageResult::Written {
        source_path: display_path(&request.source_path),
        output_path: display_path(&destination),
        transformation: transformed.metadata,
        original_size_bytes,
        output_size_bytes,
    };
    report(ProcessingStage::Completed);
    Ok(result)
}

fn ensure_not_cancelled(should_cancel: impl Fn() -> bool) -> Result<(), WriteImageError> {
    if should_cancel() {
        Err(WriteImageError::new(
            WriteImageErrorCode::Cancelled,
            "Processing was cancelled before the output was saved.",
        ))
    } else {
        Ok(())
    }
}

fn is_not_smaller(operation: WriteOperation, candidate_size: usize, original_size: u64) -> bool {
    operation == WriteOperation::Compress && candidate_size as u64 >= original_size
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

fn create_automatic_output_directory(directory: &Path) -> Result<(), WriteImageError> {
    fs::create_dir_all(directory).map_err(|error| {
        let message = if error.kind() == io::ErrorKind::PermissionDenied {
            "PixChisel does not have permission to create the automatic output folder. Choose another folder and retry."
        } else {
            "PixChisel could not create the automatic output folder. Choose another folder and retry."
        };
        WriteImageError::new(WriteImageErrorCode::OutputDirectoryNotWritable, message)
    })?;
    validate_output_directory(directory)
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

fn map_transform_error(error: TransformError) -> WriteImageError {
    if error == TransformError::Cancelled {
        WriteImageError::new(
            WriteImageErrorCode::Cancelled,
            "Processing was cancelled before the output was saved.",
        )
    } else {
        transform_failed()
    }
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
            settings: settings(format),
            destination: match conflict_policy {
                ConflictPolicy::CreateCopy | ConflictPolicy::Skip => WriteDestination::Directory {
                    path: output_directory,
                },
                ConflictPolicy::Overwrite => WriteDestination::ReplaceOriginal,
            },
            operation: WriteOperation::Convert,
        }
    }

    fn written_path(result: &WriteImageResult) -> PathBuf {
        match result {
            WriteImageResult::Written { output_path, .. } => PathBuf::from(output_path),
            WriteImageResult::Skipped { .. } | WriteImageResult::NotSmaller { .. } => {
                panic!("expected written output")
            }
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
    fn request_contract_uses_a_discriminated_destination() {
        let value = serde_json::json!({
            "sourcePath": "/images/photo.png",
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
            "destination": { "mode": "directory", "path": "/images/output" },
            "operation": "compress"
        });
        let parsed: WriteImageRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            parsed.destination,
            WriteDestination::Directory {
                path: PathBuf::from("/images/output")
            }
        );
        assert_eq!(parsed.operation, WriteOperation::Compress);
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        let replace: WriteDestination =
            serde_json::from_value(serde_json::json!({ "mode": "replaceOriginal" })).unwrap();
        assert_eq!(replace, WriteDestination::ReplaceOriginal);
        let automatic: WriteDestination = serde_json::from_value(serde_json::json!({
            "mode": "automaticDirectory",
            "path": "/images/PixChisel Copies"
        }))
        .unwrap();
        assert_eq!(
            automatic,
            WriteDestination::AutomaticDirectory {
                path: PathBuf::from("/images/PixChisel Copies")
            }
        );
    }

    #[test]
    fn terminal_results_serialize_with_frontend_discriminators_and_fields() {
        let written = WriteImageResult::Written {
            source_path: "/input/photo.jpg".into(),
            output_path: "/output/photo.jpg".into(),
            transformation: TransformationMetadata {
                input_format: OutputFormat::Jpeg,
                output_format: OutputFormat::Jpeg,
                original_width: 100,
                original_height: 50,
                output_width: 100,
                output_height: 50,
                encoded_size_bytes: 400,
                metadata_disposition: crate::imaging::transform::MetadataDisposition::Removed,
            },
            original_size_bytes: 1000,
            output_size_bytes: 400,
        };
        let value = serde_json::to_value(written).unwrap();
        assert_eq!(value["status"], "written");
        assert_eq!(value["outputPath"], "/output/photo.jpg");
        assert_eq!(value["originalSizeBytes"], 1000);
        assert_eq!(value["outputSizeBytes"], 400);
        assert_eq!(value["outputWidth"], 100);

        let skipped = WriteImageResult::Skipped {
            source_path: "/input/photo.jpg".into(),
            output_path: "/output/photo.jpg".into(),
            output_format: OutputFormat::Jpeg,
            original_size_bytes: 1000,
        };
        let value = serde_json::to_value(skipped).unwrap();
        assert_eq!(value["status"], "skipped");
        assert_eq!(value["outputPath"], "/output/photo.jpg");
        assert!(value.get("outputSizeBytes").is_none());

        let not_smaller = WriteImageResult::NotSmaller {
            source_path: "/input/photo.png".into(),
            output_format: OutputFormat::Png,
            original_size_bytes: 100,
            candidate_size_bytes: 100,
        };
        let value = serde_json::to_value(not_smaller).unwrap();
        assert_eq!(value["status"], "notSmaller");
        assert_eq!(value["candidateSizeBytes"], 100);
        assert!(value.get("outputPath").is_none());
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
        let source_before = fs::read(&source).unwrap();
        let first = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();
        fs::write(directory.0.join("photo (1).jpg"), b"keep-one").unwrap();
        let second = write_transformed_image(request(
            source.clone(),
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
        assert_eq!(fs::read(source).unwrap(), source_before);
    }

    #[test]
    fn create_copies_flattens_mixed_sources_without_modifying_them() {
        let root = TestDirectory::new();
        let output = root.0.join("output");
        fs::create_dir(&output).unwrap();
        for folder in ["one", "two/nested"] {
            let directory = root.0.join(folder);
            fs::create_dir_all(&directory).unwrap();
            let source = write_source(&directory, &format!("{}.png", folder.replace('/', "-")));
            let before = fs::read(&source).unwrap();
            let result = write_transformed_image(request(
                source.clone(),
                output.clone(),
                OutputFormat::Jpeg,
                ConflictPolicy::CreateCopy,
            ))
            .unwrap();
            assert_eq!(written_path(&result).parent(), Some(output.as_path()));
            assert_eq!(fs::read(source).unwrap(), before);
        }
    }

    #[test]
    fn overwrite_replaces_only_with_complete_decodable_output() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "source.png");
        let result = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        ))
        .unwrap();
        let path = written_path(&result);
        assert_eq!(path, source);
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
    }

    #[test]
    fn failed_transform_preserves_destination_and_leaves_no_temp_file() {
        let directory = TestDirectory::new();
        let source = directory.0.join("broken.png");
        fs::write(&source, b"not an image").unwrap();
        let before = fs::read(&source).unwrap();
        let result = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        ));
        assert_eq!(
            result.unwrap_err().code,
            WriteImageErrorCode::TransformFailed
        );
        assert_eq!(fs::read(source).unwrap(), before);
        assert!(fs::read_dir(&directory.0).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".pixchisel-")));
    }

    #[cfg(unix)]
    #[test]
    fn replace_original_preserves_source_when_temporary_creation_fails() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "unwritable.png");
        let before = fs::read(&source).unwrap();
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o555)).unwrap();
        let result = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        ));
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            result.unwrap_err().code,
            WriteImageErrorCode::OutputDirectoryNotWritable
        );
        assert_eq!(fs::read(source).unwrap(), before);
    }

    #[test]
    fn compress_never_grows_for_any_format_but_convert_and_resize_may() {
        assert!(is_not_smaller(WriteOperation::Compress, 101, 100));
        assert!(is_not_smaller(WriteOperation::Compress, 100, 100));
        assert!(!is_not_smaller(WriteOperation::Compress, 99, 100));
        assert!(!is_not_smaller(WriteOperation::Convert, 101, 100));
        assert!(!is_not_smaller(WriteOperation::Resize, 101, 100));
    }

    #[test]
    fn active_cancellation_stops_before_saving_and_preserves_the_source() {
        let root = TestDirectory::new();
        let source = write_source(&root.0, "cancel.jpg");
        let before = fs::read(&source).unwrap();
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let progress_flag = cancelled.clone();
        let check_flag = cancelled.clone();
        let mut stages = Vec::new();

        let result = write_transformed_image_with_progress_and_cancellation(
            request(
                source.clone(),
                root.0.clone(),
                OutputFormat::Webp,
                ConflictPolicy::CreateCopy,
            ),
            |progress| {
                stages.push(progress.stage);
                if progress.stage == ProcessingStage::Optimizing {
                    progress_flag.store(true, std::sync::atomic::Ordering::Release);
                }
            },
            || check_flag.load(std::sync::atomic::Ordering::Acquire),
        );

        assert_eq!(result.unwrap_err().code, WriteImageErrorCode::Cancelled);
        assert_eq!(
            stages,
            vec![
                ProcessingStage::Preparing,
                ProcessingStage::Decoding,
                ProcessingStage::Optimizing,
            ]
        );
        assert_eq!(fs::read(&source).unwrap(), before);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }

    #[test]
    fn already_optimized_png_writes_no_copy_and_preserves_original() {
        let root = TestDirectory::new();
        let source = write_source(&root.0, "optimized.png");
        let first = write_transformed_image(request(
            source,
            root.0.clone(),
            OutputFormat::Png,
            ConflictPolicy::CreateCopy,
        ))
        .unwrap();
        let optimized = written_path(&first);
        let before = fs::read(&optimized).unwrap();
        let copy_directory = root.0.join("PixChisel Copies");
        let mut copy_request = request(
            optimized.clone(),
            copy_directory.clone(),
            OutputFormat::Original,
            ConflictPolicy::CreateCopy,
        );
        copy_request.destination = WriteDestination::AutomaticDirectory {
            path: copy_directory.clone(),
        };
        copy_request.operation = WriteOperation::Compress;
        let mut stages = Vec::new();
        let copy_result = write_transformed_image_with_progress(copy_request, |progress| {
            stages.push(progress.stage);
        })
        .unwrap();
        assert!(matches!(copy_result, WriteImageResult::NotSmaller { .. }));
        assert_eq!(
            stages,
            vec![
                ProcessingStage::Preparing,
                ProcessingStage::Decoding,
                ProcessingStage::Optimizing,
                ProcessingStage::Encoding,
            ]
        );
        assert!(!copy_directory.exists());
        assert_eq!(fs::read(&optimized).unwrap(), before);

        let mut replace_request = request(
            optimized.clone(),
            root.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        );
        replace_request.operation = WriteOperation::Compress;
        let replace_result = write_transformed_image(replace_request).unwrap();
        assert!(matches!(
            replace_result,
            WriteImageResult::NotSmaller { .. }
        ));
        assert_eq!(fs::read(optimized).unwrap(), before);
    }

    #[test]
    fn automatic_directory_is_created_only_when_an_output_will_be_written() {
        let root = TestDirectory::new();
        let source = write_source(&root.0, "source.png");
        let automatic = root.0.join("PixChisel Copies");
        let mut request = WriteImageRequest {
            source_path: source,
            settings: settings(OutputFormat::Png),
            destination: WriteDestination::AutomaticDirectory {
                path: automatic.clone(),
            },
            operation: WriteOperation::Resize,
        };
        request.settings.resize.mode = ResizeMode::Width;
        request.settings.resize.width = 4;

        let result = write_transformed_image(request).unwrap();

        assert!(automatic.is_dir());
        assert!(written_path(&result).starts_with(&automatic));
    }

    #[test]
    fn replace_original_preserves_each_lossy_and_lossless_source_format() {
        for (name, format) in [
            ("photo.jpg", OutputFormat::Jpeg),
            ("graphic.png", OutputFormat::Png),
            ("asset.webp", OutputFormat::Webp),
        ] {
            let directory = TestDirectory::new();
            let source = write_source(&directory.0, name);
            let result = write_transformed_image(request(
                source.clone(),
                directory.0.clone(),
                OutputFormat::Original,
                ConflictPolicy::Overwrite,
            ))
            .unwrap();
            assert_eq!(written_path(&result), source);
            assert_eq!(detect_input_format(&source).unwrap(), format);
            assert_eq!(image::open(source).unwrap().dimensions(), (8, 4));
        }
    }

    #[test]
    fn replace_original_resize_changes_dimensions_in_place() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "resize.png");
        let mut request = request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Original,
            ConflictPolicy::Overwrite,
        );
        request.settings.resize.mode = ResizeMode::Width;
        request.settings.resize.width = 4;
        let result = write_transformed_image(request).unwrap();
        assert_eq!(written_path(&result), source);
        assert_eq!(image::open(source).unwrap().dimensions(), (4, 2));
    }

    #[test]
    fn replace_original_conversion_finalizes_then_removes_source() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "convert.png");
        let destination = directory.0.join("convert.jpg");
        let result = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Overwrite,
        ))
        .unwrap();
        assert_eq!(written_path(&result), destination);
        assert!(!source.exists());
        assert_eq!(
            detect_input_format(&destination).unwrap(),
            OutputFormat::Jpeg
        );
        assert_eq!(image::open(destination).unwrap().dimensions(), (8, 4));
    }

    #[test]
    fn replace_original_conversion_never_overwrites_an_unrelated_target() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "conflict.png");
        let source_before = fs::read(&source).unwrap();
        let destination = directory.0.join("conflict.jpg");
        fs::write(&destination, b"unrelated").unwrap();
        let error = write_transformed_image(request(
            source.clone(),
            directory.0.clone(),
            OutputFormat::Jpeg,
            ConflictPolicy::Overwrite,
        ))
        .unwrap_err();
        assert_eq!(error.code, WriteImageErrorCode::DestinationConflict);
        assert_eq!(fs::read(source).unwrap(), source_before);
        assert_eq!(fs::read(destination).unwrap(), b"unrelated");
    }

    #[test]
    fn replace_original_uses_each_sources_own_directory() {
        let root = TestDirectory::new();
        let first_directory = root.0.join("day-one");
        let second_directory = root.0.join("day-two/nested");
        fs::create_dir_all(&first_directory).unwrap();
        fs::create_dir_all(&second_directory).unwrap();
        for source in [
            write_source(&first_directory, "a.png"),
            write_source(&second_directory, "b.png"),
        ] {
            let result = write_transformed_image(WriteImageRequest {
                source_path: source.clone(),
                settings: settings(OutputFormat::Original),
                destination: WriteDestination::ReplaceOriginal,
                operation: WriteOperation::Resize,
            })
            .unwrap();
            assert_eq!(written_path(&result), source);
            assert!(source.exists());
        }
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
    fn progress_reports_real_pipeline_stages_in_order() {
        let directory = TestDirectory::new();
        let source = write_source(&directory.0, "stages.png");
        let mut stages = Vec::new();
        write_transformed_image_with_progress(
            request(
                source,
                directory.0.clone(),
                OutputFormat::Jpeg,
                ConflictPolicy::CreateCopy,
            ),
            |progress| stages.push((progress.stage, progress.percent)),
        )
        .unwrap();
        assert_eq!(
            stages,
            vec![
                (ProcessingStage::Preparing, 5),
                (ProcessingStage::Decoding, 20),
                (ProcessingStage::Optimizing, 45),
                (ProcessingStage::Encoding, 70),
                (ProcessingStage::Saving, 90),
                (ProcessingStage::Completed, 100),
            ]
        );
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
