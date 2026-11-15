use super::SearchQuery;
use crate::vfs::archive::filetime_to_unix_seconds;
use crate::vfs::local::LocalFsProvider;
use crate::vfs::{VfsEntry, VfsManager};
use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_MORE_DATA, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE,
    HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Pipes::{
    SetNamedPipeHandleState, WaitNamedPipeW, PIPE_READMODE_MESSAGE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetWindowLongPtrW,
    RegisterClassW, SendMessageW, SetWindowLongPtrW, GWLP_USERDATA, HWND_MESSAGE, WM_COPYDATA,
    WM_USER, WNDCLASSW,
};

pub const EVERYTHING3_OK: u32 = 0;
pub const EVERYTHING3_ERROR_IPC_PIPE_NOT_FOUND: u32 = 0xE0000002;
pub const EVERYTHING3_ERROR_DISCONNECTED: u32 = 0xE0000003;
pub const EVERYTHING3_ERROR_BAD_RESPONSE: u32 = 0xE000000A;

const PIPE_CMD_GET_MAJOR_VERSION: u32 = 1;
const PIPE_CMD_GET_MINOR_VERSION: u32 = 2;
const PIPE_CMD_GET_REVISION: u32 = 3;
const PIPE_CMD_GET_BUILD_NUMBER: u32 = 4;
const PIPE_CMD_QUERY2_PACKET: u32 = 18;

pub const EVERYTHING_WM_IPC: u32 = WM_USER;
pub const EVERYTHING_IPC_GET_MAJOR_VERSION: usize = 0;
pub const EVERYTHING_IPC_GET_MINOR_VERSION: usize = 1;
pub const EVERYTHING_IPC_GET_REVISION: usize = 2;
pub const EVERYTHING_IPC_GET_BUILD_NUMBER: usize = 3;
pub const EVERYTHING_IPC_IS_DB_LOADED: usize = 401;

pub const EVERYTHING_IPC_MATCHCASE: u32 = 0x00000001;
pub const EVERYTHING_IPC_REGEX: u32 = 0x00000008;
pub const EVERYTHING_IPC_FOLDER: u32 = 0x00000001;
pub const EVERYTHING_IPC_ALLRESULTS: u32 = 0xFFFFFFFF;
pub const EVERYTHING_IPC_COPYDATA_QUERY2W: usize = 18;
pub const EVERYTHING_IPC_SORT_NAME_ASCENDING: u32 = 1;

