use super::local::LocalFsProvider;
use super::{DirectoryListing, VfsEntry, VfsManager, VfsProvider};
use libloading::os::windows::Library as WinLibrary;
use libloading::Library;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::c_void;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{ULONG, WORD};
use winapi::shared::ntdef::HRESULT;
use winapi::um::propidl::PROPVARIANT;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::LibraryLoader::{
    LoadLibraryExW, LOAD_WITH_ALTERED_SEARCH_PATH,
};

#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocStringLen(strIn: *const u16, ui: u32) -> *mut u16;
}

#[link(name = "ole32")]
extern "system" {
    fn PropVariantClear(pvar: *mut PROPVARIANT) -> HRESULT;
}

const S_OK: HRESULT = 0;
const E_NOINTERFACE: HRESULT = -2147467262i32; // 0x80004002
const E_ABORT: HRESULT = -2147467260i32; // 0x80004004
const E_FAIL: HRESULT = -2147467259i32; // 0x80004005

pub const IID_IUNKNOWN: GUID = GUID {
    Data1: 0x00000000,
    Data2: 0x0000,
    Data3: 0x0000,
    Data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub const IID_ISEQUENTIALINSTREAM: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00],
};

pub const IID_ISEQUENTIALOUTSTREAM: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x00],
};

pub const IID_IINSTREAM: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x00, 0x00],
};

pub const IID_IOUTSTREAM: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00],
};

pub const IID_ICRYPTOGETTEXTPASSWORD: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x05, 0x00, 0x10, 0x00, 0x00],
};

pub const IID_ICRYPTOGETTEXTPASSWORD2: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x05, 0x00, 0x11, 0x00, 0x00],
};

pub const IID_ISETPROPERTIES: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x06, 0x00, 0x03, 0x00, 0x00],
};

pub const IID_IINARCHIVE: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x06, 0x00, 0x60, 0x00, 0x00],
};

pub const IID_IOUTARCHIVE: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x06, 0x00, 0xA0, 0x00, 0x00],
};

pub const IID_IARCHIVEEXTRACTCALLBACK: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x06, 0x00, 0x20, 0x00, 0x00],
};

pub const IID_IARCHIVEUPDATECALLBACK: GUID = GUID {
    Data1: 0x23170F69,
    Data2: 0x40C1,
    Data3: 0x278A,
    Data4: [0x00, 0x00, 0x00, 0x06, 0x00, 0x80, 0x00, 0x00],
};

pub fn format_clsid(format_id: u8) -> GUID {
    GUID {
        Data1: 0x23170F69,
        Data2: 0x40C1,
        Data3: 0x278A,
        Data4: [0x10, 0x00, 0x00, 0x01, 0x10, format_id, 0x00, 0x00],
    }
}

pub const KP_ID_PATH: u32 = 3;
pub const KP_ID_IS_DIR: u32 = 6;
pub const KP_ID_SIZE: u32 = 7;
pub const KP_ID_ATTRIB: u32 = 9;
pub const KP_ID_MTIME: u32 = 12;

const VT_EMPTY: u16 = 0;
const VT_BSTR: u16 = 8;
const VT_BOOL: u16 = 11;
const VT_UI4: u16 = 19;
const VT_UI8: u16 = 21;
const VT_FILETIME: u16 = 64;

fn guid_eq(a: &GUID, b: &GUID) -> bool {
    a.Data1 == b.Data1 && a.Data2 == b.Data2 && a.Data3 == b.Data3 && a.Data4 == b.Data4
}

// ============================================================================
// STRUCT OPZIONI COMPRESSIONE, ESTRAZIONE E PROGRESSO REAL-TIME
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    pub sources: Vec<String>,
    pub destination_archive: String,
    pub format: String,
    pub compression_level: u32,
    pub method: String,
    pub dictionary_size: String,
    pub word_size: u32,
    pub solid_block_size: String,
    pub num_threads: u32,
    pub split_volume_size: Option<String>,
    pub extra_parameters: Option<String>,
    pub update_mode: String,
    pub path_mode: String,
    pub create_sfx: bool,
    pub compress_shared_files: bool,
    pub delete_after_compression: bool,
    pub password: Option<String>,
    pub encryption_method: String,
    pub encrypt_file_names: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOptions {
    pub archive_path: String,
    pub destination_dir: String,
    pub overwrite_mode: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub operation: String,
    pub archive_name: String,
    pub current_file: String,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub speed_mb_per_sec: f64,
    pub elapsed_secs: u64,
    pub eta_secs: u64,
    pub status: String,
    pub message: Option<String>,
}

