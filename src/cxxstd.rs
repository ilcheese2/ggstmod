use std::fmt;
use std::ops::Deref;
use libc::{free, malloc, memcpy, memset};
use widestring::U16String;
use crate::output::budget_log;

const BUF_SIZE: usize = 16;
const SMALL_STRING_SIZE: usize = (BUF_SIZE - 1) / size_of::<u16>(); // 7

#[repr(C)]
pub union SmallString {
    small: [u8; BUF_SIZE],
    pub large: *mut u8
}

impl fmt::Debug for SmallString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {f.debug_struct("SmallString")
            .field("large", &self.large)
            .finish()}
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct CxxString { // wstring
    pub data: SmallString,
    length: usize, // number of characters
    capacity: usize
}

impl CxxString {
    pub fn new() -> Self {
        Self {
            data: SmallString { small: [0; BUF_SIZE] },
            length: 0,
            capacity: SMALL_STRING_SIZE,
        }
    }

    pub unsafe fn from_str(s: &str) -> Self {
        let mut string = Self::new();
        let wstring = U16String::from_str(s);
        let nbytes = wstring.len() * size_of::<u16>() + 1;
        string.length = wstring.len();
        string.capacity = wstring.len();
        if string.length <= SMALL_STRING_SIZE {
            std::ptr::copy_nonoverlapping::<u8>(wstring.as_ptr().cast(), string.data.small.as_mut_ptr(), nbytes - 1);
        } else {
            let ptr = malloc(nbytes); // msvc new uses malloc so...
            memset(ptr, 0, nbytes); // set null terminator
            memcpy(ptr, wstring.as_ptr().cast(), nbytes - 1);
            string.data.large = ptr.cast();
        }
        string
    }

    pub fn string(&self) -> String {
        if self.length <= SMALL_STRING_SIZE {
            unsafe { U16String::from_ptr(self.data.small.as_ptr().cast(), self.length) }.to_string().unwrap()
        } else {
            budget_log(format!("string: {}", self.length).as_str());
            unsafe { budget_log(format!("string: {:p}", self.data.large).as_str()) };
            unsafe { U16String::from_ptr(self.data.large.cast(), self.length) }.to_string().unwrap()
        }
    }
}

impl Drop for CxxString {
    fn drop(&mut self) {
        if self.length > SMALL_STRING_SIZE {
            unsafe {free(self.data.large.cast()) };
        }
    }
}

pub struct CxxVector<T> {
    first: *const T,
    last : *const T,
    end: *const T,
}

impl<T> CxxVector<T> {
    pub fn is_empty(&self) -> bool {
        self.first == self.last
    }
}

impl<'a, T> IntoIterator for &'a CxxVector<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe { std::slice::from_raw_parts(self.first, self.last.offset_from(self.first) as usize).iter() }
    }
}

pub struct CxxUniquePtr<T> {
    pub ptr: *mut T,
}

impl<T> Deref for CxxUniquePtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CxxStringView {
    data: *mut u16,
    length: usize,
}

impl CxxStringView {

    pub unsafe fn from_str(s: &str) -> Self {
        let mut string = Self {
            data: std::ptr::null_mut(),
            length: 0,
        };
        string.length = s.len();
        let wstring = U16String::from_str(s);
        let nbytes = wstring.len() * size_of::<u16>() + 1;
        let ptr = malloc(nbytes);
        memset(ptr, 0, nbytes);
        memcpy(ptr, wstring.as_ptr().cast(), nbytes - 1);
        string.data = ptr.cast();
        string
    }

    pub fn string(&self) -> String {
        unsafe { U16String::from_ptr(self.data, self.length) }.to_string_lossy()
    }
}

impl Drop for CxxStringView {
    fn drop(&mut self) {
        if self.length > BUF_SIZE {
            unsafe {free(self.data.cast()) };
        }
    }
}