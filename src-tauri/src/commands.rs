use crate::search::{DualSearchCoordinator, EngineStatus, SearchQuery, SearchResponse};
use crate::vfs::archive::{CompressOptions, ExtractOptions};
use crate::vfs::local::LocalFsProvider;
use crate::vfs::{DirectoryListing, DriveInfo};
use crate::AppState;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_drives(state: State<'_, AppState>) -> Result<Vec<DriveInfo>, String> {
    let vfs = state.vfs.read();
    Ok(vfs.local.list_drives())
}

#[tauri::command]
pub fn get_startup_paths() -> Result<String, String> {
    Ok(LocalFsProvider::default_home_or_c())
}

#[tauri::command]
pub fn get_cpu_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

#[tauri::command]
pub async fn pick_folder_native(initial_dir: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new().set_title("Seleziona cartella");
        if let Some(ref dir) = initial_dir {
            let p = Path::new(dir);
            if p.is_dir() {
                dialog = dialog.set_directory(p);
            } else if let Some(parent) = p.parent() {
                if parent.is_dir() {
                    dialog = dialog.set_directory(parent);
                }
            }
        }
        Ok(dialog.pick_folder().map(|p| p.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| format!("Errore apertura dialogo cartella: {}", e))?
}

#[tauri::command]
pub fn read_vfs_directory(
    path: String,
    state: State<'_, AppState>,
) -> Result<DirectoryListing, String> {
    let vfs = state.vfs.read();
    vfs.list(&path)
}

#[tauri::command]
pub fn create_directory(parent_path: String, folder_name: String) -> Result<String, String> {
    let trimmed = folder_name.trim();
    if trimmed.is_empty() {
        return Err("Il nome della cartella non può essere vuoto".to_string());
    }
    let target = Path::new(&parent_path).join(trimmed);
    fs::create_dir_all(&target)
        .map_err(|e| format!("Errore creazione cartella '{}': {}", target.display(), e))?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_item(old_path: String, new_name: String) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Il nuovo nome non può essere vuoto".to_string());
    }
    let src = Path::new(&old_path);
    let parent = src
        .parent()
        .ok_or_else(|| "Percorso padre non valido".to_string())?;
    let dst = parent.join(trimmed);
    fs::rename(src, &dst)
        .map_err(|e| format!("Impossibile rinominare in '{}': {}", dst.display(), e))?;
    Ok(dst.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_items(paths: Vec<String>) -> Result<usize, String> {
    let mut deleted = 0usize;
    for raw in paths {
        let p = Path::new(&raw);
        if !p.exists() {
            continue;
        }
        if p.is_dir() {
            fs::remove_dir_all(p)
                .map_err(|e| format!("Impossibile eliminare cartella '{}': {}", raw, e))?;
        } else {
            fs::remove_file(p)
                .map_err(|e| format!("Impossibile eliminare file '{}': {}", raw, e))?;
        }
        deleted += 1;
    }
    Ok(deleted)
}

#[tauri::command]
pub fn copy_items(sources: Vec<String>, destination_dir: String) -> Result<usize, String> {
    let dest_root = Path::new(&destination_dir);
    if !dest_root.is_dir() {
        return Err(format!(
            "La destinazione '{}' non è una cartella valida",
            destination_dir
        ));
    }

    let mut count = 0usize;
    for src_str in sources {
        let src_path = Path::new(&src_str);
        let file_name = src_path
            .file_name()
            .ok_or_else(|| format!("Nome file non valido: {}", src_str))?;
        let target_path = resolve_collision(dest_root.join(file_name));

        if src_path.is_dir() {
            copy_dir_recursive(src_path, &target_path)?;
        } else {
            fs::copy(src_path, &target_path)
                .map_err(|e| format!("Errore copia '{}': {}", src_str, e))?;
        }
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
pub fn move_items(sources: Vec<String>, destination_dir: String) -> Result<usize, String> {
    let dest_root = Path::new(&destination_dir);
    if !dest_root.is_dir() {
        return Err(format!(
            "La destinazione '{}' non è una cartella valida",
            destination_dir
        ));
    }

    let mut count = 0usize;
    for src_str in sources {
        let src_path = Path::new(&src_str);
        let file_name = src_path
            .file_name()
            .ok_or_else(|| format!("Nome elemento non valido: {}", src_str))?;
        let target_path = dest_root.join(file_name);

        if fs::rename(src_path, &target_path).is_err() {
            if src_path.is_dir() {
                copy_dir_recursive(src_path, &target_path)?;
                fs::remove_dir_all(src_path)
                    .map_err(|e| format!("Errore pulizia sorgente '{}': {}", src_str, e))?;
            } else {
                fs::copy(src_path, &target_path)
                    .map_err(|e| format!("Errore copia in spostamento '{}': {}", src_str, e))?;
                fs::remove_file(src_path)
                    .map_err(|e| format!("Errore rimozione sorgente '{}': {}", src_str, e))?;
            }
        }
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
pub fn read_file_preview(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    if !p.is_file() {
        return Err("L'elemento selezionato non è un file regolare sul disco.".to_string());
    }
    let mut file = fs::File::open(p).map_err(|e| format!("Impossibile aprire il file: {}", e))?;
    let mut buf = vec![0u8; 32 * 1024];
    let n = file
        .read(&mut buf)
        .map_err(|e| format!("Errore lettura: {}", e))?;
    buf.truncate(n);

    if buf.iter().take(512).any(|&b| b == 0) {
        let hex_lines: Vec<String> = buf
            .chunks(16)
            .take(64)
            .enumerate()
            .map(|(idx, chunk)| {
                let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
                let ascii: String = chunk
                    .iter()
                    .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                    .collect();
                format!("{:08X}  {:<48}  {}", idx * 16, hex.join(" "), ascii)
            })
            .collect();
        Ok(format!(
            "[Vista Esadecimale Binaria - Prime 64 righe]\n\n{}",
            hex_lines.join("\n")
        ))
    } else {
        Ok(String::from_utf8_lossy(&buf).to_string())
    }
}

#[tauri::command]
pub fn execute_search(
    params: SearchQuery,
    state: State<'_, AppState>,
) -> Result<SearchResponse, String> {
    let coordinator = DualSearchCoordinator::new();
    let response = coordinator.execute(&params);
    {
        let vfs = state.vfs.read();
        vfs.search
            .store_results(&response.vfs_uri, response.results.clone());
    }
    Ok(response)
}

#[tauri::command]
pub fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let vfs = state.vfs.read();
    let coordinator = DualSearchCoordinator::new();
    Ok(coordinator.probe_status(
        vfs.archive.is_loaded(),
        vfs.archive.loaded_path(),
        vfs.archive.supported_formats(),
        vfs.archive.diagnostics.clone(),
    ))
}

#[tauri::command]
pub fn reload_archive_engine(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    {
        let mut vfs = state.vfs.write();
        vfs.reload_archive_dll();
    }
    get_engine_status(state)
}

#[tauri::command]
pub fn cancel_archive_operation(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn compress_Selection(
    app: AppHandle,
    options: CompressOptions,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let vfs_arc = Arc::clone(&state.vfs);
    let cancel_flag = Arc::clone(&state.cancel_flag);
    cancel_flag.store(false, Ordering::SeqCst);

    tauri::async_runtime::spawn_blocking(move || {
        let vfs = vfs_arc.read();
        vfs.archive.compress_items_with_options(&options, &app, cancel_flag)
    })
    .await
    .map_err(|e| format!("Errore thread compressione: {}", e))?
}

#[tauri::command]
pub async fn extract_archive(
    app: AppHandle,
    options: ExtractOptions,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let vfs_arc = Arc::clone(&state.vfs);
    let cancel_flag = Arc::clone(&state.cancel_flag);
    cancel_flag.store(false, Ordering::SeqCst);

    tauri::async_runtime::spawn_blocking(move || {
        let vfs = vfs_arc.read();
        vfs.archive.extract_archive_with_options(&options, &app, cancel_flag)
    })
    .await
    .map_err(|e| format!("Errore thread estrazione: {}", e))?
}

#[tauri::command]
pub fn open_in_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &format!("cd /d \"{}\"", path)])
            .spawn()
            .map_err(|e| format!("Impossibile avviare il terminale: {}", e))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Errore creazione '{}': {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let dest_entry = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_entry)?;
        } else {
            fs::copy(entry.path(), dest_entry).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn resolve_collision(target: PathBuf) -> PathBuf {
    if !target.exists() {
        return target;
    }
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    let stem = target
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let ext = target
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for i in 2..1000 {
        let candidate = parent.join(format!("{} copy {}{}", stem, i, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    target
}
