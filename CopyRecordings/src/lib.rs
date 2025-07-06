use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use gglibrary::cxxstd::CxxString;
use gglibrary::memory::{hook_function, signature_scan, signature_scan_from_addr, Hook, ThreadSafePtr};
use gglibrary::output::{budget_log, clear_log};
use gglibrary::red::{CMemorySlot, SSaveData};
use gglibrary::ue4ss::{create_fstring, fn_FName_ToString, fn_FName_cstr, fstring_to_string, CppUserModBase, FMalloc, FName, FString, ModCallback};
use libc::memcpy;
use minhook::MinHook;
use std::alloc::Layout;
use std::ffi::c_void;
use std::fmt::Debug;
use std::io::Read;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::str::FromStr;
use std::sync::{LazyLock, OnceLock};
use widestring::U16CString;


type fn_UREDWidgetRecordingSettings_NativeOnInitialized = unsafe extern "C" fn(*mut c_void);
type fn_UREDWidgetRecordingSettings_OnInputDecisionTrigger = unsafe extern "C" fn(*mut c_void);
type fn_UREDCommonSelectorWindowBase_AddItem = unsafe extern "C" fn(*mut c_void, FName) -> *mut c_void;
type fn_UREDWidgetBase_SetTextBlockTextByID = unsafe extern "C" fn(*mut c_void, *mut FName, *mut FString);
type fn_UREDCommonSelectorWindowBase_GetCursoredItem = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type fn_FWindowsPlatformApplicationMisc_ClipboardCopy = unsafe extern "C" fn(*mut u16);
type fn_FWindowsPlatformApplicationMisc_ClipboardPaste = unsafe extern "C" fn(*mut FString);

#[allow(non_snake_case)]
struct Accessors {
    UREDWidgetRecordingSettings_NativeOnInitialized: Hook<fn_UREDWidgetRecordingSettings_NativeOnInitialized>,
    UREDWidgetRecordingSettings_OnInputDecisionTrigger: Hook<fn_UREDWidgetRecordingSettings_OnInputDecisionTrigger>,
    UREDCommonSelectorWindowBase_AddItem: fn_UREDCommonSelectorWindowBase_AddItem,
    UREDWidgetBase_SetTextBlockTextByID: fn_UREDWidgetBase_SetTextBlockTextByID,
    UREDCommonSelectorWindowBase_GetCursoredItem: fn_UREDCommonSelectorWindowBase_GetCursoredItem,
    FWindowsPlatformApplicationMisc_ClipboardCopy: fn_FWindowsPlatformApplicationMisc_ClipboardCopy,
    FWindowsPlatformApplicationMisc_ClipboardPaste: fn_FWindowsPlatformApplicationMisc_ClipboardPaste,
    FName_cstr: fn_FName_cstr,
    FName_ToString: fn_FName_ToString,
    RED_SaveData: ThreadSafePtr<*mut c_void>,
}

type Data = PhantomData<Accessors>;
static HOOKS: OnceLock<Accessors> = OnceLock::new();

pub unsafe fn create_fname(string: &str) -> *mut FName {
    let ptr = (*FMalloc)(size_of::<FName>() as u64, 0);
    let hooks = HOOKS.get().unwrap();
    unsafe {
        (hooks.FName_cstr)(ptr, U16CString::from_str(string).unwrap().as_mut_ptr().cast(), 1, std::ptr::null_mut())
    }
    ptr.cast()
}

pub unsafe fn fname_to_string(fname: *mut FName) -> String {
    let hooks = HOOKS.get().unwrap();
    let fstring = Box::into_raw(Box::new(FString {
        data: std::ptr::null_mut(),
        size: 0,
        capacity: 0,
    }));
    (hooks.FName_ToString)(fname, fstring);
    let fstring = *Box::from_raw(fstring);
    fstring_to_string(&fstring)
}


const COPY_BUTTON: &str = "Copy recordings to clipboard";
const LOAD_BUTTON: &str = "Load recordings from clipboard";

