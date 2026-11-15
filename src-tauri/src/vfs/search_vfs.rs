use super::{DirectoryListing, VfsEntry, VfsProvider};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct SearchVfsProvider {
    cache: Arc<RwLock<HashMap<String, Vec<VfsEntry>>>>,
}

impl SearchVfsProvider {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn store_results(&self, uri: &str, results: Vec<VfsEntry>) {
        self.cache.write().insert(uri.to_string(), results);
    }
}

impl VfsProvider for SearchVfsProvider {
    fn scheme(&self) -> &'static str {
        "search"
    }

    fn list_directory(&self, path: &str) -> Result<DirectoryListing, String> {
        let map = self.cache.read();
        let stored = map.get(path).cloned().unwrap_or_default();

        let mut total_files = 0usize;
        let mut total_dirs = 0usize;
        let mut total_bytes = 0u64;

        for e in &stored {
            if e.is_dir {
                total_dirs += 1;
            } else {
                total_files += 1;
                total_bytes = total_bytes.saturating_add(e.size);
            }
        }

        Ok(DirectoryListing {
            current_path: path.to_string(),
            parent_path: None,
            provider_scheme: self.scheme().to_string(),
            entries: stored,
            total_files,
            total_dirs,
            total_bytes,
        })
    }
}
