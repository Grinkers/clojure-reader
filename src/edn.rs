//! An EDN reader/presenter in Rust.
//!
//! ## Implementations
//! -  [`core::fmt::Display`] will output valid EDN for any Edn object
//!
//! ## Differences from Clojure
//! -  Escape characters are not escaped.
//! -  Tags are current unimplemented.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt;

#[cfg(feature = "floats")]
use ordered_float::OrderedFloat;

use crate::{error, parse};

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Edn<'e> {
  Vector(Vec<Edn<'e>>),
  Set(BTreeSet<Edn<'e>>),
  Map(BTreeMap<Edn<'e>, Edn<'e>>),
  List(Vec<Edn<'e>>),
  Key(&'e str),
  Symbol(&'e str),
  Str(&'e str),
  Int(i64),
  #[cfg(feature = "floats")]
  Double(OrderedFloat<f64>),
  Rational((i64, i64)),
  Char(char),
  Bool(bool),
  Nil,
}

/// Reads one object from the &str.
///
/// # Errors
///
/// See [`crate::error::Error`].
pub fn read_string(edn: &str) -> Result<Edn<'_>, error::Error> {
  Ok(parse::parse(edn)?.0)
}

/// Reads the first object from the &str and the remaining unread &str.
///
/// # Errors
///
/// Default behavior of Clojure's `read` is to throw an error on EOF, unlike `read_string`.
/// <https://clojure.github.io/tools.reader/#clojure.tools.reader.edn/read>
///
/// See [`crate::error::Error`].
pub fn read(edn: &str) -> Result<(Edn<'_>, &str), error::Error> {
  let r = parse::parse(edn)?;
  if r.0 == Edn::Nil && r.1.is_empty() {
    return Err(error::Error {
      code: error::Code::UnexpectedEOF,
      line: None,
      column: None,
      ptr: None,
    });
  }
  Ok((r.0, r.1))
}

impl<'e> Edn<'e> {
  pub fn get(&self, e: &Self) -> Option<&Self> {
    if let Edn::Map(m) = self {
      let lol = m.get(e);
      if let Some(l) = lol {
        return Some(l);
      };
    }
    None
  }
  pub fn nth(&self, i: usize) -> Option<&Self> {
    let vec = match self {
      Edn::Vector(v) => v,
      Edn::List(l) => l,
      _ => return None,
    };

    vec.get(i)
  }
}

const fn char_to_edn(c: char) -> Option<&'static str> {
  match c {
    '\n' => Some("newline"),
    '\r' => Some("return"),
    ' ' => Some("space"),
    '\t' => Some("tab"),
    _ => None,
  }
}

impl<'e> fmt::Display for Edn<'e> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Vector(v) => {
        write!(f, "[")?;
        let mut it = v.iter().peekable();
        while let Some(i) = it.next() {
          if it.peek().is_some() {
            write!(f, "{i} ")?;
          } else {
            write!(f, "{i}")?;
          }
        }
        write!(f, "]")
      }
      Self::Set(s) => {
        write!(f, "#{{")?;
        let mut it = s.iter().peekable();
        while let Some(i) = it.next() {
          if it.peek().is_some() {
            write!(f, "{i} ")?;
          } else {
            write!(f, "{i}")?;
          }
        }
        write!(f, "}}")
      }
      Self::Map(m) => {
        write!(f, "{{")?;
        let mut it = m.iter().peekable();
        while let Some(kv) = it.next() {
          if it.peek().is_some() {
            write!(f, "{} {}, ", kv.0, kv.1)?;
          } else {
            write!(f, "{} {}", kv.0, kv.1)?;
          }
        }
        write!(f, "}}")
      }
      Self::List(l) => {
        write!(f, "(")?;
        let mut it = l.iter().peekable();
        while let Some(i) = it.next() {
          if it.peek().is_some() {
            write!(f, "{i} ")?;
          } else {
            write!(f, "{i}")?;
          }
        }
        write!(f, ")")
      }
      Self::Symbol(sy) => write!(f, "{sy}"),
      Self::Key(k) => write!(f, "{k}"),
      Self::Str(s) => write!(f, "\"{s}\""),
      Self::Int(i) => write!(f, "{i}"),
      #[cfg(feature = "floats")]
      Self::Double(d) => write!(f, "{d}"),
      Self::Rational((n, d)) => write!(f, "{n}/{d}"),
      Self::Bool(b) => write!(f, "{b}"),
      Self::Char(c) => {
        write!(f, "\\")?;
        if let Some(c) = char_to_edn(*c) {
          return write!(f, "{c}");
        }
        write!(f, "{c}")
      }
      Self::Nil => write!(f, "nil"),
    }
  }
}
