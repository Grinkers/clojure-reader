//! A Clojure reader in Rust.
//!
//! This crate tries to match the behavior of Clojure's `tools.reader` as much as possible.
#![no_std]
#![cfg_attr(feature = "unstable", expect(clippy::missing_errors_doc))]

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

#[cfg(feature = "unstable")]
pub mod parse;
#[cfg(not(feature = "unstable"))]
mod parse;
