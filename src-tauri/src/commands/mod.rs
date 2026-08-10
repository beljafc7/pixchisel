use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use tauri::ipc::Channel;
use tauri::State;

use crate::filesystem::{
    open_output_folder as open_folder, preflight_output_directory as preflight_directory,
    write_transformed_image_with_progress_and_cancellation as write_image_file, ProcessingProgress,
    WriteImageError, WriteImageRequest, WriteImageResult,
};

#[derive(Default)]
pub struct ProcessingCancellation {
    batches: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ProcessingCancellation {
    fn flag(&self, id: &str) -> Arc<AtomicBool> {
        let mut batches = self
            .batches
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        batches
            .entry(id.to_owned())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    fn cancel(&self, id: &str) {
        self.flag(id).store(true, Ordering::Release);
    }

    fn clear(&self, id: &str) {
        self.batches
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(id);
    }
}
use crate::imaging::{
    discover_image_paths, generate_thumbnail_files, inspect_image_files, DiscoverImagesError,
    ImageInspectionResult, InspectImageError, ThumbnailCache, ThumbnailError, ThumbnailResult,
};

#[tauri::command]
pub async fn discover_images(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, DiscoverImagesError> {
    tauri::async_runtime::spawn_blocking(move || discover_image_paths(paths))
        .await
        .map_err(|_| DiscoverImagesError::internal())?
}

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
    cancellation_id: String,
    cancellation: State<'_, ProcessingCancellation>,
) -> Result<WriteImageResult, WriteImageError> {
    let flag = cancellation.flag(&cancellation_id);
    tauri::async_runtime::spawn_blocking(move || {
        write_image_file(
            request,
            |progress| {
                let _ = on_progress.send(progress);
            },
            || flag.load(Ordering::Acquire),
        )
    })
    .await
    .map_err(|_| WriteImageError::internal())?
}

#[tauri::command]
pub fn cancel_processing(cancellation_id: String, cancellation: State<'_, ProcessingCancellation>) {
    cancellation.cancel(&cancellation_id);
}

#[tauri::command]
pub fn clear_processing_cancellation(
    cancellation_id: String,
    cancellation: State<'_, ProcessingCancellation>,
) {
    cancellation.clear(&cancellation_id);
}
