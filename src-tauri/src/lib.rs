pub mod commands;
pub mod search;
pub mod vfs;

use commands::*;
use parking_lot::RwLock;
use std::sync::Arc;
use vfs::VfsManager;

pub struct AppState {
    pub vfs: Arc<RwLock<VfsManager>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let vfs_manager = Arc::new(RwLock::new(VfsManager::new()));

    tauri::Builder::default()
        .manage(AppState { vfs: vfs_manager })
        .invoke_handler(tauri::generate_handler![
            list_drives,
            read_vfs_directory,
            create_directory,
            delete_items,
            copy_items,
            move_items,
            read_file_preview,
            execute_search,
            get_engine_status,
            open_in_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KZip Tauri application");
}
