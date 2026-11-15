use super::SearchQuery;
use crate::vfs::local::LocalFsProvider;
use crate::vfs::VfsEntry;
use ignore::{WalkBuilder, WalkState};
use parking_lot::Mutex;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct FallbackRustSearchEngine;

impl FallbackRustSearchEngine {
    pub fn search(params: &SearchQuery) -> Vec<VfsEntry> {
        let root_path = params
            .scope_path
            .as_ref()
            .filter(|s| !s.trim().is_empty() && !s.starts_with("search://"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\"));

        let ext_filter: Option<HashSet<String>> = params.extensions.as_ref().and_then(|raw| {
            let set: HashSet<String> = raw
                .split([';', ',', ' '])
                .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if set.is_empty() {
                None
            } else {
                Some(set)
            }
        });

        let compiled_regex: Option<Regex> = if params.use_regex && !params.query.is_empty() {
            RegexBuilder::new(&params.query)
                .case_insensitive(!params.match_case)
                .build()
                .ok()
        } else {
            None
        };

        let query_needle = if params.match_case {
            params.query.trim().to_string()
        } else {
            params.query.trim().to_lowercase()
        };

        let max_results = params.max_results.max(50);
        let found_count = Arc::new(AtomicUsize::new(0));
        let results = Arc::new(Mutex::new(Vec::with_capacity(max_results.min(1024))));

        let walker = WalkBuilder::new(&root_path)
            .hidden(false)
            .git_ignore(false)
            .ignore(false)
            .parents(false)
            .threads(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
            .build_parallel();

        walker.run(|| {
            let results = Arc::clone(&results);
            let found_count = Arc::clone(&found_count);
            let ext_filter = ext_filter.clone();
            let compiled_regex = compiled_regex.clone();
            let query_needle = query_needle.clone();
            let match_case = params.match_case;
            let use_regex = params.use_regex;
            let min_size = params.min_size;
            let max_size = params.max_size;

            Box::new(move |entry_res| {
                if found_count.load(Ordering::Relaxed) >= max_results {
                    return WalkState::Quit;
                }

                let dir_entry = match entry_res {
                    Ok(e) => e,
                    Err(_) => return WalkState::Continue,
                };

                if dir_entry.depth() == 0 {
                    return WalkState::Continue;
                }

                let path = dir_entry.path();
                let file_name = match path.file_name() {
                    Some(n) => n.to_string_lossy(),
                    None => return WalkState::Continue,
                };

                if !query_needle.is_empty() {
                    if use_regex {
                        if let Some(ref rx) = compiled_regex {
                            if !rx.is_match(&file_name) {
                                return WalkState::Continue;
                            }
                        }
                    } else if match_case {
                        if !file_name.contains(&query_needle) {
                            return WalkState::Continue;
                        }
                    } else if !file_name.to_lowercase().contains(&query_needle) {
                        return WalkState::Continue;
                    }
                }

                if let Some(ref allowed_exts) = ext_filter {
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase())
                        .unwrap_or_default();
                    if !allowed_exts.contains(&ext) {
                        return WalkState::Continue;
                    }
                }

                if let Some(vfs_entry) = LocalFsProvider::entry_from_path(path) {
                    if !vfs_entry.is_dir {
                        if let Some(min_s) = min_size {
                            if vfs_entry.size < min_s {
                                return WalkState::Continue;
                            }
                        }
                        if let Some(max_s) = max_size {
                            if vfs_entry.size > max_s {
                                return WalkState::Continue;
                            }
                        }
                    }

                    let prev = found_count.fetch_add(1, Ordering::Relaxed);
                    if prev < max_results {
                        results.lock().push(vfs_entry);
                    } else {
                        return WalkState::Quit;
                    }
                }

                WalkState::Continue
            })
        });

        let mut final_vec = results.lock().clone();
        final_vec.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        final_vec
    }
}