pub unsafe extern "C" fn recording_settings_on_init(this: *mut c_void) { // TODO: use translation strings
    let hooks = HOOKS.get().unwrap();
    (hooks.UREDWidgetRecordingSettings_NativeOnInitialized.orig)(this);

    let item = (hooks.UREDCommonSelectorWindowBase_AddItem)(this, *create_fname(COPY_BUTTON));
    (hooks.UREDWidgetBase_SetTextBlockTextByID)(item, create_fname("Text"), create_fstring(COPY_BUTTON));

    let item = (hooks.UREDCommonSelectorWindowBase_AddItem)(this, *create_fname(LOAD_BUTTON));
    (hooks.UREDWidgetBase_SetTextBlockTextByID)(item, create_fname("Text"), create_fstring(LOAD_BUTTON));
}

pub unsafe extern "C" fn recording_settings_on_input(this: *mut c_void) {
    let hooks = HOOKS.get().unwrap();
    budget_log("recording_settings_on_input");
    (hooks.UREDWidgetRecordingSettings_OnInputDecisionTrigger.orig)(this);
    let item = (hooks.UREDCommonSelectorWindowBase_GetCursoredItem)(this);
    if item.is_null() {
        budget_log("item is null");
        return;
    }

    let name: *mut FName = (item as *mut u8).offset(0x18).cast();
    let name = fname_to_string(name);

    if name == COPY_BUTTON {
        let mut e = ZlibEncoder::new(std::slice::from_raw_parts::<u8>(&(***SAVE_DATA).memory_slot_blob as *const [CMemorySlot; 8] as *const u8, size_of::<CMemorySlot>() * 8), Compression::best());
        let mut buffer = Vec::new();
        e.read_to_end(&mut buffer).unwrap();
        (hooks.FWindowsPlatformApplicationMisc_ClipboardCopy)(U16CString::from_str(BASE64_STANDARD.encode(buffer)).unwrap().into_raw());
    }
    else if name == LOAD_BUTTON {
        let fstring = create_fstring("");
        (hooks.FWindowsPlatformApplicationMisc_ClipboardPaste)(fstring);
        let data = BASE64_STANDARD.decode(fstring_to_string(fstring.as_ref().unwrap())).unwrap();
        let mut d = ZlibDecoder::new(data.as_slice());
        let mut buffer = Vec::new();
        d.read_to_end(&mut buffer).unwrap();
        memcpy((&(***SAVE_DATA).memory_slot_blob) as *const [CMemorySlot; 8] as *mut c_void, buffer.as_ptr().cast(), size_of::<CMemorySlot>() * 8);
    }
}


static SAVE_DATA: LazyLock<ThreadSafePtr<SSaveData>> = LazyLock::new(|| {
    let hooks = HOOKS.get().unwrap();
    unsafe { ThreadSafePtr(((*(hooks.RED_SaveData.0)) as *mut u8).offset(0x1e0).cast()) } // idk why this offset is here
});


