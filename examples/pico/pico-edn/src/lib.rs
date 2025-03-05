#![no_main]
#![allow(unsafe_code)]
#![no_std]

extern crate alloc;

use alloc::ffi::CString;
use alloc::format;
use core::ffi::{c_char, CStr};
use core::panic::PanicInfo;

use clojure_reader::edn;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
  loop {
    unsafe {
      printf(c"panic\n".as_ptr());
      sleep_ms(500);
    }
  }
}

unsafe extern "C" {
  fn printf(format: *const c_char, ...) -> i32;
  fn sleep_ms(ms: u32);
}

#[global_allocator]
static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();

#[unsafe(no_mangle)]
/// # Safety
/// must be null terminated c str
/// # Panics
/// panics on any errors, this is just showing a minimal working example, not best practices.
pub unsafe extern "C" fn some_edn(edn: *const c_char) {
  let c_str: &CStr = unsafe { CStr::from_ptr(edn) };
  let str_slice: &str = c_str.to_str().unwrap();

  let edn = edn::read_string(str_slice).unwrap();
  let edn_str = format!("{edn}");
  let c_str = CString::new(edn_str.as_str()).unwrap();
  unsafe {
    printf(c"hello edn %s\n".as_ptr(), c_str.as_ptr());
  }
}