pub const EVERYTHING_IPC_QUERY2_REQUEST_NAME: u32 = 0x00000001;
pub const EVERYTHING_IPC_QUERY2_REQUEST_PATH: u32 = 0x00000002;
pub const EVERYTHING_IPC_QUERY2_REQUEST_FULL_PATH_AND_NAME: u32 = 0x00000004;
pub const EVERYTHING_IPC_QUERY2_REQUEST_EXTENSION: u32 = 0x00000008;
pub const EVERYTHING_IPC_QUERY2_REQUEST_SIZE: u32 = 0x00000010;
pub const EVERYTHING_IPC_QUERY2_REQUEST_DATE_CREATED: u32 = 0x00000020;
pub const EVERYTHING_IPC_QUERY2_REQUEST_DATE_MODIFIED: u32 = 0x00000040;
pub const EVERYTHING_IPC_QUERY2_REQUEST_DATE_ACCESSED: u32 = 0x00000080;
pub const EVERYTHING_IPC_QUERY2_REQUEST_ATTRIBUTES: u32 = 0x00000100;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Everything3PipeHeader {
    pub command_or_status: u32,
    pub payload_size: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct EVERYTHING_IPC_ITEMW {
    pub flags: u32,
    pub filename_offset: u32,
    pub path_offset: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct EVERYTHING_IPC_LISTW_HEADER {
    pub totfolders: u32,
    pub totfiles: u32,
    pub totitems: u32,
    pub numfolders: u32,
    pub numfiles: u32,
    pub numitems: u32,
    pub offset: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct EVERYTHING_IPC_QUERY2_HEADER {
    pub reply_hwnd: u32,
    pub reply_copydata_message: u32,
    pub search_flags: u32,
    pub offset: u32,
    pub max_results: u32,
    pub request_flags: u32,
    pub sort_type: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct EVERYTHING_IPC_ITEM2 {
    pub flags: u32,
    pub data_offset: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct EVERYTHING_IPC_LIST2_HEADER {
    pub totitems: u32,
    pub numitems: u32,
    pub offset: u32,
    pub request_flags: u32,
    pub sort_type: u32,
}

const PIPE_CANDIDATES: &[&str] = &[
    r"\\.\pipe\Everything IPC (1.5a)",
    r"\\.\pipe\Everything IPC",
];

const IPC_WINDOW_CLASSES: &[&str] = &[
    "EVERYTHING_TASKBAR_NOTIFICATION_(1.5a)",
    "EVERYTHING_TASKBAR_NOTIFICATION",
];

const REPLY_COPYDATA_ID: usize = 0x4B5A_4950;

pub struct EverythingPipeConnection {
    handle: HANDLE,
}

impl EverythingPipeConnection {
    pub fn connect() -> Result<Self, u32> {
        for &pipe_path in PIPE_CANDIDATES {
            let wide_name = encode_wide_null(pipe_path);
            unsafe {
                let mut h_pipe = CreateFileW(
                    wide_name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    null_mut(),
                );

                if h_pipe == INVALID_HANDLE_VALUE {
                    let err = GetLastError();
                    if err == ERROR_PIPE_BUSY {
                        if WaitNamedPipeW(wide_name.as_ptr(), 350) != 0 {
                            h_pipe = CreateFileW(
                                wide_name.as_ptr(),
                                GENERIC_READ | GENERIC_WRITE,
                                0,
                                null(),
                                OPEN_EXISTING,
                                FILE_ATTRIBUTE_NORMAL,
                                null_mut(),
                            );
                        }
                    }
                }

                if h_pipe != INVALID_HANDLE_VALUE {
                    let mode: u32 = PIPE_READMODE_MESSAGE;
                    let _ = SetNamedPipeHandleState(h_pipe, &mode, null(), null());
                    return Ok(Self { handle: h_pipe });
                }
            }
        }
        Err(EVERYTHING3_ERROR_IPC_PIPE_NOT_FOUND)
    }

    pub fn write_all(&self, data: &[u8]) -> Result<(), u32> {
        let mut total_written = 0usize;
        while total_written < data.len() {
            let mut written: u32 = 0;
            let remaining = &data[total_written..];
            let ok = unsafe {
                WriteFile(
                    self.handle,
                    remaining.as_ptr(),
                    remaining.len() as u32,
                    &mut written,
                    null_mut(),
                )
            };
            if ok == 0 {
                return Err(EVERYTHING3_ERROR_DISCONNECTED);
            }
            total_written += written as usize;
        }
        Ok(())
    }

    pub fn read_exact(&self, buf: &mut [u8]) -> Result<(), u32> {
        let mut total_read = 0usize;
        while total_read < buf.len() {
            let mut bytes_read: u32 = 0;
            let slice = &mut buf[total_read..];
            let ok = unsafe {
                ReadFile(
                    self.handle,
                    slice.as_mut_ptr(),
                    slice.len() as u32,
                    &mut bytes_read,
                    null_mut(),
                )
            };
            if ok == 0 {
                let err = unsafe { GetLastError() };
                if err != ERROR_MORE_DATA {
                    return Err(EVERYTHING3_ERROR_DISCONNECTED);
                }
            }
            if bytes_read == 0 {
                return Err(EVERYTHING3_ERROR_DISCONNECTED);
            }
            total_read += bytes_read as usize;
        }
        Ok(())
    }

    pub fn transact_packet(&self, cmd: u32, payload: &[u8]) -> Result<Vec<u8>, u32> {
        let req_header = Everything3PipeHeader {
            command_or_status: cmd,
            payload_size: payload.len() as u32,
        };
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &req_header as *const Everything3PipeHeader as *const u8,
                size_of::<Everything3PipeHeader>(),
            )
        };

        self.write_all(header_bytes)?;
        if !payload.is_empty() {
            self.write_all(payload)?;
        }

        let mut resp_header_buf = [0u8; size_of::<Everything3PipeHeader>()];
        self.read_exact(&mut resp_header_buf)?;

        let resp_header: Everything3PipeHeader =
            unsafe { std::ptr::read_unaligned(resp_header_buf.as_ptr() as *const _) };

        if resp_header.command_or_status != EVERYTHING3_OK {
            return Err(resp_header.command_or_status);
        }

        let mut resp_payload = vec![0u8; resp_header.payload_size as usize];
        if !resp_payload.is_empty() {
            self.read_exact(&mut resp_payload)?;
        }
        Ok(resp_payload)
    }

    pub fn query_u32(&self, cmd: u32) -> Result<u32, u32> {
        let bytes = self.transact_packet(cmd, &[])?;
        if bytes.len() >= 4 {
            Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        } else {
            Err(EVERYTHING3_ERROR_BAD_RESPONSE)
        }
    }
}

impl Drop for EverythingPipeConnection {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE && !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

pub fn serialize_query2_packet(
    reply_hwnd: u32,
    reply_msg: u32,
    search_text: &str,
    match_case: bool,
    use_regex: bool,
    max_results: u32,
) -> Vec<u8> {
    let mut search_flags = 0u32;
    if match_case {
        search_flags |= EVERYTHING_IPC_MATCHCASE;
    }
    if use_regex {
        search_flags |= EVERYTHING_IPC_REGEX;
    }

    let request_flags = EVERYTHING_IPC_QUERY2_REQUEST_NAME
        | EVERYTHING_IPC_QUERY2_REQUEST_PATH
        | EVERYTHING_IPC_QUERY2_REQUEST_SIZE
        | EVERYTHING_IPC_QUERY2_REQUEST_DATE_CREATED
        | EVERYTHING_IPC_QUERY2_REQUEST_DATE_MODIFIED
        | EVERYTHING_IPC_QUERY2_REQUEST_ATTRIBUTES;

    let header = EVERYTHING_IPC_QUERY2_HEADER {
        reply_hwnd,
        reply_copydata_message: reply_msg,
        search_flags,
        offset: 0,
        max_results: if max_results == 0 {
            EVERYTHING_IPC_ALLRESULTS
        } else {
            max_results
        },
        request_flags,
        sort_type: EVERYTHING_IPC_SORT_NAME_ASCENDING,
    };

    let wide_search = encode_wide_null(search_text);
    let header_size = size_of::<EVERYTHING_IPC_QUERY2_HEADER>();
    let string_byte_len = wide_search.len() * size_of::<u16>();

    let mut packet = Vec::with_capacity(header_size + string_byte_len);
    unsafe {
        let header_slice =
            std::slice::from_raw_parts(&header as *const _ as *const u8, header_size);
        packet.extend_from_slice(header_slice);
        let str_slice =
            std::slice::from_raw_parts(wide_search.as_ptr() as *const u8, string_byte_len);
        packet.extend_from_slice(str_slice);
    }
    packet
}

pub fn deserialize_list2_buffer(
    buffer: &[u8],
    params: &SearchQuery,
) -> Result<Vec<VfsEntry>, String> {
    let header_size = size_of::<EVERYTHING_IPC_LIST2_HEADER>();
    if buffer.len() < header_size {
        return Err("Pacchetto EVERYTHING_IPC_LIST2 troppo corto".to_string());
    }

    let list_header: EVERYTHING_IPC_LIST2_HEADER =
        unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const _) };

    let num_items = list_header.numitems as usize;
    let req_flags = list_header.request_flags;
    let item_size = size_of::<EVERYTHING_IPC_ITEM2>();
    let items_table_end = header_size.saturating_add(num_items.saturating_mul(item_size));

    if buffer.len() < items_table_end {
        return Err("Tabella EVERYTHING_IPC_ITEM2 troncata".to_string());
    }

    let mut results = Vec::with_capacity(num_items.min(params.max_results));

    for i in 0..num_items {
        if results.len() >= params.max_results {
            break;
        }

        let item_pos = header_size + i * item_size;
        let item: EVERYTHING_IPC_ITEM2 =
            unsafe { std::ptr::read_unaligned(buffer[item_pos..].as_ptr() as *const _) };

        let is_dir = (item.flags & EVERYTHING_IPC_FOLDER) != 0;
        let mut cursor = item.data_offset as usize;
        if cursor >= buffer.len() {
            continue;
        }

        let mut name = String::new();
        let mut path_part = String::new();
        let mut full_path = String::new();
        let mut size = 0u64;
        let mut date_created = 0i64;
        let mut date_modified = 0i64;
        let mut attributes = 0u32;

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_NAME) != 0 {
            if let Some((val, next)) = read_ipc_length_prefixed_wstring(buffer, cursor) {
                name = val;
                cursor = next;
            }
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_PATH) != 0 {
            if let Some((val, next)) = read_ipc_length_prefixed_wstring(buffer, cursor) {
                path_part = val;
                cursor = next;
            }
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_FULL_PATH_AND_NAME) != 0 {
            if let Some((val, next)) = read_ipc_length_prefixed_wstring(buffer, cursor) {
                full_path = val;
                cursor = next;
            }
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_SIZE) != 0 && cursor + 8 <= buffer.len() {
            let raw_sz = u64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
            size = if is_dir || raw_sz == u64::MAX { 0 } else { raw_sz };
            cursor += 8;
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_EXTENSION) != 0 {
            if let Some((_, next)) = read_ipc_length_prefixed_wstring(buffer, cursor) {
                cursor = next;
            }
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_DATE_CREATED) != 0
            && cursor + 8 <= buffer.len()
        {
            let raw_ft = u64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
            date_created = filetime_to_unix_seconds(raw_ft);
            cursor += 8;
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_DATE_MODIFIED) != 0
            && cursor + 8 <= buffer.len()
        {
            let raw_ft = u64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
            date_modified = filetime_to_unix_seconds(raw_ft);
            cursor += 8;
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_DATE_ACCESSED) != 0
            && cursor + 8 <= buffer.len()
        {
            cursor += 8;
        }

        if (req_flags & EVERYTHING_IPC_QUERY2_REQUEST_ATTRIBUTES) != 0
            && cursor + 4 <= buffer.len()
        {
            attributes = u32::from_le_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
        }

        if full_path.is_empty() {
            full_path = Path::new(&path_part)
                .join(&name)
                .to_string_lossy()
                .to_string();
        }

        if path_part.is_empty() {
            path_part = Path::new(&full_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
        }

        if !passes_post_filters(&full_path, is_dir, size, params) {
            continue;
        }

        let (stem, extension) = split_stem_and_ext(&name, is_dir);
        let is_archive = !is_dir && VfsManager::is_archive_file_path(&full_path);
        let kind = LocalFsProvider::describe_kind(is_dir, is_archive, &extension);

        results.push(VfsEntry {
            name,
            stem,
            extension,
            path: full_path,
            parent_dir: path_part,
            is_dir,
            is_parent: false,
            is_archive,
            size,
            modified: date_modified,
            created: if date_created > 0 {
                date_created
            } else {
                date_modified
            },
            kind,
            attributes,
        });
    }

    Ok(results)
}

pub fn deserialize_list1_buffer(
    buffer: &[u8],
    params: &SearchQuery,
) -> Result<Vec<VfsEntry>, String> {
    let header_size = size_of::<EVERYTHING_IPC_LISTW_HEADER>();
    if buffer.len() < header_size {
        return Err("Pacchetto EVERYTHING_IPC_LISTW troppo corto".to_string());
    }

    let list_header: EVERYTHING_IPC_LISTW_HEADER =
        unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const _) };

    let num_items = list_header.numitems as usize;
    let item_size = size_of::<EVERYTHING_IPC_ITEMW>();
    let items_end = header_size.saturating_add(num_items.saturating_mul(item_size));

    if buffer.len() < items_end {
        return Err("Tabella EVERYTHING_IPC_ITEMW troncata".to_string());
    }

    let mut results = Vec::with_capacity(num_items.min(params.max_results));

    for i in 0..num_items {
        if results.len() >= params.max_results {
            break;
        }
        let pos = header_size + i * item_size;
        let item: EVERYTHING_IPC_ITEMW =
            unsafe { std::ptr::read_unaligned(buffer[pos..].as_ptr() as *const _) };

        let is_dir = (item.flags & EVERYTHING_IPC_FOLDER) != 0;
        let name = read_null_terminated_wstring(buffer, item.filename_offset as usize);
        let path_part = read_null_terminated_wstring(buffer, item.path_offset as usize);
        let full_path = Path::new(&path_part)
            .join(&name)
            .to_string_lossy()
            .to_string();

        if !passes_post_filters(&full_path, is_dir, 0, params) {
            continue;
        }

        let (stem, extension) = split_stem_and_ext(&name, is_dir);
        let is_archive = !is_dir && VfsManager::is_archive_file_path(&full_path);
        let kind = LocalFsProvider::describe_kind(is_dir, is_archive, &extension);

        results.push(VfsEntry {
            name,
            stem,
            extension,
            path: full_path,
            parent_dir: path_part,
            is_dir,
            is_parent: false,
            is_archive,
            size: 0,
            modified: 0,
            created: 0,
            kind,
            attributes: if is_dir { 0x10 } else { 0x20 },
        });
    }

    Ok(results)
}

