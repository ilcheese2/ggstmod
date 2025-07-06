use crate::cxxstd::{CxxString, CxxStringView};
use crate::memory::ThreadSafePtr;
use crate::output::budget_log;
use libc::{exit, memcpy};
use std::cell::LazyCell;
use std::ffi::c_void;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use widestring::{U16CString, U16String, WideStr, WideString};
use winapi::shared::minwindef::{FARPROC, HINSTANCE, HINSTANCE__};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::libloaderapi::{GetModuleHandleW, GetProcAddress};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::psapi::{GetModuleInformation, MODULEINFO};

#[repr(C)]
struct UE4SSProgram {}

type fn_get_program = unsafe extern "C" fn() -> *mut c_void;
type fn_get_working_directory = unsafe extern "C" fn(*mut c_void) -> *mut CxxString;

#[repr(C)]
pub struct FString {
    pub data: *mut u8,
    pub size: u32,
    pub capacity: u32,
}

type fn_FMemory_Malloc = unsafe extern "C" fn(u64, u32) -> *mut c_void;
pub type fn_FName_cstr = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut c_void);
pub type fn_FName_ToString = unsafe extern "C" fn(*mut FName, *mut FString);

pub struct ModCallback<T>(pub unsafe extern "C" fn(*mut CppUserModBase<T>));
unsafe extern "C" fn default_mod_callback<T>(_: *mut CppUserModBase<T>) {}

impl<T> Default for ModCallback<T> {
    fn default() -> Self {
        ModCallback(default_mod_callback)
    }
}


pub struct StringModCallback<T>(pub unsafe extern "C" fn(*mut CppUserModBase<T>, *const CxxStringView));
unsafe extern "C" fn default_string_mod_callback<T>(_: *mut CppUserModBase<T>, _: *const CxxStringView) {}

impl<T> Default for StringModCallback<T> {
    fn default() -> Self {
        StringModCallback(default_string_mod_callback)
    }
}

pub struct LuaModCallback(pub unsafe extern "C" fn()); // rust function pointer support is ass
unsafe extern "C" fn default_lua_callback() {}

impl Default for LuaModCallback {
    fn default() -> Self {
        LuaModCallback(default_lua_callback)
    }
}

fn lpcwstr(string: &str) -> *const u16 {
    let wstring = U16CString::from_str(string).unwrap();
    wstring.as_ptr()
}

fn get_handle(name: &str) -> HINSTANCE {
    let module_name: U16CString = U16CString::from_str(name).unwrap();
    let handle = unsafe { GetModuleHandleW(module_name.as_ptr()) };
    //let handle = unsafe { GetModuleHandleW(lpcwstr(name)) };
    if handle.is_null() {
        budget_log("failed");
        unsafe { budget_log(format!("{:?}", GetLastError()).as_str()) };
        //panic!("Failed to get handle for module: {}", name);
    }
    handle
}

pub static UE4SS: LazyLock<ThreadSafePtr<HINSTANCE__>> =
    LazyLock::new(|| ThreadSafePtr(get_handle("UE4SS.dll")));

pub static PROGRAM: LazyLock<ThreadSafePtr<c_void>> = LazyLock::new(|| {
    unsafe { budget_log(format!("{:?}", **UE4SS).as_str()) };
    let addr = unsafe {
        GetProcAddress(
            **UE4SS,
            b"?get_program@UE4SSProgram@RC@@SAAEAV12@XZ\0"
                .as_ptr()
                .cast(),
        )
    };
    if addr.is_null() {
        budget_log("failed to get program");
        unsafe { budget_log(format!("{:?}", GetLastError()).as_str()) };
    }
    budget_log(format!("{:p}", addr).as_str());
    ThreadSafePtr(unsafe { std::mem::transmute::<FARPROC, fn_get_program>(addr)() })
});

