//! An EDN reader/presenter in Rust.
//!
//! ## Implementations
//! -  [`core::fmt::Display`] will output valid EDN for any Edn object. Alternate formatting
//!    (`{edn:#}`) outputs indented EDN, with deeply nested subtrees falling back to compact
//!    formatting to bound indentation overhead.
//! -  With the `unstable` feature enabled, [`TryFrom`]<[`parse::Node`]> implemented for [`Edn`]
//!    will convert the Node into an Edn
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

const SYMBOL_SPECIAL_CHARS: &str = ".*+!-_?$%&=<>:#";
pub(crate) const MAX_PRETTY_DEPTH: usize = 42;

fn is_symbol_char(c: char) -> bool {
	c.is_alphanumeric() || SYMBOL_SPECIAL_CHARS.contains(c)
}

fn is_symbol_start(c: char) -> bool {
	!c.is_numeric() && !matches!(c, ':' | '#') && is_symbol_char(c)
}

fn valid_symbol_part(part: &str) -> bool {
	let mut chars = part.chars();
	let Some(first) = chars.next() else { return false };
	let second = chars.clone().next();

	is_symbol_start(first)
		&& !(matches!(first, '-' | '+' | '.') && second.is_some_and(char::is_numeric))
		&& chars.all(is_symbol_char)
}

pub(crate) fn validate_tag(tag: &str, tag_span: parse::Span) -> error::Result<()> {
	let tag = tag.strip_prefix(':').unwrap_or(tag);
	let valid = tag.chars().next().is_some_and(char::is_alphabetic)
		&& match tag.split_once('/') {
			Some((prefix, name)) => {
				!name.contains('/') && valid_symbol_part(prefix) && valid_symbol_part(name)
			}
			None => valid_symbol_part(tag),
		};

	if valid { Ok(()) } else { Err(error::Error::from_position(error::Code::InvalidTag, tag_span.0)) }
}

