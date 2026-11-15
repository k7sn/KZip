use super::{DirectoryListing, DriveInfo, VfsEntry, VfsManager, VfsProvider};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use sysinfo::Disks;

pub struct LocalFsProvider;

impl LocalFsProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn list_drives(&self) -> Vec<DriveInfo> {
        let disks = Disks::new_with_refreshed_list();
        let mut drives = Vec::new();

        for disk in disks.list() {
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            let raw_name = disk.name().to_string_lossy().to_string();
            let display_name = if raw_name.trim().is_empty() {
                format!("Local Disk ({})", mount_point.trim_end_matches('\\'))
            } else {
                format!("{} ({})", raw_name, mount_point.trim_end_matches('\\'))
            };

            drives.push(DriveInfo {
                name: display_name,
                mount_point,
                file_system: disk.file_system().to_string_lossy().to_string(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                is_removable: disk.is_removable(),
            });
        }

        if drives.is_empty() {
            drives.push(DriveInfo {
                name: "System (C:)".to_string(),
                mount_point: "C:\\".to_string(),
                file_system: "NTFS".to_string(),
                total_space: 250_000_000_000,
                available_space: 120_000_000_000,
                is_removable: false,
            });
        }

        drives
    }

    pub fn entry_from_path(path: &Path) -> Option<VfsEntry> {
        let metadata = fs::metadata(path).ok()?;
        let is_dir = metadata.is_dir();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let (stem, extension) = if is_dir {
            (file_name.clone(), String::new())
        } else {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let st = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file_name.clone());
            (st, ext)
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(modified);

        let path_str = path.to_string_lossy().to_string();
        let is_archive = !is_dir && VfsManager::is_archive_file_path(&path_str);
        let kind = Self::describe_kind(is_dir, is_archive, &extension);

        #[cfg(target_os = "windows")]
        let attributes = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes()
        };
        #[cfg(not(target_os = "windows"))]
        let attributes = 0u32;

        Some(VfsEntry {
            name: file_name,
            stem,
            extension,
            path: path_str,
            is_dir,
            is_parent: false,
            is_archive,
            size: if is_dir { 0 } else { metadata.len() },
            modified,
            created,
            kind,
            attributes,
        })
    }

    pub fn describe_kind(is_dir: bool, is_archive: bool, ext: &str) -> String {
        if is_dir {
            return "folder".to_string();
        }
        if is_archive {
            return "archive".to_string();
        }
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "svg" | "ico" => "image".to_string(),
            "mp4" | "mkv" | "mov" | "avi" | "webm" => "video".to_string(),
            "mp3" | "flac" | "wav" | "ogg" | "m4a" => "audio".to_string(),
            "exe" | "msi" | "bat" | "cmd" | "ps1" => "executable".to_string(),
            "rs" | "ts" | "tsx" | "js" | "json" | "toml" | "c" | "cpp" | "h" | "py" | "css"
            | "html" | "md" => "code".to_string(),
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" => {
                "document".to_string()
            }
            _ => "file".to_string(),
        }
    }
}

impl VfsProvider for LocalFsProvider {
    fn scheme(&self) -> &'static str {
        "local"
    }

    fn list_directory(&self, raw_path: &str) -> Result<DirectoryListing, String> {
        let target_path = if raw_path.trim().is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"))
        } else {
            PathBuf::from(raw_path)
        };

        let canonical = fs::canonicalize(&target_path).unwrap_or_else(|_| target_path.clone());
        let clean_current = clean_windows_unc(&canonical);
        let parent_path = clean_current
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .filter(|p| !p.is_empty() && p != &clean_current.to_string_lossy());

        let mut entries = Vec::new();
        let mut total_files = 0usize;
        let mut total_dirs = 0usize;
        let mut total_bytes = 0u64;

        if let Some(ref parent) = parent_path {
            let parent_modified = fs::metadata(parent)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            entries.push(VfsEntry {
                name: "..".to_string(),
                stem: "..".to_string(),
                extension: String::new(),
                path: parent.clone(),
                is_dir: true,
                is_parent: true,
                is_archive: false,
                size: 0,
                modified: parent_modified,
                created: parent_modified,
                kind: "folder".to_string(),
                attributes: 0x10,
            });
        }

        let read_dir = fs::read_dir(&clean_current)
            .map_err(|e| format!("Impossibile leggere '{}': {}", clean_current.display(), e))?;

        let mut items = Vec::new();
        for item in read_dir.flatten() {
            if let Some(entry) = Self::entry_from_path(&item.path()) {
                if entry.is_dir {
                    total_dirs += 1;
                } else {
                    total_files += 1;
                    total_bytes = total_bytes.saturating_add(entry.size);
                }
                items.push(entry);
            }
        }

        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        entries.extend(items);

        Ok(DirectoryListing {
            current_path: clean_current.to_string_lossy().to_string(),
            parent_path,
            provider_scheme: self.scheme().to_string(),
            entries,
            total_files,
            total_dirs,
            total_bytes,
        })
    }
}

fn clean_windows_unc(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}
