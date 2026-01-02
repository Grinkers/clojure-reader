//! An EDN reader/presenter in Rust.
//!
//! ## Implementations
//! -  [`core::fmt::Display`] will output valid EDN for any Edn object
//!
//! ## Differences from Clojure
//! -  Escape characters are not escaped.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt;

#[cfg(feature = "arbitrary-nums")]
use bigdecimal::BigDecimal;
#[cfg(feature = "arbitrary-nums")]
use num_bigint::BigInt;
#[cfg(feature = "floats")]
use ordered_float::OrderedFloat;

use crate::{error, parse};

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Edn<'e> {
  Vector(Vec<Self>),
  Set(BTreeSet<Self>),
  Map(BTreeMap<Self, Self>),
  List(Vec<Self>),
  Key(&'e str),
  Symbol(&'e str),
  Str(&'e str),
  Int(i64),
  Tagged(&'e str, Box<Self>),
  #[cfg(feature = "floats")]
  Double(OrderedFloat<f64>),
  Rational((i64, i64)),
  #[cfg(feature = "arbitrary-nums")]
  BigInt(BigInt),
  #[cfg(feature = "arbitrary-nums")]
  BigDec(BigDecimal),
  Char(char),
  Bool(bool),
  Nil,
}

impl<'e> TryFrom<parse::Node<'e>> for Edn<'e> {
  type Error = error::Error;
  /// Elaborates a concrete [`Node`](parse::Node) into an abstract resolved [`Edn`]
  ///
  /// # Errors
  ///
  /// See [`crate::error::Error`].
  /// Always returns either `Code::HashMapDuplicateKey` or `Code::SetDuplicateKey`.
  fn try_from(value: parse::Node<'e>) -> error::Result<Self> {
    use error::{Code, Error, Result};
    use parse::Node;

    Ok(match value {
      Node::Vector(items, _) => {
        Edn::Vector(items.into_iter().map(TryInto::try_into).collect::<Result<_>>()?)
      }
      Node::Set(items, _) => {
        let mut set = BTreeSet::new();
        for node in items {
          let position = node.span().1;
          if !set.insert(node.try_into()?) {
            return Err(Error::from_position(Code::SetDuplicateKey, position));
          }
        }
        Edn::Set(set)
      }
      Node::Map(entries, _) => {
        let mut map = BTreeMap::new();
        for (key, value) in entries {
          let position = value.span().1;
          if map.insert(key.try_into()?, value.try_into()?).is_some() {
            return Err(Error::from_position(Code::HashMapDuplicateKey, position));
          }
        }
        Edn::Map(map)
      }
      Node::List(items, _) => {
        Edn::List(items.into_iter().map(TryInto::try_into).collect::<Result<_>>()?)
      }
      Node::Key(key, _) => Edn::Key(key),
      Node::Symbol(symbol, _) => Edn::Symbol(symbol),
      Node::Str(str, _) => Edn::Str(str),
      Node::Int(int, _) => Edn::Int(int),
      Node::Tagged(tag, node, _) => Edn::Tagged(tag, Box::new((*node).try_into()?)),
      #[cfg(feature = "floats")]
      Node::Double(double, _) => Edn::Double(double),
      Node::Rational(rational, _) => Edn::Rational(rational),
      #[cfg(feature = "arbitrary-nums")]
      Node::BigInt(big_int, _) => Edn::BigInt(big_int),
      #[cfg(feature = "arbitrary-nums")]
      Node::BigDec(big_dec, _) => Edn::BigDec(big_dec),
      Node::Char(ch, _) => Edn::Char(ch),
      Node::Bool(bool, _) => Edn::Bool(bool),
      Node::Nil(_) => Edn::Nil,
    })
  }
}

/// Reads one object from the &str.
///
/// # Errors
///
/// See [`crate::error::Error`].
pub fn read_string(edn: &str) -> Result<Edn<'_>, error::Error> {
  Ok(parse::parse_as_edn(edn)?.0)
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
  let r = parse::parse_as_edn(edn)?;
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

fn get_tag<'a>(tag: &'a str, key: &'a str) -> Option<&'a str> {
  // Break out early if there's no namespaces
  if !key.contains('/') {
    return None;
  }

  // ignore the leading ':'
  if !tag.starts_with(':') {
    return None;
  }
  let tag = tag.get(1..)?;
  Some(tag)
}

fn check_key<'a>(tag: &'a str, key: &'a str) -> &'a str {
  // check if the Key starts with the saved Tag
  if key.starts_with(tag) {
    let (_, key) = key.rsplit_once(tag).expect("Tag must exist, because it starts with it.");

    // ensure there's a '/' and strip it
    if let Some(k) = key.strip_prefix('/') {
      return k;
    }
  }
  key
}

impl Edn<'_> {
  pub fn get(&self, e: &Self) -> Option<&Self> {
    if let Edn::Map(m) = self {
      return m.get(e);
    } else if let Edn::Tagged(tag, m) = self {
      if let Edn::Key(key) = e {
        let tag = get_tag(tag, key)?;
        let key = check_key(tag, key);

        return m.get(&Edn::Key(key));
      }

      // Cover cases where it's not a keyword
      return m.get(e);
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

  pub fn contains(&self, e: &Self) -> bool {
    match self {
      Edn::Map(m) => m.contains_key(e),
      Edn::Tagged(tag, m) => {
        if let Edn::Key(key) = e {
          let Some(tag) = get_tag(tag, key) else { return false };
          let key = check_key(tag, key);

          return m.contains(&Edn::Key(key));
        }

        // Cover cases where it's not a keyword
        m.contains(e)
      }
      Edn::Vector(v) => v.contains(e),
      Edn::Set(s) => s.contains(e),
      Edn::List(l) => l.contains(e),
      _ => false,
    }
  }
}

pub(crate) const fn char_to_edn(c: char) -> Option<&'static str> {
  match c {
    '\n' => Some("newline"),
    '\r' => Some("return"),
    ' ' => Some("space"),
    '\t' => Some("tab"),
    _ => None,
  }
}

impl fmt::Display for Edn<'_> {
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
      Self::Tagged(t, s) => write!(f, "#{t} {s}"),
      Self::Key(k) => write!(f, ":{k}"),
      Self::Str(s) => write!(f, "\"{s}\""),
      Self::Int(i) => write!(f, "{i}"),
      #[cfg(feature = "floats")]
      Self::Double(d) => write!(f, "{d}"),
      #[cfg(feature = "arbitrary-nums")]
      Self::BigInt(bi) => write!(f, "{bi}N"),
      #[cfg(feature = "arbitrary-nums")]
      Self::BigDec(bd) => write!(f, "{bd}M"),
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
