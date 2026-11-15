pub mod archive;
pub mod local;
pub mod search_vfs;

use archive::ArchiveVfsProvider;
use local::LocalFsProvider;
use search_vfs::SearchVfsProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsEntry {
    pub name: String,
    pub stem: String,
    pub extension: String,
    pub path: String,
    pub is_dir: bool,
    pub is_parent: bool,
    pub is_archive: bool,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub kind: String,
    pub attributes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub provider_scheme: String,
    pub entries: Vec<VfsEntry>,
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_bytes: u64,
}

pub trait VfsProvider: Send + Sync {
    fn scheme(&self) -> &'static str;
    fn list_directory(&self, path: &str) -> Result<DirectoryListing, String>;
}

pub struct VfsManager {
    pub local: LocalFsProvider,
    pub archive: ArchiveVfsProvider,
    pub search: SearchVfsProvider,
}

impl VfsManager {
    pub fn new() -> Self {
        Self {
            local: LocalFsProvider::new(),
            archive: ArchiveVfsProvider::new(),
            search: SearchVfsProvider::new(),
        }
    }

    pub fn list(&self, raw_path: &str) -> Result<DirectoryListing, String> {
        if raw_path.starts_with("search://") {
            return self.search.list_directory(raw_path);
        }
        if raw_path.starts_with("archive://") {
            return self.archive.list_directory(raw_path);
        }
        if Self::is_archive_file_path(raw_path) && Path::new(raw_path).is_file() {
            let archive_uri = format!("archive://{}!/", raw_path);
            return self.archive.list_directory(&archive_uri);
        }
        self.local.list_directory(raw_path)
    }

    pub fn is_archive_file_path(path: &str) -> bool {
        let lower = path.to_ascii_lowercase();
        const ARCHIVE_EXTS: &[&str] = &[
            ".7z", ".zip", ".rar", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".zst", ".iso", ".wim",
            ".cab",
        ];
        ARCHIVE_EXTS.iter().any(|ext| lower.ends_with(ext))
    }
}
