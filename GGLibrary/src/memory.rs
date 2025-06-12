use minhook::MinHook;
use std::cell::LazyCell;
use std::ffi::c_void;
use std::fmt::format;
use std::i8::MAX;
use std::mem::ManuallyDrop;
use std::ops::{Add, Deref};
use std::sync::LazyLock;
use widestring::{U16CString, U16String};

use crate::output::budget_log;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::psapi::{GetModuleInformation, MODULEINFO};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ThreadSafePtr<T>(pub *mut T); // rust :pensive:

unsafe impl<T> Send for ThreadSafePtr<T> {}
unsafe impl<T> Sync for ThreadSafePtr<T> {}

impl<T> Deref for ThreadSafePtr<T> {
    type Target = *mut T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

static MODULE_BOUNDS: LazyLock<(ThreadSafePtr<u8>, ThreadSafePtr<u8>)> = LazyLock::new(|| {
    let module_name: U16CString = U16CString::from_str("GGST-Win64-Shipping.exe").unwrap();
    let handle = unsafe { GetModuleHandleW(module_name.as_ptr()) };
    let mut module_info: MODULEINFO = MODULEINFO {
        lpBaseOfDll: std::ptr::null_mut(),
        SizeOfImage: 0,
        EntryPoint: std::ptr::null_mut(),
    };
    unsafe {
        GetModuleInformation(
            GetCurrentProcess(),
            handle,
            &mut module_info as *mut MODULEINFO,
            size_of::<MODULEINFO>() as u32,
        )
    };

    let base = module_info.lpBaseOfDll as usize;
    let size = module_info.SizeOfImage as usize;
    unsafe {
        (
            ThreadSafePtr(std::mem::transmute(module_info.lpBaseOfDll)),
            ThreadSafePtr(std::mem::transmute(base + size)),
        )
    }
});

pub fn signature_scan(pattern: &str) -> Option<*mut u8> {
    signature_scan_from_addr(pattern, *MODULE_BOUNDS.0)
}

pub fn signature_scan_from_addr(pattern: &str, start: *mut u8) -> Option<*mut u8> {
    let (_, end) = *MODULE_BOUNDS;
    let end = *end;
    let mut i = start;
    let mut j = 0;
    let mut pattern_start = start;
    let pattern = pattern.split(" ").collect::<Vec<&str>>();
    while i < end {
        if j == pattern.len() {
            return Some(unsafe { pattern_start.add(1) });
        }
        if pattern[j] == "??"
            || pattern[j] == "?"
            || u8::from_str_radix(pattern[j], 16).unwrap() == unsafe { *i }
        {
            j += 1;
        } else {
            j = 0;
            pattern_start = i;
        }
        unsafe { i = i.offset(1) };
    }
    None
}

pub struct Hook<T>
where
    T: Copy,
{
    pub target: T,
    pub orig: T,
}

impl<T: Copy> Hook<T> {
    pub fn enable(&self) {
        unsafe { MinHook::enable_hook(std::mem::transmute_copy(&ManuallyDrop::new(self.target))) };
    } // crying and sobbing
    pub fn disable(&self) {
        unsafe { MinHook::disable_hook(std::mem::transmute_copy(&ManuallyDrop::new(self.target))) };
    }
}

pub fn hook_function<T: Copy>(sig: &str, hook: T) -> Option<Hook<T>> {
    let addr = signature_scan(sig);
    if addr.is_none() {
        budget_log(
            ("signature scan failed for: ".to_owned() + std::any::type_name::<T>()).as_str(),
        );
        return None;
    }
    hook_function_from_addr(addr.unwrap() as *mut c_void, hook)
}

pub fn hook_function_from_addr<T: Copy>(addr: *mut c_void, hook: T) -> Option<Hook<T>> {
    let res = unsafe {
        MinHook::create_hook(
            addr,
            std::mem::transmute_copy(&ManuallyDrop::new(hook)),
        )
    };
    if res.is_err() {
        budget_log(("create hook failed for: ".to_owned() + std::any::type_name::<T>()).as_str());
        budget_log(format!("{:?}", res.unwrap_err()).as_str());
        return None;
    }

    let orig = res.unwrap();
    unsafe {
        Some(Hook {
            target: std::mem::transmute_copy(&ManuallyDrop::new(addr)),
            orig: std::mem::transmute_copy(&ManuallyDrop::new(orig)),
        })
    }
}

pub fn print_memory(ptr: *const u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr , len) };
    hex::encode(slice)
}