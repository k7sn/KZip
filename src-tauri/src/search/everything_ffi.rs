use super::SearchQuery;
use crate::vfs::archive::filetime_to_unix_seconds;
use crate::vfs::local::LocalFsProvider;
use crate::vfs::{VfsEntry, VfsManager};
use libloading::Library;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use widestring::U16CString;
use winapi::shared::minwindef::{BOOL, DWORD};
use winapi::um::winuser::{FindWindowW, SendMessageW, WM_USER};

pub const EVERYTHING_WM_IPC: u32 = WM_USER;
pub const EVERYTHING_IPC_GET_MAJOR_VERSION: usize = 0;
pub const EVERYTHING_IPC_GET_MINOR_VERSION: usize = 1;
pub const EVERYTHING_IPC_GET_REVISION: usize = 2;
pub const EVERYTHING_IPC_GET_BUILD_NUMBER: usize = 3;
pub const EVERYTHING_IPC_IS_DB_LOADED: usize = 401;
pub const EVERYTHING_IPC_WNDCLASSW: &str = "EVERYTHING_TASKBAR_NOTIFICATION";

pub const EVERYTHING3_OK: DWORD = 0;
pub const EVERYTHING3_PROPERTY_ID_NAME: DWORD = 0;
pub const EVERYTHING3_PROPERTY_ID_PATH: DWORD = 1;
pub const EVERYTHING3_PROPERTY_ID_SIZE: DWORD = 2;
pub const EVERYTHING3_PROPERTY_ID_EXTENSION: DWORD = 3;
pub const EVERYTHING3_PROPERTY_ID_DATE_MODIFIED: DWORD = 5;
pub const EVERYTHING3_PROPERTY_ID_DATE_CREATED: DWORD = 6;
pub const EVERYTHING3_PROPERTY_ID_ATTRIBUTES: DWORD = 8;
pub const EVERYTHING3_SEARCH_FOLDERS_FIRST_ALWAYS: DWORD = 1;

#[repr(C)]
pub struct EVERYTHING3_CLIENT {
    _private: [u8; 0],
}

#[repr(C)]
pub struct EVERYTHING3_SEARCH_STATE {
    _private: [u8; 0],
}

#[repr(C)]
pub struct EVERYTHING3_RESULT_LIST {
    _private: [u8; 0],
}

type FnGetLastError = unsafe extern "system" fn() -> DWORD;
type FnConnectW = unsafe extern "system" fn(instance_name: *const u16) -> *mut EVERYTHING3_CLIENT;
type FnDestroyClient = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> BOOL;
type FnGetMajorVersion = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> DWORD;
type FnGetMinorVersion = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> DWORD;
type FnGetRevision = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> DWORD;
type FnGetBuildNumber = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> DWORD;
type FnIsDBLoaded = unsafe extern "system" fn(client: *mut EVERYTHING3_CLIENT) -> BOOL;

type FnCreateSearchState = unsafe extern "system" fn() -> *mut EVERYTHING3_SEARCH_STATE;
type FnDestroySearchState =
    unsafe extern "system" fn(search_state: *mut EVERYTHING3_SEARCH_STATE) -> BOOL;
type FnSetSearchMatchCase = unsafe extern "system" fn(
    search_state: *mut EVERYTHING3_SEARCH_STATE,
    match_case: BOOL,
) -> BOOL;
type FnSetSearchRegex =
    unsafe extern "system" fn(search_state: *mut EVERYTHING3_SEARCH_STATE, match_regex: BOOL) -> BOOL;
type FnSetSearchFoldersFirst = unsafe extern "system" fn(
    search_state: *mut EVERYTHING3_SEARCH_STATE,
    folders_first_type: DWORD,
) -> BOOL;
type FnSetSearchTextW = unsafe extern "system" fn(
    search_state: *mut EVERYTHING3_SEARCH_STATE,
    search: *const u16,
) -> BOOL;
type FnAddSearchPropertyRequest = unsafe extern "system" fn(
    search_state: *mut EVERYTHING3_SEARCH_STATE,
    property_id: DWORD,
) -> BOOL;
type FnSetSearchViewportCount = unsafe extern "system" fn(
    search_state: *mut EVERYTHING3_SEARCH_STATE,
    count: usize,
) -> BOOL;
type FnSearch = unsafe extern "system" fn(
    client: *mut EVERYTHING3_CLIENT,
    search_state: *mut EVERYTHING3_SEARCH_STATE,
) -> *mut EVERYTHING3_RESULT_LIST;
type FnDestroyResultList =
    unsafe extern "system" fn(result_list: *mut EVERYTHING3_RESULT_LIST) -> BOOL;

