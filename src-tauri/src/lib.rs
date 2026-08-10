mod commands;
mod filesystem;
mod imaging;

pub use filesystem::{
    write_transformed_image, ConflictPolicy, ProcessingProgress, WriteDestination, WriteImageError,
    WriteImageErrorCode, WriteImageRequest, WriteImageResult, WriteOperation,
};
pub use imaging::transform::{
    transform_image, BatchSettings, EncodedTransformation, MetadataDisposition, OutputFormat,
    ProcessingStage, ResizeMode, ResizeSettings, TransformError, TransformationMetadata,
    ValidationError, ValidationErrorCode,
};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(imaging::ThumbnailCache::new(app.handle())?);
            app.manage(commands::ProcessingCancellation::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_images,
            commands::inspect_images,
            commands::generate_thumbnails,
            commands::release_thumbnails,
            commands::clear_thumbnail_cache,
            commands::write_transformed_image,
            commands::cancel_processing,
            commands::clear_processing_cancellation,
            commands::preflight_output_directory,
            commands::open_output_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
