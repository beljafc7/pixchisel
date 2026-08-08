use std::{fs, path::Path, process::Command};

use super::{output::validate_output_directory, WriteImageError, WriteImageErrorCode};

pub fn open_output_folder(directory: &Path) -> Result<(), WriteImageError> {
    validate_output_directory(directory)?;

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(directory).spawn();

    #[cfg(target_os = "windows")]
    let result = Command::new("explorer.exe").arg(directory).spawn();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported platform",
    ));

    result.map(|_| ()).map_err(|_| {
        WriteImageError::new(
            WriteImageErrorCode::OpenOutputFolderFailed,
            "The output folder could not be opened.",
        )
    })
}

pub fn preflight_output_directory(directory: &Path) -> Result<(), WriteImageError> {
    validate_output_directory(directory)?;
    let probe = directory.join(format!(".pixchisel-write-check-{}", std::process::id()));
    match fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            fs::remove_file(&probe).map_err(|_| {
                WriteImageError::new(
                    WriteImageErrorCode::CleanupFailed,
                    "PixChisel could write to the output folder, but its check file could not be removed.",
                )
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Err(
            WriteImageError::new(
                WriteImageErrorCode::OutputDirectoryNotWritable,
                "PixChisel does not have permission to write to the selected output folder.",
            ),
        ),
        Err(_) => Err(WriteImageError::new(
            WriteImageErrorCode::OutputDirectoryNotWritable,
            "PixChisel cannot write to the selected output folder. Check that it is still connected and has available space.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_accepts_a_writable_directory_and_cleans_up() {
        let directory =
            std::env::temp_dir().join(format!("pixchisel-preflight-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();

        preflight_output_directory(&directory).unwrap();
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn preflight_rejects_a_missing_directory() {
        let directory = std::env::temp_dir().join(format!(
            "pixchisel-missing-preflight-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);

        let error = preflight_output_directory(&directory).unwrap_err();
        assert_eq!(error.code, WriteImageErrorCode::OutputDirectoryMissing);
    }
}