pub struct ProgressReporter {
    app: AppHandle,
    operation: String,
    archive_name: String,
    total_bytes: AtomicU64,
    processed_bytes: AtomicU64,
    current_file: Mutex<String>,
    start_instant: Instant,
    last_emit: Mutex<Instant>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl ProgressReporter {
    pub fn new(
        app: AppHandle,
        operation: &str,
        archive_name: &str,
        initial_total: u64,
        cancel_flag: Arc<AtomicBool>,
    ) -> Arc<Self> {
        let now = Instant::now();
        Arc::new(Self {
            app,
            operation: operation.to_string(),
            archive_name: archive_name.to_string(),
            total_bytes: AtomicU64::new(initial_total.max(1)),
            processed_bytes: AtomicU64::new(0),
            current_file: Mutex::new(String::new()),
            start_instant: now,
            last_emit: Mutex::new(now),
            cancel_flag,
        })
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn set_total(&self, total: u64) {
        self.total_bytes.store(total.max(1), Ordering::Relaxed);
        self.emit_now("running", None, true);
    }

    pub fn set_current_file(&self, name: String) {
        *self.current_file.lock() = name;
        self.emit_now("running", None, false);
    }

    pub fn set_completed(&self, completed: u64) {
        self.processed_bytes.store(completed, Ordering::Relaxed);
        self.emit_now("running", None, false);
    }

    pub fn finish(&self, status: &str, message: Option<String>) {
        if status == "completed" {
            let tot = self.total_bytes.load(Ordering::Relaxed);
            self.processed_bytes.store(tot, Ordering::Relaxed);
        }
        self.emit_now(status, message, true);
    }

    fn emit_now(&self, status: &str, message: Option<String>, force: bool) {
        let now = Instant::now();
        {
            let mut last = self.last_emit.lock();
            if !force && now.duration_since(*last).as_millis() < 45 {
                return;
            }
            *last = now;
        }

        let tot = self.total_bytes.load(Ordering::Relaxed).max(1);
        let proc_b = self.processed_bytes.load(Ordering::Relaxed).min(tot);
        let pct = ((proc_b as f64 / tot as f64) * 100.0).clamp(0.0, 100.0);
        let elapsed_f = self.start_instant.elapsed().as_secs_f64().max(0.001);
        let speed_mb = (proc_b as f64 / (1024.0 * 1024.0)) / elapsed_f;
        let remaining_bytes = tot.saturating_sub(proc_b);
        let bytes_per_sec = (proc_b as f64 / elapsed_f).max(1.0);
        let eta_secs = (remaining_bytes as f64 / bytes_per_sec) as u64;

        let payload = ProgressPayload {
            operation: self.operation.clone(),
            archive_name: self.archive_name.clone(),
            current_file: self.current_file.lock().clone(),
            processed_bytes: proc_b,
            total_bytes: tot,
            percent: pct,
            speed_mb_per_sec: speed_mb,
            elapsed_secs: self.start_instant.elapsed().as_secs(),
            eta_secs,
            status: status.to_string(),
            message,
        };

        let _ = self.app.emit("archive-progress", &payload);
    }
}

// ============================================================================
// COM INTERFACES PER 7-ZIP (`IInArchive`, `IOutArchive`, `ISetProperties`)
// ============================================================================

#[repr(C)]
pub struct IInArchiveVtbl {
    pub query_interface: unsafe extern "system" fn(
        this: *mut IInArchive,
        riid: *const GUID,
        ppv_object: *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(this: *mut IInArchive) -> ULONG,
    pub release: unsafe extern "system" fn(this: *mut IInArchive) -> ULONG,
    pub open: unsafe extern "system" fn(
        this: *mut IInArchive,
        stream: *mut c_void,
        max_check_start_position: *const u64,
        open_archive_callback: *mut c_void,
    ) -> HRESULT,
    pub close: unsafe extern "system" fn(this: *mut IInArchive) -> HRESULT,
    pub get_number_of_items:
        unsafe extern "system" fn(this: *mut IInArchive, num_items: *mut u32) -> HRESULT,
    pub get_property: unsafe extern "system" fn(
        this: *mut IInArchive,
        index: u32,
        prop_id: u32,
        value: *mut PROPVARIANT,
    ) -> HRESULT,
    pub extract: unsafe extern "system" fn(
        this: *mut IInArchive,
        indices: *const u32,
        num_items: u32,
        test_mode: i32,
        extract_callback: *mut c_void,
    ) -> HRESULT,
    pub get_archive_property: unsafe extern "system" fn(
        this: *mut IInArchive,
        prop_id: u32,
        value: *mut PROPVARIANT,
    ) -> HRESULT,
    pub get_number_of_properties:
        unsafe extern "system" fn(this: *mut IInArchive, num_props: *mut u32) -> HRESULT,
    pub get_property_info: unsafe extern "system" fn(
        this: *mut IInArchive,
        index: u32,
        name: *mut *mut u16,
        prop_id: *mut u32,
        var_type: *mut WORD,
    ) -> HRESULT,
    pub get_number_of_archive_properties:
        unsafe extern "system" fn(this: *mut IInArchive, num_props: *mut u32) -> HRESULT,
    pub get_archive_property_info: unsafe extern "system" fn(
        this: *mut IInArchive,
        index: u32,
        name: *mut *mut u16,
        prop_id: *mut u32,
        var_type: *mut WORD,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IInArchive {
    pub lp_vtbl: *const IInArchiveVtbl,
}

#[repr(C)]
pub struct IOutArchiveVtbl {
    pub query_interface: unsafe extern "system" fn(
        this: *mut IOutArchive,
        riid: *const GUID,
        ppv_object: *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(this: *mut IOutArchive) -> ULONG,
    pub release: unsafe extern "system" fn(this: *mut IOutArchive) -> ULONG,
    pub update_items: unsafe extern "system" fn(
        this: *mut IOutArchive,
        out_stream: *mut c_void,
        num_items: u32,
        update_callback: *mut c_void,
    ) -> HRESULT,
    pub get_file_time_type:
        unsafe extern "system" fn(this: *mut IOutArchive, type_out: *mut u32) -> HRESULT,
}

#[repr(C)]
pub struct IOutArchive {
    pub lp_vtbl: *const IOutArchiveVtbl,
}

#[repr(C)]
pub struct ISetPropertiesVtbl {
    pub query_interface: unsafe extern "system" fn(
        this: *mut ISetProperties,
        riid: *const GUID,
        ppv_object: *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(this: *mut ISetProperties) -> ULONG,
    pub release: unsafe extern "system" fn(this: *mut ISetProperties) -> ULONG,
    pub set_properties: unsafe extern "system" fn(
        this: *mut ISetProperties,
        names: *const *const u16,
        values: *const PROPVARIANT,
        num_props: u32,
    ) -> HRESULT,
}

#[repr(C)]
pub struct ISetProperties {
    pub lp_vtbl: *const ISetPropertiesVtbl,
}

// ============================================================================
// TEAR-OFF COM PER `ICryptoGetTextPassword` & `ICryptoGetTextPassword2`
// ============================================================================

#[repr(C)]
struct ICryptoGetTextPasswordVtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComPasswordProvider,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComPasswordProvider) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComPasswordProvider) -> ULONG,
    crypto_get_text_password: unsafe extern "system" fn(
        this: *mut ComPasswordProvider,
        password: *mut *mut u16,
    ) -> HRESULT,
}

static PASSWORD_VTBL: ICryptoGetTextPasswordVtbl = ICryptoGetTextPasswordVtbl {
    query_interface: pwd_qi,
    add_ref: pwd_add_ref,
    release: pwd_release,
    crypto_get_text_password: pwd_get_text,
};

#[repr(C)]
struct ComPasswordProvider {
    vtbl: *const ICryptoGetTextPasswordVtbl,
    ref_count: AtomicU32,
    password: String,
}

impl ComPasswordProvider {
    fn new_boxed(password: String) -> *mut Self {
        Box::into_raw(Box::new(Self {
            vtbl: &PASSWORD_VTBL,
            ref_count: AtomicU32::new(1),
            password,
        }))
    }
}

unsafe extern "system" fn pwd_qi(
    this: *mut ComPasswordProvider,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN) || guid_eq(iid, &IID_ICRYPTOGETTEXTPASSWORD) {
        *ppv = this as *mut c_void;
        pwd_add_ref(this);
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn pwd_add_ref(this: *mut ComPasswordProvider) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn pwd_release(this: *mut ComPasswordProvider) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn pwd_get_text(
    this: *mut ComPasswordProvider,
    password: *mut *mut u16,
) -> HRESULT {
    if password.is_null() {
        return E_FAIL;
    }
    let this_ref = &*this;
    let wide: Vec<u16> = this_ref.password.encode_utf16().collect();
    *password = SysAllocStringLen(wide.as_ptr(), wide.len() as u32);
    S_OK
}

#[repr(C)]
struct ICryptoGetTextPassword2Vtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComPasswordProvider2,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComPasswordProvider2) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComPasswordProvider2) -> ULONG,
    crypto_get_text_password2: unsafe extern "system" fn(
        this: *mut ComPasswordProvider2,
        password_is_defined: *mut i32,
        password: *mut *mut u16,
    ) -> HRESULT,
}

static PASSWORD2_VTBL: ICryptoGetTextPassword2Vtbl = ICryptoGetTextPassword2Vtbl {
    query_interface: pwd2_qi,
    add_ref: pwd2_add_ref,
    release: pwd2_release,
    crypto_get_text_password2: pwd2_get_text,
};

#[repr(C)]
struct ComPasswordProvider2 {
    vtbl: *const ICryptoGetTextPassword2Vtbl,
    ref_count: AtomicU32,
    password: Option<String>,
}

impl ComPasswordProvider2 {
    fn new_boxed(password: Option<String>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            vtbl: &PASSWORD2_VTBL,
            ref_count: AtomicU32::new(1),
            password,
        }))
    }
}

unsafe extern "system" fn pwd2_qi(
    this: *mut ComPasswordProvider2,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN) || guid_eq(iid, &IID_ICRYPTOGETTEXTPASSWORD2) {
        *ppv = this as *mut c_void;
        pwd2_add_ref(this);
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn pwd2_add_ref(this: *mut ComPasswordProvider2) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn pwd2_release(this: *mut ComPasswordProvider2) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn pwd2_get_text(
    this: *mut ComPasswordProvider2,
    password_is_defined: *mut i32,
    password: *mut *mut u16,
) -> HRESULT {
    if password_is_defined.is_null() || password.is_null() {
        return E_FAIL;
    }
    let this_ref = &*this;
    match this_ref.password.as_ref().filter(|s| !s.is_empty()) {
        Some(pwd) => {
            *password_is_defined = 1;
            let wide: Vec<u16> = pwd.encode_utf16().collect();
            *password = SysAllocStringLen(wide.as_ptr(), wide.len() as u32);
        }
        None => {
            *password_is_defined = 0;
            *password = SysAllocStringLen(null(), 0);
        }
    }
    S_OK
}

// ============================================================================
// IMPLEMENTAZIONE COM `IInStream` PER LETTURA FILE ARCHIVIO
// ============================================================================

#[repr(C)]
struct IInStreamVtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComFileInStream,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComFileInStream) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComFileInStream) -> ULONG,
    read: unsafe extern "system" fn(
        this: *mut ComFileInStream,
        data: *mut c_void,
        size: u32,
        processed_size: *mut u32,
    ) -> HRESULT,
    seek: unsafe extern "system" fn(
        this: *mut ComFileInStream,
        offset: i64,
        seek_origin: u32,
        new_position: *mut u64,
    ) -> HRESULT,
}

