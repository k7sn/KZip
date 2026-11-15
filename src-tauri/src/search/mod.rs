pub mod everything_ipc;
pub mod fallback;

use crate::vfs::VfsEntry;
use everything_ipc::EverythingPureRustClient;
use fallback::FallbackRustSearchEngine;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub query: String,
    pub scope_path: Option<String>,
    pub extensions: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub use_regex: bool,
    pub match_case: bool,
    pub max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub engine_used: String,
    pub elapsed_ms: u128,
    pub vfs_uri: String,
    pub results: Vec<VfsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub everything_dll_loaded: bool,
    pub everything_service_connected: bool,
    pub everything_version: Option<String>,
    pub active_search_engine: String,
    pub archive_dll_loaded: bool,
    pub archive_formats_count: u32,
}

pub struct DualSearchCoordinator {
    client: EverythingPureRustClient,
}

impl DualSearchCoordinator {
    pub fn new() -> Self {
        Self {
            client: EverythingPureRustClient::new(),
        }
    }

    pub fn execute(&self, params: &SearchQuery) -> SearchResponse {
        let start = Instant::now();

        if let Ok(results) = self.client.search(params) {
            return SearchResponse {
                engine_used: "Everything 1.5 Pure-Rust IPC/Pipe Client".to_string(),
                elapsed_ms: start.elapsed().as_millis(),
                vfs_uri: format!("search://{}", params.query),
                results,
            };
        }

        let results = FallbackRustSearchEngine::search(params);
        SearchResponse {
            engine_used: "Pure-Rust Parallel Fallback (ignore + regex)".to_string(),
            elapsed_ms: start.elapsed().as_millis(),
            vfs_uri: format!("search://{}", params.query),
            results,
        }
    }

    pub fn probe_status(&self, archive_loaded: bool, archive_formats: u32) -> EngineStatus {
        match self.client.probe_status() {
            Some(version) => EngineStatus {
                everything_dll_loaded: true,
                everything_service_connected: true,
                everything_version: Some(version),
                active_search_engine: "Everything 1.5 Native Pipe/IPC (Pure Rust)".to_string(),
                archive_dll_loaded: archive_loaded,
                archive_formats_count: archive_formats,
            },
            None => EngineStatus {
                everything_dll_loaded: false,
                everything_service_connected: false,
                everything_version: None,
                active_search_engine: "Pure-Rust Parallel Fallback (ignore + regex)".to_string(),
                archive_dll_loaded: archive_loaded,
                archive_formats_count: archive_formats,
            },
        }
    }
}
