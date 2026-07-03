//! A Clojure reader in Rust.
//!
//! This crate tries to match the behavior of Clojure's `tools.reader` as much as possible.
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod edn;
pub mod error;

#[cfg(feature = "serde")]
pub use de::from_str;
#[cfg(feature = "serde")]
pub use ser::to_string;

#[cfg(feature = "serde")]
pub mod de;
#[cfg(feature = "serde")]
pub mod ser;

#[cfg(feature = "unstable")]
pub mod parse;
#[cfg(not(feature = "unstable"))]
mod parse;