pub unsafe extern "C" fn on_unreal_init(this: *mut CppUserModBase<Data>) {
    budget_log("unreal_init");

    // advlib::advcmd::Cmd_chapterclear
    let addr = signature_scan("48 89 5c 24 ? 55 56 57 48 81 ec ? ? ? ? 48 8b 05 ? ? ? ? 48 33 c4 48 89 84 24 ? ? ? ? 48 8b 1d ? ? ? ? 8b ea").unwrap();
    let mut save_manager_inst  =  signature_scan_from_addr("48 8B 1D ? ? ? ?", addr).unwrap();
    let offset = (save_manager_inst.offset(3) as *mut u32).read_unaligned();
    let save_data = save_manager_inst.offset(offset as isize + 7);
    budget_log(format!("{:p}", save_data).as_str());


    HOOKS.set(Accessors {
        UREDWidgetRecordingSettings_NativeOnInitialized: hook_function::<fn_UREDWidgetRecordingSettings_NativeOnInitialized>(
            "48 8b c4 48 89 48 ? 55 41 57 48 8d 68 ? 48 81 ec ? ? ? ? 48 89 58 ? 48 8b d9",
            recording_settings_on_init).unwrap(),
        UREDWidgetRecordingSettings_OnInputDecisionTrigger: hook_function::<fn_UREDWidgetRecordingSettings_OnInputDecisionTrigger>(
            "40 55 53 57 48 8b ec 48 83 ec ? 48 8b d9 e8 ? ? ? ? 48 8b cb e8 ? ? ? ? 48 8b f8 48 85 c0 0f 84 ? ? ? ? 48 8b 50 ? 48 8d 4d ? 48 89 55 ? 48 8d 55 ? 48 89 74 24",
            recording_settings_on_input).unwrap(),
        UREDCommonSelectorWindowBase_AddItem: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 5c 24 ? 57 48 83 ec ? 48 8b da 48 8b f9 48 83 bf ? ? ? ? ? 74 ? e8 ? ? ? ? 48 85 c0 74 ? 48 8b 97 ? ? ? ? 4c 8d 40 ? 48 63 40 ? 3b 42 ? 7f ? 48 8b c8 48 8b 42 ? 4c 39 04 c8 75 ? 48 85 d2 75 ? 48 8b cf e8 ? ? ? ? 48 8b f8 48 85 c0 74 ? e8 ? ? ? ? 48 8b 57 ? 4c 8d 40 ? 48 63 40 ? 3b 42 ? 7f ? 48 8b c8 48 8b 42 ? 4c 39 04 c8 74 ? 33 c0 48 8b 5c 24 ? 48 83 c4 ? 5f c3 48 89 6c 24 ? 48 89 74 24 ? 4c 89 74 24 ? e8 ? ? ? ? 33 f6 48 85 c0 74 ? 48 8b af ? ? ? ? 48 8d 50 ? 48 63 40 ? 3b 45 ? 7f ? 48 8b c8 48 8b 45 ? 48 39 14 c8 74 ? 48 8b ee 48 8d 15 ? ? ? ? 48 8b cf e8 ? ? ? ? 4c 8b f0 48 85 c0 74 ? e8 ? ? ? ? 49 8b 4e ? 48 8b d0 e8 ? ? ? ? 84 c0 74 ? 4c 89 b7 ? ? ? ? 48 85 ed 74 ? 4c 8b c3 48 8b d5 48 8b cf e8 ? ? ? ? 4c 8b b7")
            .unwrap())),
        UREDWidgetBase_SetTextBlockTextByID: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 5c 24 ? 48 89 6c 24 ? 48 89 74 24 ? 57 48 83 ec ? 49 8b f0 48 8b da"
        ).unwrap())),
        UREDCommonSelectorWindowBase_GetCursoredItem: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 83 ec ? e8 ? ? ? ? 48 85 c0 74 ? 48 8b c8 48 83 c4 ? e9 ? ? ? ? 48 83 c4 ? c3 cc 48 83 ec ? 48 8b 49"
        ).unwrap())),
        FWindowsPlatformApplicationMisc_ClipboardCopy: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 4c 24 ? 48 83 ec ? ff 15"
        ).unwrap())),
        FWindowsPlatformApplicationMisc_ClipboardPaste: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 4c 24 ? 56 57 48 81 ec ? ? ? ? ff 15"
        ).unwrap())),
        FName_cstr: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 5c 24 ? 57 48 83 ec ? 48 8b d9 48 89 54 24 ? 33 c9"
        ).unwrap())),
        FName_ToString: std::mem::transmute_copy(&ManuallyDrop::new(signature_scan(
            "48 89 5c 24 ? 55 56 57 48 8b ec 48 83 ec ? 8b 01"
        ).unwrap())),
        RED_SaveData: ThreadSafePtr(save_data.cast()),
    });

    let res = MinHook::enable_all_hooks();
    if res.is_err() {
        budget_log("enable all hooks failed");
        budget_log(format!("{:?}", res.unwrap_err()).as_str());
        return;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn start_mod() -> *mut CppUserModBase<Data> {
    clear_log();
    let vtable = Box::new(gglibrary::ue4ss::Vtable {
        on_unreal_init: ModCallback(on_unreal_init),
        ..Default::default()
    });

    unsafe {
        Box::into_raw(Box::new(CppUserModBase {
            vtable: Box::into_raw(vtable),
            padding: [0; 0x18],
            mod_name: CxxString::from_str(""),
            mod_version: CxxString::from_str("1"),
            mod_description: CxxString::from_str(""),
            mod_authors: CxxString::from_str("ilcheese2"),
            mod_intended_sdk_version: CxxString::from_str(""),
            data: Default::default()
        }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uninstall_mod(cpp_mod: *mut CppUserModBase<Data>) {
    std::ptr::drop_in_place(cpp_mod);
    std::alloc::dealloc(cpp_mod.cast(), Layout::new::<CppUserModBase<Data>>());
}