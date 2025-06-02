use std::cell::LazyCell;
use std::fmt::format;
use std::i8::MAX;
use std::ops::{Add, Deref};
use std::sync::LazyLock;
use widestring::{U16CString, U16String};

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
    let module_name: U16CString  = U16CString::from_str("GGST-Win64-Shipping.exe").unwrap();
    let handle = unsafe { GetModuleHandleW(module_name.as_ptr()) };
    let mut module_info: MODULEINFO = MODULEINFO {
        lpBaseOfDll: std::ptr::null_mut(),
        SizeOfImage: 0,
        EntryPoint: std::ptr::null_mut(),
    };
    unsafe { GetModuleInformation(GetCurrentProcess(), handle, &mut module_info as *mut MODULEINFO, size_of::<MODULEINFO>() as u32) };

    let base = module_info.lpBaseOfDll as usize;
    let size = module_info.SizeOfImage as usize;
    unsafe { (ThreadSafePtr(std::mem::transmute(module_info.lpBaseOfDll)), ThreadSafePtr(std::mem::transmute(base + size))) }
});

pub fn signature_scan(
    pattern: &str,
) -> Option<*const u8> {
    let (start, end) = *MODULE_BOUNDS;
    let (start, end) = (*start, *end);
    let size = unsafe { end.offset_from(start) };
    let mut i = start;
    let mut j = 0;
    let mut pattern_start = start;
    let pattern = pattern.split(" ").collect::<Vec<&str>>();
    while i < end {
        if j == pattern.len() {
            return Some(unsafe{ pattern_start.add(1) });
        }
        if pattern[j] == "??" || pattern[j] == "?" || u8::from_str_radix(pattern[j], 16).unwrap() == unsafe { *i } {
            j += 1;
        } else {
            j = 0;
            pattern_start = i;
        }
        unsafe { i = i.offset(1) };
    }
    None
}