fn read_ipc_length_prefixed_wstring(buffer: &[u8], offset: usize) -> Option<(String, usize)> {
    if offset + 4 > buffer.len() {
        return None;
    }
    let char_len = u32::from_le_bytes(buffer[offset..offset + 4].try_into().ok()?) as usize;
    let str_start = offset + 4;
    let byte_len = char_len.checked_mul(2)?;
    let str_end = str_start.checked_add(byte_len)?;
    let next_offset = str_end.checked_add(2)?;
    if next_offset > buffer.len() {
        return None;
    }

    let u16_slice: Vec<u16> = buffer[str_start..str_end]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();

    Some((String::from_utf16_lossy(&u16_slice), next_offset))
}

fn read_null_terminated_wstring(buffer: &[u8], offset: usize) -> String {
    if offset >= buffer.len() {
        return String::new();
    }
    let mut chars = Vec::new();
    let mut cur = offset;
    while cur + 2 <= buffer.len() {
        let ch = u16::from_le_bytes([buffer[cur], buffer[cur + 1]]);
        if ch == 0 {
            break;
        }
        chars.push(ch);
        cur += 2;
    }
    String::from_utf16_lossy(&chars)
}

pub struct EverythingPureRustClient;

impl EverythingPureRustClient {
    pub fn new() -> Self {
        Self
    }

