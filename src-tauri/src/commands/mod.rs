use std::path::PathBuf;

use crate::imaging::{inspect_image_files, ImageInspectionResult, InspectImageError};

#[tauri::command]
pub async fn inspect_images(
    paths: Vec<PathBuf>,
) -> Result<Vec<ImageInspectionResult>, InspectImageError> {
    tauri::async_runtime::spawn_blocking(move || inspect_image_files(paths))
        .await
        .map_err(|_| InspectImageError::internal())
}
