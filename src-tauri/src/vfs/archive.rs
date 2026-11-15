use super::{DirectoryListing, VfsEntry, VfsProvider};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{DWORD, ULONG, WORD};
use winapi::shared::ntdef::HRESULT;
use winapi::um::propidl::PROPVARIANT;

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
pub const KP_ID_MTIME: u32 = 12;

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

type CreateObjectFn = unsafe extern "system" fn(
    clsid: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT;

type GetNumberOfFormatsFn = unsafe extern "system" fn(num_formats: *mut u32) -> HRESULT;

pub struct KZipCoreLibrary {
    _core_lib: Library,
    _codecs_lib: Option<Library>,
    _zs_codecs_lib: Option<Library>,
    pub create_object: CreateObjectFn,
    pub get_number_of_formats: Option<GetNumberOfFormatsFn>,
}

impl KZipCoreLibrary {
    pub fn try_load() -> Option<Arc<Self>> {
        let candidate_dirs = Self::candidate_directories();

        for dir in candidate_dirs {
            let core_path = dir.join("KZip.Core.dll");
            if !core_path.exists() {
                continue;
            }

            unsafe {
                let core_lib = Library::new(&core_path).ok()?;
                let codecs_path = dir.join("Codecs").join("KZip.Codecs.dll");
                let zs_codecs_path = dir.join("Codecs").join("KZip.ZS.Codecs.dll");

                let codecs_lib = if codecs_path.exists() {
                    Library::new(codecs_path).ok()
                } else {
                    None
                };

                let zs_codecs_lib = if zs_codecs_path.exists() {
                    Library::new(zs_codecs_path).ok()
                } else {
                    None
                };

                let create_obj_sym: Symbol<CreateObjectFn> = core_lib.get(b"CreateObject\0").ok()?;
                let create_object = *create_obj_sym;

                let get_number_of_formats = core_lib
                    .get::<GetNumberOfFormatsFn>(b"GetNumberOfFormats\0")
                    .ok()
                    .map(|s| *s);

                return Some(Arc::new(Self {
                    _core_lib: core_lib,
                    _codecs_lib: codecs_lib,
                    _zs_codecs_lib: zs_codecs_lib,
                    create_object,
                    get_number_of_formats,
                }));
            }
        }
        None
    }

    fn candidate_directories() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                dirs.push(parent.to_path_buf());
                dirs.push(parent.join("binaries"));
                dirs.push(parent.join("bin"));
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

pub struct ArchiveVfsProvider {
    core: Option<Arc<KZipCoreLibrary>>,
}

impl ArchiveVfsProvider {
    pub fn new() -> Self {
        Self {
            core: KZipCoreLibrary::try_load(),
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.core.is_some()
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

    fn detect_format_id(archive_path: &str) -> u8 {
        let lower = archive_path.to_ascii_lowercase();
        if lower.ends_with(".7z") {
            0x07
        } else if lower.ends_with(".zip") {
            0x01
        } else if lower.ends_with(".rar") {
            0x03
        } else if lower.ends_with(".tar") {
            0xEE
        } else if lower.ends_with(".gz") || lower.ends_with(".tgz") {
            0xEF
        } else if lower.ends_with(".xz") {
            0x0C
        } else if lower.ends_with(".iso") {
            0xE7
        } else {
            0x07
        }
    }
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

        let parent_uri = if inner_path.trim_matches('/').is_empty() {
            parent_dir.clone()
        } else {
            let trimmed = inner_path.trim_matches('/');
            if let Some((up, _)) = trimmed.rsplit_once('/') {
                format!("archive://{}!/{}", archive_file, up)
            } else {
                format!("archive://{}!/", archive_file)
            }
        };

        let mut entries = vec![VfsEntry {
            name: "..".to_string(),
            stem: "..".to_string(),
            extension: String::new(),
            path: parent_uri.clone(),
            is_dir: true,
            is_parent: true,
            is_archive: false,
            size: 0,
            modified: 0,
            created: 0,
            kind: "folder".to_string(),
            attributes: 0x10,
        }];

        if let Some(ref core) = self.core {
            let format_id = Self::detect_format_id(&archive_file);
            let clsid = format_clsid(format_id);
            let mut archive_ptr: *mut c_void = std::ptr::null_mut();

            let hr = unsafe {
                (core.create_object)(
                    &clsid as *const GUID,
                    &IID_IINARCHIVE as *const GUID,
                    &mut archive_ptr,
                )
            };

            if hr == 0 && !archive_ptr.is_null() {
                let in_archive = archive_ptr as *mut IInArchive;
                unsafe {
                    ((*(*in_archive).lp_vtbl).release)(in_archive);
                }
            }
        } else {
            entries.push(VfsEntry {
                name: "[KZip.Core.dll non rilevata in binaries/ - Posizionare KZip.Core.dll per il mounting COM]".to_string(),
                stem: "KZip.Core.dll info".to_string(),
                extension: "info".to_string(),
                path: parent_dir,
                is_dir: false,
                is_parent: false,
                is_archive: false,
                size: 0,
                modified: 0,
                created: 0,
                kind: "document".to_string(),
                attributes: 0,
            });
        }

        Ok(DirectoryListing {
            current_path: path.to_string(),
            parent_path: Some(parent_uri),
            provider_scheme: self.scheme().to_string(),
            entries,
            total_files: 0,
            total_dirs: 0,
            total_bytes: 0,
        })
    }
}

pub fn filetime_to_unix_seconds(ft_u64: u64) -> i64 {
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    if ft_u64 > EPOCH_DIFF {
        ((ft_u64 - EPOCH_DIFF) / 10_000_000) as i64
    } else {
        0
    }
}

#[allow(dead_code)]
pub fn _suppress_unused_winapi(_d: DWORD) {}
