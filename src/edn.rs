use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt;

#[cfg(feature = "floats")]
use ordered_float::OrderedFloat;

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

/// # Errors
///
/// See error.rs
pub fn read_string(edn: &str) -> Result<Edn<'_>, crate::error::Error> {
    crate::parse::parse(edn)
}

impl<'e> Edn<'e> {
    pub fn get(&self, e: &Edn<'e>) -> Option<&Edn<'e>> {
        if let Edn::Map(m) = self {
            let lol = m.get(e);
            if let Some(l) = lol {
                return Some(l);
            };
        }
        None
    }
    pub fn nth(&self, i: usize) -> Option<&Edn<'e>> {
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
