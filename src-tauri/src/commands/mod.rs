use std::path::PathBuf;

use tauri::State;

use crate::imaging::{
    generate_thumbnail_files, inspect_image_files, ImageInspectionResult, InspectImageError,
    ThumbnailCache, ThumbnailError, ThumbnailResult,
};

#[tauri::command]
pub async fn inspect_images(
    paths: Vec<PathBuf>,
) -> Result<Vec<ImageInspectionResult>, InspectImageError> {
    tauri::async_runtime::spawn_blocking(move || inspect_image_files(paths))
        .await
        .map_err(|_| InspectImageError::internal())
}

#[tauri::command]
pub async fn generate_thumbnails(
    paths: Vec<PathBuf>,
    cache: State<'_, ThumbnailCache>,
) -> Result<Vec<ThumbnailResult>, ThumbnailError> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || generate_thumbnail_files(paths, cache))
        .await
        .map_err(|_| ThumbnailError::internal())
}

#[tauri::command]
pub async fn release_thumbnails(
    paths: Vec<PathBuf>,
    cache: State<'_, ThumbnailCache>,
) -> Result<(), ThumbnailError> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cache.release(&paths))
        .await
        .map_err(|_| ThumbnailError::internal())?
}

#[tauri::command]
pub async fn clear_thumbnail_cache(cache: State<'_, ThumbnailCache>) -> Result<(), ThumbnailError> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cache.clear())
        .await
        .map_err(|_| ThumbnailError::internal())?
}