type FnGetResultListViewportCount =
    unsafe extern "system" fn(result_list: *const EVERYTHING3_RESULT_LIST) -> usize;
type FnIsFolderResult = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
) -> BOOL;
type FnGetResultNameW = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
    out_wbuf: *mut u16,
    wbuf_size_in_wchars: usize,
) -> usize;
type FnGetResultPathW = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
    out_wbuf: *mut u16,
    wbuf_size_in_wchars: usize,
) -> usize;
type FnGetResultFullPathNameW = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
    out_wbuf: *mut u16,
    wbuf_size_in_wchars: usize,
) -> usize;
type FnGetResultSize = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
) -> u64;
type FnGetResultDateModified = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
) -> u64;
type FnGetResultDateCreated = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
) -> u64;
type FnGetResultAttributes = unsafe extern "system" fn(
    result_list: *const EVERYTHING3_RESULT_LIST,
    result_index: usize,
) -> DWORD;

pub struct EverythingSdk3Engine {
    _lib: Library,
    get_last_error: FnGetLastError,
    connect_w: FnConnectW,
    destroy_client: FnDestroyClient,
    get_major_version: FnGetMajorVersion,
    get_minor_version: FnGetMinorVersion,
    get_revision: FnGetRevision,
    get_build_number: FnGetBuildNumber,
    is_db_loaded: FnIsDBLoaded,
    create_search_state: FnCreateSearchState,
    destroy_search_state: FnDestroySearchState,
    set_search_match_case: FnSetSearchMatchCase,
    set_search_regex: FnSetSearchRegex,
    set_search_folders_first: FnSetSearchFoldersFirst,
    set_search_text_w: FnSetSearchTextW,
    add_search_property_request: FnAddSearchPropertyRequest,
    set_search_viewport_count: FnSetSearchViewportCount,
    search_fn: FnSearch,
    destroy_result_list: FnDestroyResultList,
    get_result_list_viewport_count: FnGetResultListViewportCount,
    is_folder_result: FnIsFolderResult,
    get_result_name_w: FnGetResultNameW,
    get_result_path_w: FnGetResultPathW,
    get_result_full_path_name_w: FnGetResultFullPathNameW,
    get_result_size: FnGetResultSize,
    get_result_date_modified: FnGetResultDateModified,
    get_result_date_created: FnGetResultDateCreated,
    get_result_attributes: FnGetResultAttributes,
}

impl EverythingSdk3Engine {
    pub fn try_load() -> Option<Arc<Self>> {
        for dir in Self::candidate_dirs() {
            let dll_path = dir.join("KZip.Search.dll");
            if !dll_path.exists() {
                continue;
            }
            if let Ok(lib) = unsafe { Library::new(&dll_path) } {
                if let Some(engine) = Self::bind_symbols(lib) {
                    return Some(Arc::new(engine));
                }
            }
        }
        None
    }

