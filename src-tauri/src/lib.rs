pub mod commands;
pub mod search;
pub mod vfs;

use commands::*;
use parking_lot::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use vfs::VfsManager;

pub struct AppState {
    pub vfs: Arc<RwLock<VfsManager>>,
    pub cancel_flag: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let vfs_manager = Arc::new(RwLock::new(VfsManager::new()));
    let cancel_flag = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .manage(AppState {
            vfs: vfs_manager,
            cancel_flag,
        })
        .invoke_handler(tauri::generate_handler![
            list_drives,
            get_startup_paths,
            get_cpu_threads,
            pick_folder_native,
            read_vfs_directory,
            create_directory,
            rename_item,
            delete_items,
            copy_items,
            move_items,
            read_file_preview,
            execute_search,
            get_engine_status,
            reload_archive_engine,
            compress_Selection,
            extract_archive,
            cancel_archive_operation,
            open_in_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KZip Tauri application");
}