    pub fn probe_status(&self) -> Option<String> {
        if let Ok(pipe) = EverythingPipeConnection::connect() {
            if let (Ok(maj), Ok(min), Ok(rev), Ok(bld)) = (
                pipe.query_u32(PIPE_CMD_GET_MAJOR_VERSION),
                pipe.query_u32(PIPE_CMD_GET_MINOR_VERSION),
                pipe.query_u32(PIPE_CMD_GET_REVISION),
                pipe.query_u32(PIPE_CMD_GET_BUILD_NUMBER),
            ) {
                return Some(format!("{}.{}.{}.{}", maj, min, rev, bld));
            }
            if let Some(v) = Self::probe_ipc_window_version() {
                return Some(v);
            }
            return Some("1.5 (Named Pipe Connected)".to_string());
        }

        Self::probe_ipc_window_version()
    }

    fn find_everything_ipc_hwnd() -> Option<HWND> {
        for &cls in IPC_WINDOW_CLASSES {
            let wide_cls = encode_wide_null(cls);
            let hwnd = unsafe { FindWindowW(wide_cls.as_ptr(), null()) };
            if !hwnd.is_null() {
                return Some(hwnd);
            }
        }
        None
    }

    pub fn probe_ipc_window_version() -> Option<String> {
        let hwnd = Self::find_everything_ipc_hwnd()?;
        unsafe {
            let loaded = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_IS_DB_LOADED, 0);
            if loaded == 0 {
                return None;
            }
            let maj = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_MAJOR_VERSION, 0);
            let min = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_MINOR_VERSION, 0);
            let rev = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_REVISION, 0);
            let bld = SendMessageW(hwnd, EVERYTHING_WM_IPC, EVERYTHING_IPC_GET_BUILD_NUMBER, 0);
            Some(format!("{}.{}.{}.{}", maj, min, rev, bld))
        }
    }

    pub fn build_search_expression(params: &SearchQuery) -> String {
        if params.use_regex {
            return params.query.clone();
        }

        let mut parts = Vec::new();

        if let Some(ref scope) = params.scope_path {
            let trimmed = scope.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("search://") {
                parts.push(format!("\"{}\"", trimmed));
            }
        }

        if let Some(ref exts) = params.extensions {
            let cleaned: Vec<&str> = exts
                .split([';', ',', ' '])
                .map(|s| s.trim().trim_start_matches('.'))
                .filter(|s| !s.is_empty())
                .collect();
            if !cleaned.is_empty() {
                parts.push(format!("ext:{}", cleaned.join(";")));
            }
        }

        match (params.min_size, params.max_size) {
            (Some(min), Some(max)) if max >= min => {
                parts.push(format!("size:{}..{}", min, max));
            }
            (Some(min), None) => {
                parts.push(format!("size:>={}", min));
            }
            (None, Some(max)) => {
                parts.push(format!("size:<={}", max));
            }
            _ => {}
        }

        if !params.query.trim().is_empty() {
            parts.push(params.query.trim().to_string());
        }

        parts.join(" ")
    }

    pub fn search(&self, params: &SearchQuery) -> Result<Vec<VfsEntry>, String> {
        let search_expr = Self::build_search_expression(params);
        let max_res = params.max_results.max(100) as u32;

        if let Ok(pipe) = EverythingPipeConnection::connect() {
            let query2_bytes = serialize_query2_packet(
                0,
                REPLY_COPYDATA_ID as u32,
                &search_expr,
                params.match_case,
                params.use_regex,
                max_res,
            );

            if let Ok(resp_buf) = pipe.transact_packet(PIPE_CMD_QUERY2_PACKET, &query2_bytes) {
                if let Ok(entries) = deserialize_list2_buffer(&resp_buf, params) {
                    return Ok(entries);
                }
            }
        }

        if let Some(ipc_hwnd) = Self::find_everything_ipc_hwnd() {
            return Self::execute_ipc_copydata_query2(ipc_hwnd, &search_expr, params);
        }

        Err("Everything 1.5 Named Pipe / IPC non disponibile".to_string())
    }

    fn execute_ipc_copydata_query2(
        everything_hwnd: HWND,
        search_expr: &str,
        params: &SearchQuery,
    ) -> Result<Vec<VfsEntry>, String> {
        unsafe {
            let class_name = encode_wide_null("KZipEverythingIpcReceiverClass");
            let h_instance = GetModuleHandleW(null());

            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(ipc_receiver_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: h_instance,
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: class_name.as_ptr(),
            };
            let _ = RegisterClassW(&wc);

            let mut received_buffer: Vec<u8> = Vec::new();
            let reply_hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                h_instance,
                null(),
            );

            if reply_hwnd.is_null() {
                return Err("Impossibile creare finestra message-only per IPC".to_string());
            }

            SetWindowLongPtrW(
                reply_hwnd,
                GWLP_USERDATA,
                &mut received_buffer as *mut Vec<u8> as isize,
            );

            let query_packet = serialize_query2_packet(
                reply_hwnd as usize as u32,
                REPLY_COPYDATA_ID as u32,
                search_expr,
                params.match_case,
                params.use_regex,
                params.max_results.max(100) as u32,
            );

            let cds = COPYDATASTRUCT {
                dwData: EVERYTHING_IPC_COPYDATA_QUERY2W,
                cbData: query_packet.len() as u32,
                lpData: query_packet.as_ptr() as *mut c_void,
            };

            let sent_ok = SendMessageW(
                everything_hwnd,
                WM_COPYDATA,
                reply_hwnd as WPARAM,
                &cds as *const COPYDATASTRUCT as LPARAM,
            );

            DestroyWindow(reply_hwnd);

            if sent_ok == 0 || received_buffer.is_empty() {
                return Err("Nessuna risposta da EVERYTHING_IPC_COPYDATA_QUERY2W".to_string());
            }

            deserialize_list2_buffer(&received_buffer, params)
                .or_else(|_| deserialize_list1_buffer(&received_buffer, params))
        }
    }
}