pub static CONFIG_PATH: LazyLock<String> = LazyLock::new(|| {
    let addr = unsafe {
        GetProcAddress(**UE4SS, (b"?get_working_directory@UE4SSProgram@RC@@QEAA?AV?$basic_string@_WU?$char_traits@_W@std@@V?$allocator@_W@2@@std@@XZ\0").as_ptr().cast())
    };
    if addr.is_null() {
        budget_log("failed to get working directory");
    }
    let a = (*PROGRAM).0;
    budget_log(format!("{:p}", a).as_str());
    //let ptr = unsafe { std::mem::transmute::<FARPROC, fn_get_working_directory>(addr)(a) };
    let ptr: *const CxxString = unsafe { std::mem::transmute(a.offset(624)) };
    if ptr.is_null() {
        budget_log("failed to get working directory pointer");
        unsafe {
            exit(0);
        }
    }
    budget_log(format!("{:p}", ptr).as_str());

    unsafe {
        Path::new(std::ptr::read_unaligned(ptr).string().as_str())
            .join("random_chara_config.toml")
            .into_os_string()
            .into_string()
            .unwrap()
    }
});

pub static FMalloc: LazyLock<fn_FMemory_Malloc> = LazyLock::new(|| {
    let addr = unsafe {
        GetProcAddress(
            **UE4SS,
            (b"?Malloc@FMemory@Unreal@RC@@SAPEAX_KI@Z\0")
                .as_ptr()
                .cast(),
        )
    };
    if addr.is_null() {
        budget_log("failed to get fmemory malloc");
    }
    unsafe { std::mem::transmute::<FARPROC, fn_FMemory_Malloc>(addr) }
});

static FName: LazyLock<fn_FName_cstr> = LazyLock::new(|| {
    let addr = unsafe {
        GetProcAddress(
            **UE4SS,
            (b"??0FName@Unreal@RC@@QEAA@PEB_WW4EFindName@12@PEAX@Z\0")
                .as_ptr()
                .cast(),
        )
    };
    if addr.is_null() {
        budget_log("failed to get fname construct with string");
    }
    unsafe { std::mem::transmute::<FARPROC, fn_FName_cstr>(addr) }
});

// FNAME_Add
pub unsafe fn create_fstring(string: &str) -> *mut FString {
    let ptr = (*FMalloc)((2 * string.len() + 1) as u64, 0);
    unsafe { memcpy(ptr, U16CString::from_str(string).unwrap().as_mut_ptr().cast(), 2 * (string.len() + 1)); }
    let fstring: *mut FString = (*FMalloc)(size_of::<FString>() as u64, 0).cast();
    (*fstring).data = ptr.cast();
    (*fstring).size = (string.len() + 1) as u32;
    (*fstring).capacity = align_to(string.len() + 1, 4) as u32; // idk if align is required
    fstring
}

pub unsafe fn fstring_to_string(fstring: &FString) -> String {
    let size = fstring.size as usize;
    if size == 0 {
        return String::new();
    }
    let data = (*fstring).data.cast::<u16>();
    U16String::from_ptr(data, size - 1).to_string().unwrap()
}

pub fn align_to(size: usize, alignment: usize) -> usize {
    if size % alignment == 0 {
        size
    } else {
        size + (alignment - (size % alignment))
    }
}

pub type FName = u64;

#[derive(Default)]
pub struct Vtable<T> {
    pub destructor: ModCallback<T>, // uhh
    pub on_update: ModCallback<T>,
    pub on_unreal_init: ModCallback<T>,
    pub on_ui_init: ModCallback<T>,
    pub on_program_start: ModCallback<T>,
    pub on_lua_start: LuaModCallback,
    pub on_lua_start2: LuaModCallback,
    pub on_lua_stop: LuaModCallback,
    pub on_lua_stop2: LuaModCallback,
    pub on_dll_load: StringModCallback<T>,
    pub render_tab: ModCallback<T>,
    pub padding: [u8; 0x8], // idk
}

#[repr(C)]
pub struct CppUserModBase<T> {
    pub vtable: *const Vtable<T>,
    pub padding: [u8; 0x18], //std::vector<std::shared_ptr<GUI::GUITab>> GUITabs{}
    pub mod_name: CxxString,
    pub mod_version: CxxString,
    pub mod_description: CxxString,
    pub mod_authors: CxxString,
    pub mod_intended_sdk_version: CxxString,
    pub data: T,
}

impl<T> Drop for CppUserModBase<T> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.vtable as *mut Vtable<T>);
        }
    }
}