impl<'e> TryFrom<parse::Node<'e>> for Edn<'e> {
	type Error = error::Error;
	/// Elaborates a concrete [`Node`](parse::Node) into an abstract resolved [`Edn`]
	///
	/// ```
	/// #[cfg(feature = "unstable")]
	/// {
	///   use clojure_reader::{parse, edn::Edn};
	///
	///   let edn: Edn = parse::Node::no_discards(parse::NodeKind::Nil, parse::Span::default())
	///     .try_into()
	///     .unwrap();
	///
	///   assert_eq!(edn, Edn::Nil);
	/// }
	/// ```
	///
	/// # Errors
	///
	/// See [`crate::error::Error`].
	///
	/// [HMDK]: error::Code::HashMapDuplicateKey
	/// [SDK]: error::Code::SetDuplicateKey
	/// [IT]: error::Code::InvalidTag
	fn try_from(parse::Node { kind: value, .. }: parse::Node<'e>) -> error::Result<Self> {
		use error::{Code, Error, Result};
		use parse::NodeKind;

		Ok(match value {
			NodeKind::Vector(items, _) => {
				Edn::Vector(items.into_iter().map(TryInto::try_into).collect::<Result<_>>()?)
			}
			NodeKind::Set(items, _) => {
				let mut set = BTreeSet::new();
				for node in items {
					let position = node.span().1;
					if !set.insert(node.try_into()?) {
						return Err(Error::from_position(Code::SetDuplicateKey, position));
					}
				}
				Edn::Set(set)
			}
			NodeKind::Map(entries, _) => {
				let mut map = BTreeMap::new();
				for (key, value) in entries {
					let position = value.span().1;
					if map.insert(key.try_into()?, value.try_into()?).is_some() {
						return Err(Error::from_position(Code::HashMapDuplicateKey, position));
					}
				}
				Edn::Map(map)
			}
			NodeKind::List(items, _) => {
				Edn::List(items.into_iter().map(TryInto::try_into).collect::<Result<_>>()?)
			}
			NodeKind::Key(key) => Edn::Key(key),
			NodeKind::Symbol(symbol) => Edn::Symbol(symbol),
			NodeKind::Str(str) => Edn::Str(str),
			NodeKind::Int(int) => Edn::Int(int),
			NodeKind::Tagged(tag, tag_span, node) => {
				validate_tag(tag, tag_span)?;
				if tag.starts_with(':') && !matches!(&node.kind, NodeKind::Map(..)) {
					return Err(Error::from_position(Code::InvalidTag, tag_span.0));
				}
				Edn::Tagged(tag, Box::new((*node).try_into()?))
			}
			#[cfg(feature = "floats")]
			NodeKind::Double(double) => Edn::Double(double),
			NodeKind::Rational(rational) => Edn::Rational(rational),
			#[cfg(feature = "arbitrary-nums")]
			NodeKind::BigInt(big_int) => Edn::BigInt(big_int),
			#[cfg(feature = "arbitrary-nums")]
			NodeKind::BigDec(big_dec) => Edn::BigDec(big_dec),
			NodeKind::Char(ch) => Edn::Char(ch),
			NodeKind::Bool(bool) => Edn::Bool(bool),
			NodeKind::Nil => Edn::Nil,
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
	let (edn, remaining) = parse::parse_optional_edn(edn)?;
	let Some(edn) = edn else {
		return Err(error::Error {
			code: error::Code::UnexpectedEOF,
			line: None,
			column: None,
			ptr: None,
		});
	};
	Ok((edn, remaining))
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

fn write_indent(f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
	for _ in 0..depth {
		f.write_str("\t")?;
	}
	Ok(())
}

impl Edn<'_> {
	fn fmt_edn(&self, f: &mut fmt::Formatter<'_>, pretty: bool, depth: usize) -> fmt::Result {
		let pretty = pretty && depth < MAX_PRETTY_DEPTH;
		match self {
			Self::Vector(v) => Self::fmt_sequence(f, "[", "]", v, pretty, depth),
			Self::Set(s) => Self::fmt_sequence(f, "#{", "}", s, pretty, depth),
			Self::Map(m) => {
				f.write_str("{")?;
				let mut entries = m.iter().peekable();
				while let Some((key, value)) = entries.next() {
					if pretty {
						f.write_str("\n")?;
						write_indent(f, depth + 1)?;
					}
					key.fmt_edn(f, pretty, depth + 1)?;
					f.write_str(" ")?;
					value.fmt_edn(f, pretty, depth + 1)?;
					if entries.peek().is_some() {
						if pretty {
							f.write_str(",")?;
						} else {
							f.write_str(", ")?;
						}
					}
				}
				if pretty && !m.is_empty() {
					f.write_str("\n")?;
					write_indent(f, depth)?;
				}
				f.write_str("}")
			}
			Self::List(l) => Self::fmt_sequence(f, "(", ")", l, pretty, depth),
			Self::Symbol(sy) => write!(f, "{sy}"),
			Self::Tagged(t, value) => {
				write!(f, "#{t} ")?;
				value.fmt_edn(f, pretty, depth)
			}
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
				f.write_str("\\")?;
				if let Some(c) = char_to_edn(*c) {
					return f.write_str(c);
				}
				write!(f, "{c}")
			}
			Self::Nil => f.write_str("nil"),
		}
	}

	fn fmt_sequence<'a, I>(
		f: &mut fmt::Formatter<'_>,
		opener: &str,
		closer: &str,
		items: I,
		pretty: bool,
		depth: usize,
	) -> fmt::Result
	where
		I: IntoIterator<Item = &'a Self>,
		Self: 'a,
	{
		f.write_str(opener)?;
		let mut items = items.into_iter().peekable();
		let mut has_items = false;
		while let Some(item) = items.next() {
			has_items = true;
			if pretty {
				f.write_str("\n")?;
				write_indent(f, depth + 1)?;
			}
			item.fmt_edn(f, pretty, depth + 1)?;
			if !pretty && items.peek().is_some() {
				f.write_str(" ")?;
			}
		}
		if pretty && has_items {
			f.write_str("\n")?;
			write_indent(f, depth)?;
		}
		f.write_str(closer)
	}
}

impl fmt::Display for Edn<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_edn(f, f.alternate(), 0)
	}
}