unsafe extern "system" fn ipc_receiver_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_COPYDATA && lparam != 0 {
        let cds = &*(lparam as *const COPYDATASTRUCT);
        if cds.dwData == REPLY_COPYDATA_ID && !cds.lpData.is_null() && cds.cbData > 0 {
            let user_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<u8>;
            if !user_ptr.is_null() {
                let slice =
                    std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize);
                (*user_ptr).clear();
                (*user_ptr).extend_from_slice(slice);
                return 1;
            }
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn passes_post_filters(full_path: &str, is_dir: bool, size: u64, params: &SearchQuery) -> bool {
    if let Some(ref scope) = params.scope_path {
        let trimmed = scope.trim();
        if params.use_regex && !trimmed.is_empty() && !trimmed.starts_with("search://") {
            if !full_path
                .to_lowercase()
                .starts_with(&trimmed.to_lowercase())
            {
                return false;
            }
        }
    }
    if !is_dir {
        if let Some(min_s) = params.min_size {
            if size < min_s {
                return false;
            }
        }
        if let Some(max_s) = params.max_size {
            if size > max_s {
                return false;
            }
        }
    }
    true
}

fn split_stem_and_ext(name: &str, is_dir: bool) -> (String, String) {
    if is_dir {
        return (name.to_string(), String::new());
    }
    let p = Path::new(name);
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    (stem, ext)
}

fn encode_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
