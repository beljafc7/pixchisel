mod commands;
mod imaging;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(imaging::ThumbnailCache::new(app.handle())?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::inspect_images,
            commands::generate_thumbnails,
            commands::release_thumbnails,
            commands::clear_thumbnail_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