static FILE_IN_STREAM_VTBL: IInStreamVtbl = IInStreamVtbl {
    query_interface: file_in_stream_qi,
    add_ref: file_in_stream_add_ref,
    release: file_in_stream_release,
    read: file_in_stream_read,
    seek: file_in_stream_seek,
};

#[repr(C)]
pub struct ComFileInStream {
    vtbl: *const IInStreamVtbl,
    ref_count: AtomicU32,
    file: File,
}

impl ComFileInStream {
    pub fn new_boxed(path: &Path) -> Result<*mut Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("Impossibile aprire '{}': {}", path.display(), e))?;
        let boxed = Box::new(Self {
            vtbl: &FILE_IN_STREAM_VTBL,
            ref_count: AtomicU32::new(1),
            file,
        });
        Ok(Box::into_raw(boxed))
    }
}

unsafe extern "system" fn file_in_stream_qi(
    this: *mut ComFileInStream,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN)
        || guid_eq(iid, &IID_ISEQUENTIALINSTREAM)
        || guid_eq(iid, &IID_IINSTREAM)
    {
        *ppv = this as *mut c_void;
        file_in_stream_add_ref(this);
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn file_in_stream_add_ref(this: *mut ComFileInStream) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn file_in_stream_release(this: *mut ComFileInStream) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn file_in_stream_read(
    this: *mut ComFileInStream,
    data: *mut c_void,
    size: u32,
    processed_size: *mut u32,
) -> HRESULT {
    if !processed_size.is_null() {
        *processed_size = 0;
    }
    if size == 0 {
        return S_OK;
    }
    let buf = std::slice::from_raw_parts_mut(data as *mut u8, size as usize);
    let this_mut = &mut *this;
    match this_mut.file.read(buf) {
        Ok(n) => {
            if !processed_size.is_null() {
                *processed_size = n as u32;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

unsafe extern "system" fn file_in_stream_seek(
    this: *mut ComFileInStream,
    offset: i64,
    seek_origin: u32,
    new_position: *mut u64,
) -> HRESULT {
    let pos = match seek_origin {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return E_FAIL,
    };
    let this_mut = &mut *this;
    match this_mut.file.seek(pos) {
        Ok(new_pos) => {
            if !new_position.is_null() {
                *new_position = new_pos;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

// ============================================================================
// IMPLEMENTAZIONE COM `IOutStream` PER SCRITTURA FILE
// ============================================================================

#[repr(C)]
struct IOutStreamVtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComFileOutStream,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComFileOutStream) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComFileOutStream) -> ULONG,
    write: unsafe extern "system" fn(
        this: *mut ComFileOutStream,
        data: *const c_void,
        size: u32,
        processed_size: *mut u32,
    ) -> HRESULT,
    seek: unsafe extern "system" fn(
        this: *mut ComFileOutStream,
        offset: i64,
        seek_origin: u32,
        new_position: *mut u64,
    ) -> HRESULT,
    set_size: unsafe extern "system" fn(this: *mut ComFileOutStream, new_size: u64) -> HRESULT,
}

static FILE_OUT_STREAM_VTBL: IOutStreamVtbl = IOutStreamVtbl {
    query_interface: file_out_stream_qi,
    add_ref: file_out_stream_add_ref,
    release: file_out_stream_release,
    write: file_out_stream_write,
    seek: file_out_stream_seek,
    set_size: file_out_stream_set_size,
};

#[repr(C)]
pub struct ComFileOutStream {
    vtbl: *const IOutStreamVtbl,
    ref_count: AtomicU32,
    file: File,
}

impl ComFileOutStream {
    pub fn new_boxed(path: &Path) -> Result<*mut Self, String> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file = File::create(path)
            .map_err(|e| format!("Impossibile creare '{}': {}", path.display(), e))?;
        let boxed = Box::new(Self {
            vtbl: &FILE_OUT_STREAM_VTBL,
            ref_count: AtomicU32::new(1),
            file,
        });
        Ok(Box::into_raw(boxed))
    }
}

unsafe extern "system" fn file_out_stream_qi(
    this: *mut ComFileOutStream,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN)
        || guid_eq(iid, &IID_ISEQUENTIALOUTSTREAM)
        || guid_eq(iid, &IID_IOUTSTREAM)
    {
        *ppv = this as *mut c_void;
        file_out_stream_add_ref(this);
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn file_out_stream_add_ref(this: *mut ComFileOutStream) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn file_out_stream_release(this: *mut ComFileOutStream) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn file_out_stream_write(
    this: *mut ComFileOutStream,
    data: *const c_void,
    size: u32,
    processed_size: *mut u32,
) -> HRESULT {
    if !processed_size.is_null() {
        *processed_size = 0;
    }
    if size == 0 {
        return S_OK;
    }
    let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
    let this_mut = &mut *this;
    match this_mut.file.write(slice) {
        Ok(n) => {
            if !processed_size.is_null() {
                *processed_size = n as u32;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

unsafe extern "system" fn file_out_stream_seek(
    this: *mut ComFileOutStream,
    offset: i64,
    seek_origin: u32,
    new_position: *mut u64,
) -> HRESULT {
    let pos = match seek_origin {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return E_FAIL,
    };
    let this_mut = &mut *this;
    match this_mut.file.seek(pos) {
        Ok(p) => {
            if !new_position.is_null() {
                *new_position = p;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

unsafe extern "system" fn file_out_stream_set_size(
    this: *mut ComFileOutStream,
    new_size: u64,
) -> HRESULT {
    let this_mut = &mut *this;
    match this_mut.file.set_len(new_size) {
        Ok(_) => S_OK,
        Err(_) => E_FAIL,
    }
}

// ============================================================================
// IMPLEMENTAZIONE COM `IArchiveExtractCallback` PER ESTRAZIONE ARCHIVI
// ============================================================================

#[repr(C)]
struct IArchiveExtractCallbackVtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComExtractCallback,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComExtractCallback) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComExtractCallback) -> ULONG,
    set_total: unsafe extern "system" fn(this: *mut ComExtractCallback, total: u64) -> HRESULT,
    set_completed: unsafe extern "system" fn(
        this: *mut ComExtractCallback,
        complete_value: *const u64,
    ) -> HRESULT,
    get_stream: unsafe extern "system" fn(
        this: *mut ComExtractCallback,
        index: u32,
        out_stream: *mut *mut c_void,
        ask_extract_mode: i32,
    ) -> HRESULT,
    prepare_operation:
        unsafe extern "system" fn(this: *mut ComExtractCallback, ask_extract_mode: i32) -> HRESULT,
    set_operation_result:
        unsafe extern "system" fn(this: *mut ComExtractCallback, op_res: i32) -> HRESULT,
}

static EXTRACT_CALLBACK_VTBL: IArchiveExtractCallbackVtbl = IArchiveExtractCallbackVtbl {
    query_interface: extract_cb_qi,
    add_ref: extract_cb_add_ref,
    release: extract_cb_release,
    set_total: extract_cb_set_total,
    set_completed: extract_cb_set_completed,
    get_stream: extract_cb_get_stream,
    prepare_operation: extract_cb_prepare_op,
    set_operation_result: extract_cb_set_op_res,
};

#[repr(C)]
pub struct ComExtractCallback {
    vtbl: *const IArchiveExtractCallbackVtbl,
    ref_count: AtomicU32,
    in_archive: *mut IInArchive,
    dest_dir: PathBuf,
    overwrite_mode: String,
    password: Option<String>,
    reporter: Arc<ProgressReporter>,
}

impl ComExtractCallback {
    pub fn new_boxed(
        in_archive: *mut IInArchive,
        dest_dir: PathBuf,
        overwrite_mode: String,
        password: Option<String>,
        reporter: Arc<ProgressReporter>,
    ) -> *mut Self {
        Box::into_raw(Box::new(Self {
            vtbl: &EXTRACT_CALLBACK_VTBL,
            ref_count: AtomicU32::new(1),
            in_archive,
            dest_dir,
            overwrite_mode,
            password,
            reporter,
        }))
    }
}

unsafe extern "system" fn extract_cb_qi(
    this: *mut ComExtractCallback,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN) || guid_eq(iid, &IID_IARCHIVEEXTRACTCALLBACK) {
        *ppv = this as *mut c_void;
        extract_cb_add_ref(this);
        S_OK
    } else if guid_eq(iid, &IID_ICRYPTOGETTEXTPASSWORD) {
        let this_ref = &*this;
        let pwd = this_ref.password.clone().unwrap_or_default();
        *ppv = ComPasswordProvider::new_boxed(pwd) as *mut c_void;
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn extract_cb_add_ref(this: *mut ComExtractCallback) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn extract_cb_release(this: *mut ComExtractCallback) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn extract_cb_set_total(
    this: *mut ComExtractCallback,
    total: u64,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    this_ref.reporter.set_total(total);
    S_OK
}

unsafe extern "system" fn extract_cb_set_completed(
    this: *mut ComExtractCallback,
    complete: *const u64,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    if !complete.is_null() {
        this_ref.reporter.set_completed(*complete);
    }
    S_OK
}

unsafe extern "system" fn extract_cb_get_stream(
    this: *mut ComExtractCallback,
    index: u32,
    out_stream: *mut *mut c_void,
    ask_extract_mode: i32,
) -> HRESULT {
    if out_stream.is_null() {
        return E_FAIL;
    }
    *out_stream = null_mut();
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    if ask_extract_mode != 0 {
        return S_OK;
    }

    let archive = this_ref.in_archive;
    let rel_path = read_archive_item_string(archive, index, KP_ID_PATH).unwrap_or_default();
    let is_dir = read_archive_item_bool(archive, index, KP_ID_IS_DIR);

    this_ref.reporter.set_current_file(rel_path.clone());

    let sanitized = rel_path.replace('/', "\\").trim_start_matches('\\').to_string();
    let mut full_target = this_ref.dest_dir.join(sanitized);

    if is_dir {
        let _ = fs::create_dir_all(&full_target);
        return S_OK;
    }

    if full_target.exists() {
        match this_ref.overwrite_mode.as_str() {
            "skip" => return S_OK,
            "rename" => {
                full_target = auto_rename_path(full_target);
            }
            _ => {} // overwrite
        }
    }

    match ComFileOutStream::new_boxed(&full_target) {
        Ok(stream_ptr) => {
            *out_stream = stream_ptr as *mut c_void;
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

unsafe extern "system" fn extract_cb_prepare_op(
    this: *mut ComExtractCallback,
    _mode: i32,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        E_ABORT
    } else {
        S_OK
    }
}

unsafe extern "system" fn extract_cb_set_op_res(
    this: *mut ComExtractCallback,
    _res: i32,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        E_ABORT
    } else {
        S_OK
    }
}

// ============================================================================
// IMPLEMENTAZIONE COM `IArchiveUpdateCallback` PER COMPRESSIONE ARCHIVI
// ============================================================================

pub struct UpdateSourceItem {
    pub disk_path: PathBuf,
    pub archive_rel_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_filetime: u64,
    pub attributes: u32,
}

#[repr(C)]
struct IArchiveUpdateCallbackVtbl {
    query_interface: unsafe extern "system" fn(
        this: *mut ComUpdateCallback,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
    add_ref: unsafe extern "system" fn(this: *mut ComUpdateCallback) -> ULONG,
    release: unsafe extern "system" fn(this: *mut ComUpdateCallback) -> ULONG,
    set_total: unsafe extern "system" fn(this: *mut ComUpdateCallback, total: u64) -> HRESULT,
    set_completed: unsafe extern "system" fn(
        this: *mut ComUpdateCallback,
        complete_value: *const u64,
    ) -> HRESULT,
    get_update_item_info: unsafe extern "system" fn(
        this: *mut ComUpdateCallback,
        index: u32,
        new_data: *mut i32,
        new_props: *mut i32,
        index_in_archive: *mut u32,
    ) -> HRESULT,
    get_property: unsafe extern "system" fn(
        this: *mut ComUpdateCallback,
        index: u32,
        prop_id: u32,
        value: *mut PROPVARIANT,
    ) -> HRESULT,
    get_stream: unsafe extern "system" fn(
        this: *mut ComUpdateCallback,
        index: u32,
        in_stream: *mut *mut c_void,
    ) -> HRESULT,
    set_operation_result:
        unsafe extern "system" fn(this: *mut ComUpdateCallback, op_res: i32) -> HRESULT,
}

static UPDATE_CALLBACK_VTBL: IArchiveUpdateCallbackVtbl = IArchiveUpdateCallbackVtbl {
    query_interface: update_cb_qi,
    add_ref: update_cb_add_ref,
    release: update_cb_release,
    set_total: update_cb_set_total,
    set_completed: update_cb_set_completed,
    get_update_item_info: update_cb_get_item_info,
    get_property: update_cb_get_property,
    get_stream: update_cb_get_stream,
    set_operation_result: update_cb_set_op_res,
};

#[repr(C)]
pub struct ComUpdateCallback {
    vtbl: *const IArchiveUpdateCallbackVtbl,
    ref_count: AtomicU32,
    items: Vec<UpdateSourceItem>,
    password: Option<String>,
    reporter: Arc<ProgressReporter>,
}

impl ComUpdateCallback {
    pub fn new_boxed(
        items: Vec<UpdateSourceItem>,
        password: Option<String>,
        reporter: Arc<ProgressReporter>,
    ) -> *mut Self {
        Box::into_raw(Box::new(Self {
            vtbl: &UPDATE_CALLBACK_VTBL,
            ref_count: AtomicU32::new(1),
            items,
            password,
            reporter,
        }))
    }
}

unsafe extern "system" fn update_cb_qi(
    this: *mut ComUpdateCallback,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || riid.is_null() {
        return E_FAIL;
    }
    let iid = &*riid;
    if guid_eq(iid, &IID_IUNKNOWN) || guid_eq(iid, &IID_IARCHIVEUPDATECALLBACK) {
        *ppv = this as *mut c_void;
        update_cb_add_ref(this);
        S_OK
    } else if guid_eq(iid, &IID_ICRYPTOGETTEXTPASSWORD2) {
        let this_ref = &*this;
        *ppv = ComPasswordProvider2::new_boxed(this_ref.password.clone()) as *mut c_void;
        S_OK
    } else if guid_eq(iid, &IID_ICRYPTOGETTEXTPASSWORD) {
        let this_ref = &*this;
        let pwd = this_ref.password.clone().unwrap_or_default();
        *ppv = ComPasswordProvider::new_boxed(pwd) as *mut c_void;
        S_OK
    } else {
        *ppv = null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn update_cb_add_ref(this: *mut ComUpdateCallback) -> ULONG {
    let this_ref = &*this;
    this_ref.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn update_cb_release(this: *mut ComUpdateCallback) -> ULONG {
    let prev = {
        let this_ref = &*this;
        this_ref.ref_count.fetch_sub(1, Ordering::Release)
    };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this));
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn update_cb_set_total(
    this: *mut ComUpdateCallback,
    total: u64,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    this_ref.reporter.set_total(total);
    S_OK
}

unsafe extern "system" fn update_cb_set_completed(
    this: *mut ComUpdateCallback,
    complete: *const u64,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    if !complete.is_null() {
        this_ref.reporter.set_completed(*complete);
    }
    S_OK
}

unsafe extern "system" fn update_cb_get_item_info(
    this: *mut ComUpdateCallback,
    _index: u32,
    new_data: *mut i32,
    new_props: *mut i32,
    index_in_archive: *mut u32,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    if !new_data.is_null() {
        *new_data = 1;
    }
    if !new_props.is_null() {
        *new_props = 1;
    }
    if !index_in_archive.is_null() {
        *index_in_archive = u32::MAX;
    }
    S_OK
}

unsafe extern "system" fn update_cb_get_property(
    this: *mut ComUpdateCallback,
    index: u32,
    prop_id: u32,
    value: *mut PROPVARIANT,
) -> HRESULT {
    if value.is_null() {
        return E_FAIL;
    }
    std::ptr::write_bytes(value, 0, 1);

    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }

    let item = match this_ref.items.get(index as usize) {
        Some(it) => it,
        None => return E_FAIL,
    };

    let payload_ptr = (value as *mut u8).add(8);

    match prop_id {
        KP_ID_PATH => {
            let wide: Vec<u16> = item.archive_rel_path.encode_utf16().collect();
            let bstr = SysAllocStringLen(wide.as_ptr(), wide.len() as u32);
            (*value).vt = VT_BSTR;
            *(payload_ptr as *mut *mut u16) = bstr;
        }
        KP_ID_IS_DIR => {
            (*value).vt = VT_BOOL;
            *(payload_ptr as *mut i16) = if item.is_dir { -1 } else { 0 };
        }
        KP_ID_SIZE => {
            if !item.is_dir {
                (*value).vt = VT_UI8;
                *(payload_ptr as *mut u64) = item.size;
            }
        }
        KP_ID_ATTRIB => {
            (*value).vt = VT_UI4;
            *(payload_ptr as *mut u32) = item.attributes;
        }
        KP_ID_MTIME => {
            (*value).vt = VT_FILETIME;
            *(payload_ptr as *mut u64) = item.mtime_filetime;
        }
        _ => {
            (*value).vt = VT_EMPTY;
        }
    }
    S_OK
}

unsafe extern "system" fn update_cb_get_stream(
    this: *mut ComUpdateCallback,
    index: u32,
    in_stream: *mut *mut c_void,
) -> HRESULT {
    if in_stream.is_null() {
        return E_FAIL;
    }
    *in_stream = null_mut();
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        return E_ABORT;
    }
    let item = match this_ref.items.get(index as usize) {
        Some(it) => it,
        None => return E_FAIL,
    };
    this_ref
        .reporter
        .set_current_file(item.archive_rel_path.clone());
    if item.is_dir {
        return S_OK;
    }
    match ComFileInStream::new_boxed(&item.disk_path) {
        Ok(ptr) => {
            *in_stream = ptr as *mut c_void;
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

unsafe extern "system" fn update_cb_set_op_res(
    this: *mut ComUpdateCallback,
    _op_res: i32,
) -> HRESULT {
    let this_ref = &*this;
    if this_ref.reporter.is_cancelled() {
        E_ABORT
    } else {
        S_OK
    }
}

// ============================================================================
// CONFIGURAZIONE PROPRIETÀ COM `ISetProperties`
// ============================================================================

unsafe fn apply_compression_properties(
    out_archive: *mut IOutArchive,
    opts: &CompressOptions,
) {
    let mut set_props_ptr: *mut c_void = null_mut();
    let hr = ((*(*out_archive).lp_vtbl).query_interface)(
        out_archive,
        &IID_ISETPROPERTIES as *const GUID,
        &mut set_props_ptr,
    );
    if hr != S_OK || set_props_ptr.is_null() {
        return;
    }

    let set_props = set_props_ptr as *mut ISetProperties;
    let mut wide_names: Vec<Vec<u16>> = Vec::new();
    let mut prop_values: Vec<PROPVARIANT> = Vec::new();

    let mut add_u32_prop = |name: &str, val: u32| {
        wide_names.push(name.encode_utf16().chain(std::iter::once(0)).collect());
        let mut pv: PROPVARIANT = std::mem::zeroed();
        pv.vt = VT_UI4;
        let payload = (&mut pv as *mut PROPVARIANT as *mut u8).add(8);
        *(payload as *mut u32) = val;
        prop_values.push(pv);
    };

    let mut add_bool_prop = |name: &str, val: bool| {
        wide_names.push(name.encode_utf16().chain(std::iter::once(0)).collect());
        let mut pv: PROPVARIANT = std::mem::zeroed();
        pv.vt = VT_BOOL;
        let payload = (&mut pv as *mut PROPVARIANT as *mut u8).add(8);
        *(payload as *mut i16) = if val { -1 } else { 0 };
        prop_values.push(pv);
    };

    let mut add_bstr_prop = |name: &str, val: &str| {
        wide_names.push(name.encode_utf16().chain(std::iter::once(0)).collect());
        let mut pv: PROPVARIANT = std::mem::zeroed();
        pv.vt = VT_BSTR;
        let w: Vec<u16> = val.encode_utf16().collect();
        let bstr = SysAllocStringLen(w.as_ptr(), w.len() as u32);
        let payload = (&mut pv as *mut PROPVARIANT as *mut u8).add(8);
        *(payload as *mut *mut u16) = bstr;
        prop_values.push(pv);
    };

    add_u32_prop("x", opts.compression_level);
    if opts.num_threads > 0 {
        add_u32_prop("mt", opts.num_threads);
    }

    let fmt = opts.format.to_ascii_lowercase();
    if fmt == "7z" {
        if !opts.method.is_empty() && opts.compression_level > 0 {
            add_bstr_prop("0", &opts.method);
        }
        if !opts.dictionary_size.is_empty() && opts.compression_level > 0 {
            add_bstr_prop("d", &opts.dictionary_size);
        }
        if opts.word_size > 0 && opts.compression_level > 0 {
            add_u32_prop("fb", opts.word_size);
        }
        if !opts.solid_block_size.is_empty() {
            add_bstr_prop("s", &opts.solid_block_size);
        }
        if opts.password.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
            add_bool_prop("he", opts.encrypt_file_names);
        }
    } else if fmt == "zip" {
        if !opts.method.is_empty() {
            add_bstr_prop("m", &opts.method);
        }
        if opts.password.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && !opts.encryption_method.is_empty()
        {
            add_bstr_prop("em", &opts.encryption_method);
        }
    }

    let name_ptrs: Vec<*const u16> = wide_names.iter().map(|v| v.as_ptr()).collect();
    if !name_ptrs.is_empty() {
        let _ = ((*(*set_props).lp_vtbl).set_properties)(
            set_props,
            name_ptrs.as_ptr(),
            prop_values.as_ptr(),
            name_ptrs.len() as u32,
        );
    }

    for pv in &mut prop_values {
        let _ = PropVariantClear(pv);
    }
    ((*(*set_props).lp_vtbl).release)(set_props);
}

// ============================================================================
// LOADER MULTI-PERCORSO `KZip.Core.dll` CON `LoadLibraryExW` + DIAGNOSTICA
// ============================================================================

type CreateObjectFn = unsafe extern "system" fn(
    clsid: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT;

type GetNumberOfFormatsFn = unsafe extern "system" fn(num_formats: *mut u32) -> HRESULT;

pub struct KZipCoreLibrary {
    _core_lib: Library,
    _codec_libs: Vec<Library>,
    pub loaded_path: String,
    pub create_object: CreateObjectFn,
    pub get_number_of_formats: Option<GetNumberOfFormatsFn>,
}

impl KZipCoreLibrary {
    pub fn try_load_with_diagnostics() -> (Option<Arc<Self>>, Vec<String>) {
        let mut logs = Vec::new();
        let candidates = Self::candidate_dll_paths();

        for dll_path in candidates {
            let display = dll_path.display().to_string();
            if !dll_path.exists() {
                logs.push(format!("[NON TROVATO] {}", display));
                continue;
            }

            let wide_path: Vec<u16> = dll_path
                .as_os_str()
                .encode_wide_compat()
                .into_iter()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                let h_mod = LoadLibraryExW(
                    wide_path.as_ptr(),
                    null_mut(),
                    LOAD_WITH_ALTERED_SEARCH_PATH,
                );

                if h_mod.is_null() {
                    let err_code = GetLastError();
                    logs.push(format!(
                        "[ERRORE LoadLibraryExW] {} -> GetLastError() = 0x{:08X} (codice {})",
                        display, err_code, err_code
                    ));
                    continue;
                }

                let core_lib: Library = WinLibrary::from_raw(h_mod as _).into();

                let create_object = match core_lib.get::<CreateObjectFn>(b"CreateObject\0") {
                    Ok(sym) => *sym,
                    Err(e) => {
                        logs.push(format!(
                            "[ERRORE Simbolo CreateObject] {} -> {}",
                            display, e
                        ));
                        continue;
                    }
                };

                let get_number_of_formats = core_lib
                    .get::<GetNumberOfFormatsFn>(b"GetNumberOfFormats\0")
                    .ok()
                    .map(|s| *s);

                let mut codec_libs = Vec::new();
                if let Some(parent_dir) = dll_path.parent() {
                    let codecs_dir = parent_dir.join("Codecs");
                    if codecs_dir.is_dir() {
                        if let Ok(rd) = fs::read_dir(&codecs_dir) {
                            for entry in rd.flatten() {
                                let cp = entry.path();
                                if cp
                                    .extension()
                                    .map(|e| e.eq_ignore_ascii_case("dll"))
                                    .unwrap_or(false)
                                {
                                    let w_codec: Vec<u16> = cp
                                        .as_os_str()
                                        .encode_wide_compat()
                                        .into_iter()
                                        .chain(std::iter::once(0))
                                        .collect();
                                    let hc = LoadLibraryExW(
                                        w_codec.as_ptr(),
                                        null_mut(),
                                        LOAD_WITH_ALTERED_SEARCH_PATH,
                                    );
                                    if !hc.is_null() {
                                        logs.push(format!(
                                            "[CODEC CARICATO] {}",
                                            cp.display()
                                        ));
                                        codec_libs.push(WinLibrary::from_raw(hc as _).into());
                                    } else {
                                        let ce = GetLastError();
                                        logs.push(format!(
                                            "[ERRORE CODEC] {} -> GetLastError() = 0x{:08X}",
                                            cp.display(),
                                            ce
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        logs.push(format!(
                            "[INFO] Sottocartella Codecs non presente in: {}",
                            codecs_dir.display()
                        ));
                    }
                }

                logs.push(format!("[OK] KZip.Core.dll caricata da: {}", display));

                return (
                    Some(Arc::new(Self {
                        _core_lib: core_lib,
                        _codec_libs: codec_libs,
                        loaded_path: display,
                        create_object,
                        get_number_of_formats,
                    })),
                    logs,
                );
            }
        }

        (None, logs)
    }

    fn candidate_dll_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let dll_names = ["KZip.Core.dll", "NanaZip.Core.dll", "7z.dll"];

        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                for name in dll_names {
                    paths.push(exe_dir.join(name));
                    paths.push(exe_dir.join("binaries").join(name));
                    paths.push(exe_dir.join("bin").join(name));
                }
            }
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for name in dll_names {
            paths.push(manifest_dir.join("binaries").join(name));
            paths.push(manifest_dir.join("bin").join(name));
            paths.push(manifest_dir.join(name));
        }

        let mut seen = HashSet::new();
        paths
            .into_iter()
            .filter(|p| seen.insert(p.clone()))
            .collect()
    }

    pub fn query_supported_formats_count(&self) -> u32 {
        if let Some(func) = self.get_number_of_formats {
            let mut count: u32 = 0;
            unsafe {
                if func(&mut count) == 0 {
                    return count;
                }
            }
        }
        0
    }
}

trait OsStrWideCompat {
    fn encode_wide_compat(&self) -> Vec<u16>;
}

impl OsStrWideCompat for std::ffi::OsStr {
    fn encode_wide_compat(&self) -> Vec<u16> {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            self.encode_wide().collect()
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.to_string_lossy().encode_utf16().collect()
        }
    }
}

// ============================================================================
// ARCHIVE VFS PROVIDER + OPERAZIONI COMPRESSIONE / ESTRAZIONE CON PROGRESSO
// ============================================================================

pub struct ArchiveVfsProvider {
    core: Option<Arc<KZipCoreLibrary>>,
    pub diagnostics: Vec<String>,
}

impl ArchiveVfsProvider {
    pub fn new() -> Self {
        let (core, diagnostics) = KZipCoreLibrary::try_load_with_diagnostics();
        Self { core, diagnostics }
    }

    pub fn is_loaded(&self) -> bool {
        self.core.is_some()
    }

    pub fn loaded_path(&self) -> Option<String> {
        self.core.as_ref().map(|c| c.loaded_path.clone())
    }

    pub fn supported_formats(&self) -> u32 {
        self.core
            .as_ref()
            .map(|c| c.query_supported_formats_count())
            .unwrap_or(0)
    }

    fn parse_archive_uri(uri: &str) -> Result<(String, String), String> {
        let without_scheme = uri
            .strip_prefix("archive://")
            .ok_or_else(|| "URI archivio non valido".to_string())?;
        if let Some((archive_file, inner)) = without_scheme.split_once("!/") {
            Ok((archive_file.to_string(), inner.to_string()))
        } else {
            Ok((without_scheme.to_string(), String::new()))
        }
    }

    pub fn detect_format_id(archive_path: &str) -> u8 {
        let lower = archive_path.to_ascii_lowercase();
        if lower.ends_with(".7z") {
            0x07
        } else if lower.ends_with(".zip") {
            0x01
        } else if lower.ends_with(".rar") {
            0x03
        } else if lower.ends_with(".tar") {
            0xEE
        } else if lower.ends_with(".gz") || lower.ends_with(".tgz") || lower.ends_with(".gzip") {
            0xEF
        } else if lower.ends_with(".bz2") || lower.ends_with(".bzip2") {
            0x02
        } else if lower.ends_with(".xz") {
            0x0C
        } else if lower.ends_with(".iso") {
            0xE7
        } else if lower.ends_with(".wim") {
            0xE6
        } else if lower.ends_with(".cab") {
            0x08
        } else {
            0x07
        }
    }

    unsafe fn open_in_archive(
        core: &KZipCoreLibrary,
        archive_path: &Path,
    ) -> Result<(*mut IInArchive, *mut ComFileInStream), String> {
        let primary_fmt = Self::detect_format_id(&archive_path.to_string_lossy());
        let candidates = [
            primary_fmt,
            0x07,
            0x01,
            0x03,
            0xCC,
            0xEE,
            0xEF,
            0x0C,
            0x02,
            0xE7,
            0xE6,
            0x08,
        ];

        for &fmt_id in &candidates {
            let clsid = format_clsid(fmt_id);
            let mut archive_ptr: *mut c_void = null_mut();
            let hr = (core.create_object)(
                &clsid as *const GUID,
                &IID_IINARCHIVE as *const GUID,
                &mut archive_ptr,
            );
            if hr != S_OK || archive_ptr.is_null() {
                continue;
            }

            let in_archive = archive_ptr as *mut IInArchive;
            let stream_ptr = match ComFileInStream::new_boxed(archive_path) {
                Ok(s) => s,
                Err(e) => {
                    ((*(*in_archive).lp_vtbl).release)(in_archive);
                    return Err(e);
                }
            };

            let max_check: u64 = 1 << 22;
            let open_hr = ((*(*in_archive).lp_vtbl).open)(
                in_archive,
                stream_ptr as *mut c_void,
                &max_check,
                null_mut(),
            );

            if open_hr == S_OK {
                return Ok((in_archive, stream_ptr));
            } else {
                ((*(*in_archive).lp_vtbl).close)(in_archive);
                ((*(*in_archive).lp_vtbl).release)(in_archive);
                file_in_stream_release(stream_ptr);
            }
        }

        Err(format!(
            "Impossibile aprire l'archivio '{}' tramite KZip.Core.dll COM IInArchive",
            archive_path.display()
        ))
    }

    pub fn extract_archive_with_options(
        &self,
        opts: &ExtractOptions,
        app: &AppHandle,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<usize, String> {
        let core = self.core.as_ref().ok_or_else(|| {
            format!(
                "KZip.Core.dll non caricata.\nDiagnostica:\n{}",
                self.diagnostics.join("\n")
            )
        })?;

        let archive_path = Path::new(&opts.archive_path);
        let dest_path = PathBuf::from(&opts.destination_dir);
        fs::create_dir_all(&dest_path)
            .map_err(|e| format!("Impossibile creare cartella destinazione: {}", e))?;

        let archive_label = archive_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| opts.archive_path.clone());

        unsafe {
            let (in_archive, stream_ptr) = Self::open_in_archive(core, archive_path)?;
            let mut num_items: u32 = 0;
            ((*(*in_archive).lp_vtbl).get_number_of_items)(in_archive, &mut num_items);

            let mut total_unpacked: u64 = 0;
            for idx in 0..num_items {
                total_unpacked = total_unpacked
                    .saturating_add(read_archive_item_u64(in_archive, idx, KP_ID_SIZE));
            }

            let reporter = ProgressReporter::new(
                app.clone(),
                "Estrazione",
                &archive_label,
                total_unpacked.max(1),
                cancel_flag,
            );

            let cb_ptr = ComExtractCallback::new_boxed(
                in_archive,
                dest_path,
                opts.overwrite_mode.clone(),
                opts.password.clone(),
                Arc::clone(&reporter),
            );

            let hr = ((*(*in_archive).lp_vtbl).extract)(
                in_archive,
                null(),
                u32::MAX,
                0,
                cb_ptr as *mut c_void,
            );

            extract_cb_release(cb_ptr);
            ((*(*in_archive).lp_vtbl).close)(in_archive);
            ((*(*in_archive).lp_vtbl).release)(in_archive);
            file_in_stream_release(stream_ptr);

            if hr == E_ABORT || reporter.is_cancelled() {
                reporter.finish("cancelled", Some("Estrazione annullata dall'utente".to_string()));
                return Err("Operazione di estrazione annullata.".to_string());
            }

            if hr != S_OK {
                let err_msg = format!("Estrazione COM fallita (HRESULT: {:#X})", hr);
                reporter.finish("error", Some(err_msg.clone()));
                return Err(err_msg);
            }

            reporter.finish("completed", None);
            Ok(num_items as usize)
        }
    }

    pub fn compress_items_with_options(
        &self,
        opts: &CompressOptions,
        app: &AppHandle,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<usize, String> {
        let core = self.core.as_ref().ok_or_else(|| {
            format!(
                "KZip.Core.dll non caricata.\nDiagnostica:\n{}",
                self.diagnostics.join("\n")
            )
        })?;

        let mut update_items = Vec::new();
        for src in &opts.sources {
            let p = PathBuf::from(src);
            if !p.exists() {
                continue;
            }
            let base_parent = p.parent().unwrap_or_else(|| Path::new(""));
            collect_update_items(&p, base_parent, &opts.path_mode, &mut update_items)?;
        }

        if update_items.is_empty() {
            return Err("Nessun file valido selezionato per la compressione".to_string());
        }

        let total_source_bytes: u64 = update_items.iter().map(|i| i.size).sum();
        let archive_label = Path::new(&opts.destination_archive)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| opts.destination_archive.clone());

        let reporter = ProgressReporter::new(
            app.clone(),
            "Compressione",
            &archive_label,
            total_source_bytes.max(1),
            cancel_flag,
        );

        let fmt_id = Self::detect_format_id(&opts.destination_archive);
        let clsid = format_clsid(fmt_id);

        unsafe {
            let mut out_arch_ptr: *mut c_void = null_mut();
            let hr = (core.create_object)(
                &clsid as *const GUID,
                &IID_IOUTARCHIVE as *const GUID,
                &mut out_arch_ptr,
            );
            if hr != S_OK || out_arch_ptr.is_null() {
                let msg = format!(
                    "Il formato selezionato non supporta la compressione (HRESULT: {:#X})",
                    hr
                );
                reporter.finish("error", Some(msg.clone()));
                return Err(msg);
            }

            let out_archive = out_arch_ptr as *mut IOutArchive;
            apply_compression_properties(out_archive, opts);

            let out_stream = match ComFileOutStream::new_boxed(Path::new(&opts.destination_archive))
            {
                Ok(s) => s,
                Err(e) => {
                    ((*(*out_archive).lp_vtbl).release)(out_archive);
                    reporter.finish("error", Some(e.clone()));
                    return Err(e);
                }
            };

            let total_count = update_items.len();
            let cb_ptr = ComUpdateCallback::new_boxed(
                update_items,
                opts.password.clone(),
                Arc::clone(&reporter),
            );

            let update_hr = ((*(*out_archive).lp_vtbl).update_items)(
                out_archive,
                out_stream as *mut c_void,
                total_count as u32,
                cb_ptr as *mut c_void,
            );

            update_cb_release(cb_ptr);
            ((*(*out_archive).lp_vtbl).release)(out_archive);
            file_out_stream_release(out_stream);

            if update_hr == E_ABORT || reporter.is_cancelled() {
                let _ = fs::remove_file(&opts.destination_archive);
                reporter.finish(
                    "cancelled",
                    Some("Compressione annullata dall'utente".to_string()),
                );
                return Err("Operazione di compressione annullata.".to_string());
            }

            if update_hr != S_OK {
                let msg = format!("Compressione COM fallita (HRESULT: {:#X})", update_hr);
                reporter.finish("error", Some(msg.clone()));
                return Err(msg);
            }

            if opts.delete_after_compression {
                for src in &opts.sources {
                    let p = Path::new(src);
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(p);
                    } else if p.is_file() {
                        let _ = fs::remove_file(p);
                    }
                }
            }

            reporter.finish("completed", None);
            Ok(total_count)
        }
    }
}

fn collect_update_items(
    current: &Path,
    base_root: &Path,
    path_mode: &str,
    out: &mut Vec<UpdateSourceItem>,
) -> Result<(), String> {
    let meta = fs::metadata(current).map_err(|e| e.to_string())?;
    let is_dir = meta.is_dir();

    let rel = match path_mode {
        "full" => current
            .to_string_lossy()
            .replace('\\', "/")
            .split_once(':')
            .map(|(_, rest)| rest.trim_start_matches('/').to_string())
            .unwrap_or_else(|| current.to_string_lossy().replace('\\', "/")),
        "absolute" => current.to_string_lossy().replace('\\', "/"),
        _ => current
            .strip_prefix(base_root)
            .unwrap_or(current)
            .to_string_lossy()
            .replace('\\', "/"),
    };

    let mtime_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mtime_filetime = unix_seconds_to_filetime(mtime_secs);

    #[cfg(target_os = "windows")]
    let attributes = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes()
    };
    #[cfg(not(target_os = "windows"))]
    let attributes = if is_dir { 0x10 } else { 0x20 };

    out.push(UpdateSourceItem {
        disk_path: current.to_path_buf(),
        archive_rel_path: rel,
        is_dir,
        size: if is_dir { 0 } else { meta.len() },
        mtime_filetime,
        attributes,
    });

    if is_dir {
        for entry in fs::read_dir(current).map_err(|e| e.to_string())?.flatten() {
            collect_update_items(&entry.path(), base_root, path_mode, out)?;
        }
    }
    Ok(())
}

fn auto_rename_path(target: PathBuf) -> PathBuf {
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

    for i in 1..1000 {
        let candidate = parent.join(format!("{}_{}{}", stem, i, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    target
}

impl VfsProvider for ArchiveVfsProvider {
    fn scheme(&self) -> &'static str {
        "archive"
    }

    fn list_directory(&self, path: &str) -> Result<DirectoryListing, String> {
        let (archive_file, inner_path) = Self::parse_archive_uri(path)?;
        let parent_dir = Path::new(&archive_file)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "C:\\".to_string());

        let clean_inner = inner_path.trim_matches('/').replace('\\', "/");

        let parent_uri = if clean_inner.is_empty() {
            parent_dir.clone()
        } else if let Some((up, _)) = clean_inner.rsplit_once('/') {
            format!("archive://{}!/{}", archive_file, up)
        } else {
            format!("archive://{}!/", archive_file)
        };

        let mut entries = vec![VfsEntry {
            name: "..".to_string(),
            stem: "..".to_string(),
            extension: String::new(),
            path: parent_uri.clone(),
            parent_dir: parent_uri.clone(),
            is_dir: true,
            is_parent: true,
            is_archive: false,
            size: 0,
            modified: 0,
            created: 0,
            kind: "folder".to_string(),
            attributes: 0x10,
        }];

        let mut total_files = 0usize;
        let mut total_dirs = 0usize;
        let mut total_bytes = 0u64;

        let core = match self.core {
            Some(ref c) => c,
            None => {
                return Err(format!(
                    "Impossibile aprire l'archivio: KZip.Core.dll non caricata.\n\nPercorsi tentati:\n{}",
                    self.diagnostics.join("\n")
                ));
            }
        };

        unsafe {
            let (in_archive, stream_ptr) =
                Self::open_in_archive(core, Path::new(&archive_file))?;

            let mut num_items: u32 = 0;
            ((*(*in_archive).lp_vtbl).get_number_of_items)(in_archive, &mut num_items);

            let prefix = if clean_inner.is_empty() {
                String::new()
            } else {
                format!("{}/", clean_inner)
            };

            let mut seen_synthetic_dirs = HashSet::new();
            let mut items = Vec::new();

            for idx in 0..num_items {
                let raw_item_path =
                    read_archive_item_string(in_archive, idx, KP_ID_PATH).unwrap_or_default();
                let norm_path = raw_item_path.replace('\\', "/").trim_matches('/').to_string();
                if norm_path.is_empty() {
                    continue;
                }

                if !prefix.is_empty() && !norm_path.starts_with(&prefix) {
                    continue;
                }

                let remainder = if prefix.is_empty() {
                    &norm_path[..]
                } else {
                    &norm_path[prefix.len()..]
                };

                if remainder.is_empty() {
                    continue;
                }

                if let Some((first_seg, _rest)) = remainder.split_once('/') {
                    if seen_synthetic_dirs.insert(first_seg.to_string()) {
                        let sub_uri = if clean_inner.is_empty() {
                            format!("archive://{}!/{}", archive_file, first_seg)
                        } else {
                            format!("archive://{}!/{}/{}", archive_file, clean_inner, first_seg)
                        };
                        total_dirs += 1;
                        items.push(VfsEntry {
                            name: first_seg.to_string(),
                            stem: first_seg.to_string(),
                            extension: String::new(),
                            path: sub_uri,
                            parent_dir: path.to_string(),
                            is_dir: true,
                            is_parent: false,
                            is_archive: false,
                            size: 0,
                            modified: 0,
                            created: 0,
                            kind: "folder".to_string(),
                            attributes: 0x10,
                        });
                    }
                } else {
                    let is_dir = read_archive_item_bool(in_archive, idx, KP_ID_IS_DIR);
                    let size = read_archive_item_u64(in_archive, idx, KP_ID_SIZE);
                    let mtime_ft = read_archive_item_u64(in_archive, idx, KP_ID_MTIME);
                    let modified = filetime_to_unix_seconds(mtime_ft);

                    if is_dir && !seen_synthetic_dirs.insert(remainder.to_string()) {
                        continue;
                    }

                    let name = remainder.to_string();
                    let (stem, extension) = if is_dir {
                        (name.clone(), String::new())
                    } else {
                        let p = Path::new(&name);
                        let ext = p
                            .extension()
                            .map(|e| e.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let st = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| name.clone());
                        (st, ext)
                    };

                    let entry_uri = if clean_inner.is_empty() {
                        format!("archive://{}!/{}", archive_file, name)
                    } else {
                        format!("archive://{}!/{}/{}", archive_file, clean_inner, name)
                    };

                    if is_dir {
                        total_dirs += 1;
                    } else {
                        total_files += 1;
                        total_bytes = total_bytes.saturating_add(size);
                    }

                    let is_archive = !is_dir && VfsManager::is_archive_file_path(&name);
                    let kind = LocalFsProvider::describe_kind(is_dir, is_archive, &extension);

                    items.push(VfsEntry {
                        name,
                        stem,
                        extension,
                        path: entry_uri,
                        parent_dir: path.to_string(),
                        is_dir,
                        is_parent: false,
                        is_archive,
                        size,
                        modified,
                        created: modified,
                        kind,
                        attributes: if is_dir { 0x10 } else { 0x20 },
                    });
                }
            }

            ((*(*in_archive).lp_vtbl).close)(in_archive);
            ((*(*in_archive).lp_vtbl).release)(in_archive);
            file_in_stream_release(stream_ptr);

            items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            entries.extend(items);
        }

        Ok(DirectoryListing {
            current_path: path.to_string(),
            parent_path: Some(parent_uri),
            provider_scheme: self.scheme().to_string(),
            entries,
            total_files,
            total_dirs,
            total_bytes,
        })
    }
}

unsafe fn read_archive_item_string(
    in_archive: *mut IInArchive,
    index: u32,
    prop_id: u32,
) -> Option<String> {
    let mut prop: PROPVARIANT = std::mem::zeroed();
    let hr = ((*(*in_archive).lp_vtbl).get_property)(in_archive, index, prop_id, &mut prop);
    if hr != S_OK {
        return None;
    }
    let result = if prop.vt == VT_BSTR {
        let payload_ptr = (&prop as *const PROPVARIANT as *const u8).add(8);
        let bstr_ptr = *(payload_ptr as *const *const u16);
        if !bstr_ptr.is_null() {
            let mut len = 0usize;
            while *bstr_ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(bstr_ptr, len);
            Some(String::from_utf16_lossy(slice))
        } else {
            None
        }
    } else {
        None
    };
    let _ = PropVariantClear(&mut prop);
    result
}

unsafe fn read_archive_item_bool(
    in_archive: *mut IInArchive,
    index: u32,
    prop_id: u32,
) -> bool {
    let mut prop: PROPVARIANT = std::mem::zeroed();
    let hr = ((*(*in_archive).lp_vtbl).get_property)(in_archive, index, prop_id, &mut prop);
    if hr != S_OK {
        return false;
    }
    let val = if prop.vt == VT_BOOL {
        let payload_ptr = (&prop as *const PROPVARIANT as *const u8).add(8);
        *(payload_ptr as *const i16) != 0
    } else {
        false
    };
    let _ = PropVariantClear(&mut prop);
    val
}

unsafe fn read_archive_item_u64(
    in_archive: *mut IInArchive,
    index: u32,
    prop_id: u32,
) -> u64 {
    let mut prop: PROPVARIANT = std::mem::zeroed();
    let hr = ((*(*in_archive).lp_vtbl).get_property)(in_archive, index, prop_id, &mut prop);
    if hr != S_OK {
        return 0;
    }
    let payload_ptr = (&prop as *const PROPVARIANT as *const u8).add(8);
    let val = match prop.vt {
        VT_UI8 | VT_FILETIME => *(payload_ptr as *const u64),
        VT_UI4 => *(payload_ptr as *const u32) as u64,
        _ => 0,
    };
    let _ = PropVariantClear(&mut prop);
    val
}

pub fn filetime_to_unix_seconds(ft_u64: u64) -> i64 {
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    if ft_u64 > EPOCH_DIFF {
        ((ft_u64 - EPOCH_DIFF) / 10_000_000) as i64
    } else {
        0
    }
}

pub fn unix_seconds_to_filetime(unix_secs: u64) -> u64 {
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    unix_secs.saturating_mul(10_000_000).saturating_add(EPOCH_DIFF)
}
