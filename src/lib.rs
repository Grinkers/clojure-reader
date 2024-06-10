//! A Clojure reader in Rust.
//!
//! This crate tries to match the behavior of Clojure's `tools.reader` as much as possible. EDN is
//! almost complete.
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod edn;
pub mod error;

mod parse;
