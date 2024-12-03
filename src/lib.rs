//! A Clojure reader in Rust.
//!
//! This crate tries to match the behavior of Clojure's `tools.reader` as much as possible.
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod edn;
pub mod error;

#[cfg(feature = "derive")]
pub use de::from_str;
#[cfg(feature = "derive")]
pub use ser::to_string;

#[cfg(feature = "derive")]
pub mod de;
#[cfg(feature = "derive")]
pub mod ser;

mod parse;
