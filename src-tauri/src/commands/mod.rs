use std::path::PathBuf;

use tauri::ipc::Channel;
use tauri::State;

use crate::filesystem::{
    open_output_folder as open_folder, preflight_output_directory as preflight_directory,
    write_transformed_image_with_progress as write_image_file, ProcessingProgress, WriteImageError,
    WriteImageRequest, WriteImageResult,
};
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
pub async fn preflight_output_directory(directory: PathBuf) -> Result<(), WriteImageError> {
    tauri::async_runtime::spawn_blocking(move || preflight_directory(&directory))
        .await
        .map_err(|_| WriteImageError::internal())?
}

#[tauri::command]
pub async fn open_output_folder(directory: PathBuf) -> Result<(), WriteImageError> {
    tauri::async_runtime::spawn_blocking(move || open_folder(&directory))
        .await
        .map_err(|_| WriteImageError::internal())?
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

#[tauri::command]
pub async fn write_transformed_image(
    request: WriteImageRequest,
    on_progress: Channel<ProcessingProgress>,
) -> Result<WriteImageResult, WriteImageError> {
    tauri::async_runtime::spawn_blocking(move || {
        write_image_file(request, |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|_| WriteImageError::internal())?
}
