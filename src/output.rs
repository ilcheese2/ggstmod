// use std::mem;
// use std::ops::Deref;
// use crate::budget_log;
// use crate::cxxstd::{CxxStringView, CxxUniquePtr, CxxVector};
//
// #[repr(C)]
// pub struct DeviceVtable {
//     pub has_optional_args: unsafe extern "C" fn(*const Device) -> bool,
//     pub receive: unsafe extern "C" fn(*const Device, *const CxxStringView),
//     pub receive_with_optional_arg: unsafe extern "C" fn(*const Device, *const CxxStringView, u32),
// }
//
// #[repr(C)]
// struct Device {
//     vtable: *const DeviceVtable,
// }
//
// #[link(name="UE4SS", kind="raw-dylib")]
// unsafe extern "C" {
//     // msvc name mangling is stupid
//     #[link_name = "?get_default_devices_ref@DefaultTargets@Output@RC@@SAAEAV?$vector@V?$unique_p tr@VOutputDevice@Output@RC@@U?$default_delete@VOutputDevice@Output@RC@@@std@@@st d@@V?$allocator@V?$unique_ptr@VOutputDevice@Output@RC@@U?$default_delete@VOutput Device@Output@RC@@@std@@@std@@@2@@std@@XZ();"]
//     pub fn get_default_devices_ref() -> *mut CxxVector<CxxUniquePtr<Device>>;
// }
//
// pub fn log() {
//     unsafe {
//         for device in (*get_default_devices_ref()).into_iter() {
//             budget_log(format!("optional {}", ((*device.vtable).has_optional_args)(device.ptr)).as_str());
//         }
//     }
// }

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub fn budget_log(s: &str) {
    #[cfg(debug_assertions)] {}
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(env!("CARGO_MANIFEST_DIR").to_owned() + "/budget.log").unwrap();
        f.write((s.to_string() + "\n").as_bytes()).unwrap();
   // }
}

pub fn clear_log() {
    //#[cfg(debug_assertions)]
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .open(env!("CARGO_MANIFEST_DIR").to_owned() + "/budget.log").unwrap();

    f.set_len(0).unwrap();
}