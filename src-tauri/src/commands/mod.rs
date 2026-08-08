use std::path::PathBuf;

use crate::imaging::{inspect_image_file, ImageInspection, InspectImageError};

#[tauri::command]
pub async fn inspect_image(path: PathBuf) -> Result<ImageInspection, InspectImageError> {
    tauri::async_runtime::spawn_blocking(move || inspect_image_file(&path))
        .await
        .map_err(|_| InspectImageError::internal())?
}
