use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverImagesError {
    pub message: String,
}

impl DiscoverImagesError {
    pub fn internal() -> Self {
        Self {
            message: "The selected folder could not be scanned.".into(),
        }
    }
}

pub fn discover_image_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, DiscoverImagesError> {
    let mut discovered = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                push_unique(&mut discovered, &mut seen, path);
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let mut folder_paths = Vec::new();
            scan_directory(&path, &mut folder_paths)?;
            folder_paths
                .sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
            for image in folder_paths {
                push_unique(&mut discovered, &mut seen, image);
            }
        } else {
            // Explicit files continue to content inspection, including unsupported files.
            push_unique(&mut discovered, &mut seen, path);
        }
    }
    Ok(discovered)
}

fn scan_directory(directory: &Path, images: &mut Vec<PathBuf>) -> Result<(), DiscoverImagesError> {
    let entries = fs::read_dir(directory).map_err(|_| DiscoverImagesError::internal())?;
    for entry in entries {
        let entry = entry.map_err(|_| DiscoverImagesError::internal())?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|_| DiscoverImagesError::internal())?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            scan_directory(&path, images)?;
        } else if file_type.is_file() && has_supported_extension(&path) {
            images.push(path);
        }
    }
    Ok(())
}

fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp"
            )
        })
}

fn push_unique(output: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    if seen.insert(path.clone()) {
        output.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let number = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pixchisel-discovery-{}-{number}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self, relative: &str) -> PathBuf {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, b"fixture").unwrap();
            path
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recursively_finds_supported_extensions_and_ignores_other_content() {
        let directory = TestDirectory::new();
        let jpg = directory.file("photo.jpg");
        let png = directory.file("nested/logo.PNG");
        let webp = directory.file("nested/deeper/image.WEBP");
        directory.file("notes.txt");
        directory.file("document.pdf");
        directory.file(".hidden.jpg");
        directory.file(".hidden/inside.png");

        let result = discover_image_paths(vec![directory.0.clone()]).unwrap();
        assert_eq!(result, vec![webp, png, jpg]);
    }

    #[test]
    fn empty_folder_returns_no_images() {
        let directory = TestDirectory::new();
        assert!(discover_image_paths(vec![directory.0.clone()])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn mixed_and_overlapping_inputs_are_deduplicated_in_stable_order() {
        let directory = TestDirectory::new();
        let direct = directory.file("a.jpg");
        let nested = directory.file("nested/b.png");
        let result = discover_image_paths(vec![
            direct.clone(),
            directory.0.clone(),
            directory.0.join("nested"),
        ])
        .unwrap();
        assert_eq!(result, vec![direct, nested]);
    }

    #[cfg(unix)]
    #[test]
    fn directory_symlinks_are_not_followed() {
        use std::os::unix::fs::symlink;
        let directory = TestDirectory::new();
        let image = directory.file("real/photo.jpg");
        symlink(&directory.0, directory.0.join("real/cycle")).unwrap();
        assert_eq!(
            discover_image_paths(vec![directory.0.clone()]).unwrap(),
            vec![image]
        );
    }

    #[test]
    fn explicit_unsupported_file_is_preserved_for_actionable_inspection() {
        let directory = TestDirectory::new();
        let unsupported = directory.file("notes.txt");
        assert_eq!(
            discover_image_paths(vec![unsupported.clone()]).unwrap(),
            vec![unsupported]
        );
    }
}
