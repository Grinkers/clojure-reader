#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod edn;
pub mod error;

mod parse;