    fn candidate_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(p) = exe.parent() {
                dirs.push(p.to_path_buf());
                dirs.push(p.join("binaries"));
                dirs.push(p.join("bin"));
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.clone());
            dirs.push(cwd.join("binaries"));
            dirs.push(cwd.join("bin"));
            dirs.push(cwd.join("src-tauri").join("binaries"));
            dirs.push(cwd.join("src-tauri").join("bin"));
        }
        dirs
    }

    fn bind_symbols(lib: Library) -> Option<Self> {
        unsafe {
            let get_last_error = *lib.get::<FnGetLastError>(b"Everything3_GetLastError\0").ok()?;
            let connect_w = *lib.get::<FnConnectW>(b"Everything3_ConnectW\0").ok()?;
            let destroy_client =
                *lib.get::<FnDestroyClient>(b"Everything3_DestroyClient\0").ok()?;
            let get_major_version = *lib
                .get::<FnGetMajorVersion>(b"Everything3_GetMajorVersion\0")
                .ok()?;
            let get_minor_version = *lib
                .get::<FnGetMinorVersion>(b"Everything3_GetMinorVersion\0")
                .ok()?;
            let get_revision = *lib.get::<FnGetRevision>(b"Everything3_GetRevision\0").ok()?;
            let get_build_number = *lib
                .get::<FnGetBuildNumber>(b"Everything3_GetBuildNumber\0")
                .ok()?;
            let is_db_loaded = *lib.get::<FnIsDBLoaded>(b"Everything3_IsDBLoaded\0").ok()?;
            let create_search_state = *lib
                .get::<FnCreateSearchState>(b"Everything3_CreateSearchState\0")
                .ok()?;
            let destroy_search_state = *lib
                .get::<FnDestroySearchState>(b"Everything3_DestroySearchState\0")
                .ok()?;
            let set_search_match_case = *lib
                .get::<FnSetSearchMatchCase>(b"Everything3_SetSearchMatchCase\0")
                .ok()?;
            let set_search_regex = *lib
                .get::<FnSetSearchRegex>(b"Everything3_SetSearchRegex\0")
                .ok()?;
            let set_search_folders_first = *lib
                .get::<FnSetSearchFoldersFirst>(b"Everything3_SetSearchFoldersFirst\0")
                .ok()?;
            let set_search_text_w = *lib
                .get::<FnSetSearchTextW>(b"Everything3_SetSearchTextW\0")
                .ok()?;
            let add_search_property_request = *lib
                .get::<FnAddSearchPropertyRequest>(b"Everything3_AddSearchPropertyRequest\0")
                .ok()?;
            let set_search_viewport_count = *lib
                .get::<FnSetSearchViewportCount>(b"Everything3_SetSearchViewportCount\0")
                .ok()?;
            let search_fn = *lib.get::<FnSearch>(b"Everything3_Search\0").ok()?;
            let destroy_result_list = *lib
                .get::<FnDestroyResultList>(b"Everything3_DestroyResultList\0")
                .ok()?;
            let get_result_list_viewport_count = *lib
                .get::<FnGetResultListViewportCount>(b"Everything3_GetResultListViewportCount\0")
                .ok()?;
            let is_folder_result = *lib
                .get::<FnIsFolderResult>(b"Everything3_IsFolderResult\0")
                .ok()?;
            let get_result_name_w = *lib
                .get::<FnGetResultNameW>(b"Everything3_GetResultNameW\0")
                .ok()?;
            let get_result_path_w = *lib
                .get::<FnGetResultPathW>(b"Everything3_GetResultPathW\0")
                .ok()?;
            let get_result_full_path_name_w = *lib
                .get::<FnGetResultFullPathNameW>(b"Everything3_GetResultFullPathNameW\0")
                .ok()?;
            let get_result_size = *lib
                .get::<FnGetResultSize>(b"Everything3_GetResultSize\0")
                .ok()?;
            let get_result_date_modified = *lib
                .get::<FnGetResultDateModified>(b"Everything3_GetResultDateModified\0")
                .ok()?;
            let get_result_date_created = *lib
                .get::<FnGetResultDateCreated>(b"Everything3_GetResultDateCreated\0")
                .ok()?;
            let get_result_attributes = *lib
                .get::<FnGetResultAttributes>(b"Everything3_GetResultAttributes\0")
                .ok()?;

            Some(Self {
                _lib: lib,
                get_last_error,
                connect_w,
                destroy_client,
                get_major_version,
                get_minor_version,
                get_revision,
                get_build_number,
                is_db_loaded,
                create_search_state,
                destroy_search_state,
                set_search_match_case,
                set_search_regex,
                set_search_folders_first,
                set_search_text_w,
                add_search_property_request,
                set_search_viewport_count,
                search_fn,
                destroy_result_list,
                get_result_list_viewport_count,
                is_folder_result,
                get_result_name_w,
                get_result_path_w,
                get_result_full_path_name_w,
                get_result_size,
                get_result_date_modified,
                get_result_date_created,
                get_result_attributes,
            })
        }
    }

    unsafe fn connect_any_instance(&self) -> *mut EVERYTHING3_CLIENT {
        let client = (self.connect_w)(std::ptr::null());
        if !client.is_null() {
            return client;
        }
        if let Ok(inst_15a) = U16CString::from_str("1.5a") {
            let client_15 = (self.connect_w)(inst_15a.as_ptr());
            if !client_15.is_null() {
                return client_15;
            }
        }
        std::ptr::null_mut()
    }

    pub fn probe_connection_version(&self) -> Option<String> {
        unsafe {
            let client = self.connect_any_instance();
            if !client.is_null() {
                let maj = (self.get_major_version)(client);
                let min = (self.get_minor_version)(client);
                let rev = (self.get_revision)(client);
                let bld = (self.get_build_number)(client);
                (self.destroy_client)(client);
                return Some(format!("{}.{}.{}.{}", maj, min, rev, bld));
            }
        }
        Self::probe_legacy_ipc_window()
    }

    pub fn probe_legacy_ipc_window() -> Option<String> {
        let class_w = U16CString::from_str(EVERYTHING_IPC_WNDCLASSW).ok()?;
        unsafe {
            let hwnd = FindWindowW(class_w.as_ptr(), std::ptr::null());
            if hwnd.is_null() {
                return None;
            }
            let db_loaded = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_IS_DB_LOADED, 0);
            if db_loaded == 0 {
                return None;
            }
            let maj = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_MAJOR_VERSION, 0);
            let min = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_MINOR_VERSION, 0);
            let rev = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_REVISION, 0);
            let bld = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_BUILD_NUMBER, 0);
            Some(format!("{}.{}.{}.{}", maj, min, rev, bld))
        }
    }

    fn build_everything_search_syntax(params: &SearchQuery) -> String {
        let mut tokens = Vec::new();

        if let Some(ref scope) = params.scope_path {
            let trimmed = scope.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("search://") {
                tokens.push(format!("\"{}\"", trimmed));
            }
        }

        if let Some(ref exts) = params.extensions {
            let clean_exts: Vec<&str> = exts
                .split([';', ',', ' '])
                .map(|s| s.trim().trim_start_matches('.'))
                .filter(|s| !s.is_empty())
                .collect();
            if !clean_exts.is_empty() {
                tokens.push(format!("ext:{}", clean_exts.join(";")));
            }
        }

        if !params.use_regex {
            match (params.min_size, params.max_size) {
                (Some(min), Some(max)) if max >= min => {
                    tokens.push(format!("size:{}..{}", min, max));
                }
                (Some(min), None) => {
                    tokens.push(format!("size:>={}", min));
                }
                (None, Some(max)) => {
                    tokens.push(format!("size:<={}", max));
                }
                _ => {}
            }
        }

        if !params.query.trim().is_empty() {
            tokens.push(params.query.trim().to_string());
        }

        tokens.join(" ")
    }

    pub fn search(&self, params: &SearchQuery) -> Result<Vec<VfsEntry>, String> {
        unsafe {
            let client = self.connect_any_instance();
            if client.is_null() {
                let err = (self.get_last_error)();
                return Err(format!("Everything3_ConnectW fallito (codice: {:#X})", err));
            }

            if (self.is_db_loaded)(client) == 0 {
                (self.destroy_client)(client);
                return Err("Database Everything non ancora caricato".to_string());
            }

            let state = (self.create_search_state)();
            if state.is_null() {
                (self.destroy_client)(client);
                return Err("Impossibile allocare EVERYTHING3_SEARCH_STATE".to_string());
            }

            let formatted_query = if params.use_regex {
                params.query.clone()
            } else {
                Self::build_everything_search_syntax(params)
            };

            let wide_query = U16CString::from_str(&formatted_query)
                .map_err(|_| "Query contiene caratteri nulli non validi".to_string())?;

            (self.set_search_text_w)(state, wide_query.as_ptr());
            (self.set_search_match_case)(state, if params.match_case { 1 } else { 0 });
            (self.set_search_regex)(state, if params.use_regex { 1 } else { 0 });
            (self.set_search_folders_first)(state, EVERYTHING3_SEARCH_FOLDERS_FIRST_ALWAYS);
            (self.set_search_viewport_count)(state, params.max_results.max(100));

            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_NAME);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_PATH);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_SIZE);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_EXTENSION);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_DATE_MODIFIED);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_DATE_CREATED);
            (self.add_search_property_request)(state, EVERYTHING3_PROPERTY_ID_ATTRIBUTES);

            let result_list = (self.search_fn)(client, state);
            if result_list.is_null() {
                let err = (self.get_last_error)();
                (self.destroy_search_state)(state);
                (self.destroy_client)(client);
                if err != EVERYTHING3_OK {
                    return Err(format!("Everything3_Search errore: {:#X}", err));
                }
                return Ok(Vec::new());
            }

            let count = (self.get_result_list_viewport_count)(result_list);
            let mut entries = Vec::with_capacity(count.min(params.max_results));
            let mut name_buf = vec![0u16; 1024];
            let mut full_buf = vec![0u16; 2048];

            for idx in 0..count {
                if entries.len() >= params.max_results {
                    break;
                }

                let is_dir = (self.is_folder_result)(result_list, idx) != 0;
                let name = Self::read_wide_string(|buf, cap| {
                    (self.get_result_name_w)(result_list, idx, buf, cap)
                }, &mut name_buf);

                let mut full_path = Self::read_wide_string(|buf, cap| {
                    (self.get_result_full_path_name_w)(result_list, idx, buf, cap)
                }, &mut full_buf);

                if full_path.is_empty() {
                    let parent = Self::read_wide_string(|buf, cap| {
                        (self.get_result_path_w)(result_list, idx, buf, cap)
                    }, &mut full_buf);
                    full_path = Path::new(&parent)
                        .join(&name)
                        .to_string_lossy()
                        .to_string();
                }

                if let Some(ref scope) = params.scope_path {
                    if params.use_regex && !scope.is_empty() && !scope.starts_with("search://") {
                        if !full_path
                            .to_lowercase()
                            .starts_with(&scope.to_lowercase())
                        {
                            continue;
                        }
                    }
                }

                let raw_size = (self.get_result_size)(result_list, idx);
                let size = if is_dir || raw_size == u64::MAX {
                    0
                } else {
                    raw_size
                };

                if let Some(min_s) = params.min_size {
                    if !is_dir && size < min_s {
                        continue;
                    }
                }
                if let Some(max_s) = params.max_size {
                    if !is_dir && size > max_s {
                        continue;
                    }
                }

                let mtime_raw = (self.get_result_date_modified)(result_list, idx);
                let ctime_raw = (self.get_result_date_created)(result_list, idx);
                let attributes = (self.get_result_attributes)(result_list, idx);

                let modified = filetime_to_unix_seconds(mtime_raw);
                let created = filetime_to_unix_seconds(ctime_raw);

                let path_obj = Path::new(&name);
                let (stem, extension) = if is_dir {
                    (name.clone(), String::new())
                } else {
                    let ext = path_obj
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let st = path_obj
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| name.clone());
                    (st, ext)
                };

                let is_archive = !is_dir && VfsManager::is_archive_file_path(&full_path);
                let kind = LocalFsProvider::describe_kind(is_dir, is_archive, &extension);

                entries.push(VfsEntry {
                    name,
                    stem,
                    extension,
                    path: full_path,
                    is_dir,
                    is_parent: false,
                    is_archive,
                    size,
                    modified,
                    created,
                    kind,
                    attributes,
                });
            }

            (self.destroy_result_list)(result_list);
            (self.destroy_search_state)(state);
            (self.destroy_client)(client);

            Ok(entries)
        }
    }

    fn read_wide_string<F>(mut fetch: F, buf: &mut [u16]) -> String
    where
        F: FnMut(*mut u16, usize) -> usize,
    {
        let written = fetch(buf.as_mut_ptr(), buf.len());
        if written == 0 {
            return String::new();
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(written.min(buf.len()));
        String::from_utf16_lossy(&buf[..len])
    }
}

#[allow(dead_code)]
fn _keep_c_void(_p: *const c_void) {}
