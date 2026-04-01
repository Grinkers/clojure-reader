//! An EDN syntax parser in Rust.
#![expect(clippy::inline_always)]

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::mem::replace;
use core::primitive::str;

use crate::edn::Edn;
use crate::error::{Code, Error};

#[cfg(feature = "arbitrary-nums")]
use bigdecimal::BigDecimal;
#[cfg(feature = "arbitrary-nums")]
use num_bigint::BigInt;
#[cfg(feature = "floats")]
use ordered_float::OrderedFloat;

/// Possible kinds of an EDN node
///
/// **NOTE:** The vector of items in [`NodeKind::Set`] may contain duplicate items.
/// **NOTE:** The vector of entries in [`NodeKind::Map`] may contain duplicate keys.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum NodeKind<'e> {
	Vector(
		Vec<Node<'e>>,
		/* Any trailing discards inside vector, e.g. `[foo bar #_baz #_qux]` */ Vec<Discard<'e>>,
	),
	Set(
		Vec<Node<'e>>,
		/* Any trailing discards inside set, e.g. `#{foo bar #_baz #_qux}` */ Vec<Discard<'e>>,
	),
	Map(
		Vec<(Node<'e>, Node<'e>)>,
		/* Any trailing discards inside map, e.g. `{:foo bar #_baz #_qux}` */ Vec<Discard<'e>>,
	),
	List(
		Vec<Node<'e>>,
		/* Any trailing discards inside list, e.g. `(foo bar #_baz #_qux)` */ Vec<Discard<'e>>,
	),
	Key(&'e str),
	Symbol(&'e str),
	Str(&'e str),
	Int(i64),
	Tagged(&'e str, /* Span of the tag string */ Span, Box<Node<'e>>),
	#[cfg(feature = "floats")]
	Double(OrderedFloat<f64>),
	Rational((i64, i64)),
	#[cfg(feature = "arbitrary-nums")]
	BigInt(BigInt),
	#[cfg(feature = "arbitrary-nums")]
	BigDec(BigDecimal),
	Char(char),
	Bool(bool),
	#[default]
	Nil,
}

/// A **discarded** form containing the node that was discarded
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Discard<'e>(pub Node<'e>, pub Span);

/// Concrete EDN syntax tree.
///
/// Parse one with [`parse`], then convert it to an [`Edn`] with [`Edn::try_from`].
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node<'e> {
	pub kind: NodeKind<'e>,
	pub span: Span,
	pub leading_discards: Vec<Discard<'e>>,
}

impl<'e> Node<'e> {
	/// Construct a `Node` with the given kind and span and no leading discards.
	pub const fn no_discards(kind: NodeKind<'e>, span: Span) -> Self {
		Self { kind, span, leading_discards: Vec::new() }
	}

	#[inline]
	pub const fn span(&self) -> Span {
		self.span
	}
}

/// Parse a single `Node` from a [`SourceReader`], consuming that form.
///
/// # Examples
///
/// ```
/// #[cfg(feature = "unstable")]
/// {
///   use clojure_reader::parse::{Node, NodeKind::*, SourceReader, parse};
///
///   let source = r#"
/// (->> txs
///   (keep :refund-amt)
///   (reduce +))
///   ; total refund amount
/// "#;
///   let mut reader = SourceReader::new(source);
///   let Node { kind: node, .. } = parse(&mut reader).expect("failed to parse");
///
///   let List(nodes, _) = node else { panic!("unexpected") };
///
///   // Destruct main list
///   let nodes: Vec<_> = nodes.into_iter().map(|n| n.kind).collect();
///   let [Symbol("->>"), Symbol("txs"), List(keep, _), List(reduce, _)] = nodes.as_slice() else {
///     panic!("unexpected");
///   };
///
///   // Destruct the list calling `keep`
///   let keep: Vec<_> = keep.into_iter().map(|n| &n.kind).collect();
///   let [Symbol("keep"), Key("refund-amt")] = keep.as_slice() else {
///     panic!("unexpected");
///   };
///
///   // Destruct the list calling `reduce`
///   let reduce: Vec<_> = reduce.into_iter().map(|n| &n.kind).collect();
///   let [Symbol("reduce"), Symbol("+")] = reduce.as_slice() else {
///     panic!("unexpected");
///   };
///
///   assert_eq!(reader.remaining(), "\n  ; total refund amount\n");
/// }
/// ```
///
/// # Errors
///
/// See [`crate::error::Error`].
#[cfg_attr(not(feature = "unstable"), expect(dead_code))]
pub fn parse<'r, 'e: 'r>(reader: &'r mut SourceReader<'e>) -> Result<Node<'e>, Error> {
	parse_with(reader, &NodeBuilder)
}

/// Parse the first EDN form from a string and return it with the unread remainder.
///
/// # Errors
///
/// See [`crate::error::Error`].
pub fn parse_as_edn(edn: &str) -> Result<(Edn<'_>, &str), Error> {
	let mut source_reader = SourceReader::new(edn);
	let parsed = parse_with(&mut source_reader, &EdnBuilder)?;
	Ok((parsed, source_reader.remaining()))
}

const DELIMITERS: [char; 8] = [',', ']', '}', ')', ';', '(', '[', '{'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
	pub line: usize,
	pub column: usize,
	pub ptr: usize,
}

impl Default for Position {
	fn default() -> Self {
		Self { line: 1, column: 1, ptr: 0 }
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span(pub Position, pub Position);

impl Span {
	/// Whether the span is empty
	pub const fn is_empty(&self) -> bool {
		self.0.ptr == self.1.ptr
	}
}

/// A string-slice reader that records how much of the slice has been read
#[derive(Debug)]
pub struct SourceReader<'s> {
	slice: &'s str,
	// Position till where this string has been read
	read_pos: Position,
}

impl<'e> SourceReader<'e> {
	pub fn new(source: &'e str) -> Self {
		Self { slice: source, read_pos: Position::default() }
	}

	/// Span from some previously marked position to the current position of the reader
	#[inline(always)]
	pub const fn span_from(&self, marker: Position) -> Span {
		Span(marker, self.read_pos)
	}

	/// The portion of the source-string remaining to be read
	///
	/// ```
	/// #[cfg(feature = "unstable")]
	/// {
	///   use clojure_reader::parse::{SourceReader, parse};
	///
	///   let mut s = SourceReader::new("() []");
	///   let _ = parse(&mut s).expect("failed to parse");
	///   assert_eq!(s.remaining(), " []");
	/// }
	/// ```
	pub fn remaining(&self) -> &'e str {
		&self.slice[self.read_pos.ptr..]
	}

	/// Finishes the source-reader, returning:
	/// 1. the current reader position, and
	/// 2. the original source-string.
	///
	/// ```
	/// #[cfg(feature = "unstable")]
	/// {
	///   use clojure_reader::parse::{Position, SourceReader, parse};
	///
	///   let mut s = SourceReader::new("() []");
	///   let _ = parse(&mut s).expect("failed to parse");
	///
	///   let (pos, slice) = s.finish();
	///   assert_eq!(
	///       pos,
	///       Position {
	///           line: 1,
	///           column: 3,
	///           ptr: 2
	///       }
	///   );
	///   assert_eq!(slice, "() []");
	///   assert_eq!(&slice[pos.ptr..], " []");
	/// }
	/// ```
	#[cfg_attr(not(feature = "unstable"), expect(dead_code))]
	pub const fn finish(self) -> (Position, &'e str) {
		(self.read_pos, self.slice)
	}

	// Slurps until whitespace or delimiter, returning the slice.
	#[inline(always)]
	fn slurp_literal(&mut self) -> &'e str {
		let token = self.slice[self.read_pos.ptr..]
			.split(|c: char| c.is_whitespace() || DELIMITERS.contains(&c) || c == '"')
			.next()
			.expect("Expected at least an empty slice");

		self.read_pos.ptr += token.len();
		self.read_pos.column += token.chars().count();
		token
	}

	// Slurps a char. Special handling for chars that happen to be delimiters
	#[inline(always)]
	fn slurp_char(&mut self) -> &'e str {
		let starting_ptr = self.read_pos.ptr;

		let mut ptr = 0;
		while let Some(c) = self.peek_next() {
			// first is always \\, second is always a char we want.
			// Handles edge cases of having a valid "\\[" but also "\\c[lolthisisvalidedn"
			if ptr > 1 && (c.is_whitespace() || DELIMITERS.contains(&c)) {
				break;
			}

			let _ = self.nibble_next();
			ptr += c.len_utf8();
		}
		&self.slice[starting_ptr..starting_ptr + ptr]
	}

	#[inline(always)]
	fn slurp_str(&mut self) -> Result<&'e str, Error> {
		let _ = self.nibble_next(); // Consume the leading '"' char
		let starting_ptr = self.read_pos.ptr;
		let mut escape = false;
		loop {
			if let Some(c) = self.nibble_next() {
				if escape {
					match c {
						't' | 'r' | 'n' | '\\' | '"' => (),
						_ => {
							return Err(Error::from_position(Code::InvalidEscape, self.read_pos));
						}
					}
					escape = false;
				} else if c == '"' {
					return Ok(&self.slice[starting_ptr..self.read_pos.ptr - 1]);
				} else {
					escape = c == '\\';
				}
			} else {
				return Err(Error::from_position(Code::UnexpectedEOF, self.read_pos));
			}
		}
	}

	#[inline(always)]
	fn slurp_tag(&mut self) -> Result<&'e str, Error> {
		self.nibble_whitespace();
		let starting_ptr = self.read_pos.ptr;

		loop {
			if let Some(c) = self.peek_next() {
				if c.is_whitespace() || DELIMITERS.contains(&c) {
					return Ok(&self.slice[starting_ptr..self.read_pos.ptr]);
				}
				let _ = self.nibble_next();
			} else {
				return Err(Error::from_position(Code::UnexpectedEOF, self.read_pos));
			}
		}
	}

	// Nibbles away until the next new line
	#[inline(always)]
	fn nibble_newline(&mut self) {
		let len =
			self.slice[self.read_pos.ptr..].split('\n').next().expect("Expected at least an empty slice");
		self.read_pos.ptr += len.len();
		self.nibble_whitespace();
	}

	// Nibbles away until the start of the next form
	#[inline(always)]
	fn nibble_whitespace(&mut self) {
		while let Some(n) = self.peek_next() {
			if n == ',' || n.is_whitespace() {
				let _ = self.nibble_next();
				continue;
			}
			break;
		}
	}

	// Consumes next
	#[inline(always)]
	fn nibble_next(&mut self) -> Option<char> {
		let char = self.slice[self.read_pos.ptr..].chars().next();
		if let Some(c) = char {
			self.read_pos.ptr += c.len_utf8();
			if c == '\n' {
				self.read_pos.line += 1;
				self.read_pos.column = 1;
			} else {
				self.read_pos.column += 1;
			}
		}
		char
	}

	// Peek into the next char
	#[inline(always)]
	fn peek_next(&self) -> Option<char> {
		self.slice[self.read_pos.ptr..].chars().next()
	}
}

struct Parsed<I> {
	item: I,
	span: Span,
}

impl<I> Parsed<I> {
	const fn new(item: I, span: Span) -> Self {
		Self { item, span }
	}
}

struct Walker<'e, 'r, B: InternalParser<'e>> {
	reader: &'r mut SourceReader<'e>,
	stack: Vec<ParseContext<'e, B>>,
}

impl<'e, 'r, B: InternalParser<'e>> Walker<'e, 'r, B> {
	fn new(reader: &'r mut SourceReader<'e>) -> Self {
		Self {
			reader,
			stack: alloc::vec![ParseContext { kind: ContextKind::Top, discards: Vec::new() }],
		}
	}

	#[inline(always)]
	const fn pos(&self) -> Position {
		self.reader.read_pos
	}

	/// Span from some previously marked position to the current position of the walker's reader
	#[inline(always)]
	const fn span_from(&self, marker: Position) -> Span {
		Span(marker, self.reader.read_pos)
	}

	#[inline(always)]
	fn push_context(&mut self, ctx: ParseContext<'e, B>) {
		self.stack.push(ctx);
	}

	#[inline(always)]
	fn pop_context(&mut self) -> Option<ParseContext<'e, B>> {
		self.stack.pop()
	}

	#[inline(always)]
	const fn stack_len(&self) -> usize {
		self.stack.len()
	}

	const fn make_error(&self, code: Code) -> Error {
		Error::from_position(code, self.pos())
	}

	fn last_context_discards(&mut self) -> Option<&mut Vec<B::Discard>> {
		match self.stack.last_mut() {
			Some(ParseContext { discards, .. }) => Some(discards),
			None => None,
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum OpenDelimiter {
	Vector,
	List,
	Map,
	Hash,
}

// `Position`, wherever present, contains the start position of that context
enum ContextKind<'e, B: InternalParser<'e>> {
	Top,
	Vector(B::VectorContext, Position),
	List(B::ListContext, Position),
	Map(B::MapContext, Position),
	Set(B::SetContext, Position),
	Tag(&'e str, /* Span of the tag string */ Span, Position),
	Discard(Position),
}

struct ParseContext<'e, B: InternalParser<'e>> {
	kind: ContextKind<'e, B>,
	discards: Vec<B::Discard>,
}

impl<'e, B: InternalParser<'e>> ParseContext<'e, B> {
	const fn no_discards(kind: ContextKind<'e, B>) -> Self {
		Self { kind, discards: Vec::new() }
	}
}

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "arbitrary-nums"), derive(Copy))]
enum Atom<'e> {
	Key(&'e str),
	Symbol(&'e str),
	Str(&'e str),
	Int(i64),
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

trait InternalParser<'e> {
	type Item;
	type Discard;
	type VectorContext;
	type ListContext;
	type MapContext;
	type SetContext;

	fn atom(&self, atom: Atom<'e>, span: Span) -> Self::Item;

	fn with_leading_discards(
		&self,
		item: Self::Item,
		leading_discards: Vec<Self::Discard>,
	) -> Self::Item;

	fn new_vector_context(&self) -> Self::VectorContext;

	fn new_list_context(&self) -> Self::ListContext;

	fn new_map_context(&self) -> Self::MapContext;

	fn new_set_context(&self) -> Self::SetContext;

	fn add_to_vector(
		&self,
		ctx: &mut Self::VectorContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error>;

	fn add_to_list(
		&self,
		ctx: &mut Self::ListContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error>;

	fn add_to_map(
		&self,
		ctx: &mut Self::MapContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error>;

	fn add_to_set(
		&self,
		ctx: &mut Self::SetContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error>;

	fn finish_vector(
		&self,
		ctx: Self::VectorContext,
		trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error>;

	fn finish_set(
		&self,
		ctx: Self::SetContext,
		trailing_discards: Vec<Self::Discard>,
		validate: bool,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error>;

	fn finish_map(
		&self,
		ctx: Self::MapContext,
		trailing_discards: Vec<Self::Discard>,
		validate: bool,
		close_pos: Position,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error>;

	fn finish_list(
		&self,
		ctx: Self::ListContext,
		trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error>;

	fn tag(
		&self,
		tag: &'e str,
		tag_span: Span,
		value: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
		span: Span,
	) -> Parsed<Self::Item>;

	fn discard(
		&self,
		value: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
		discard_span: Span,
	) -> Self::Discard;

	fn nil(&self, span: Span) -> Self::Item {
		self.atom(Atom::Nil, span)
	}
}

struct EdnBuilder;

impl<'e> InternalParser<'e> for EdnBuilder {
	type Item = Edn<'e>;
	type Discard = ();
	type VectorContext = Vec<Edn<'e>>;
	type ListContext = Vec<Edn<'e>>;
	type MapContext = (Vec<(Parsed<Edn<'e>>, Parsed<Edn<'e>>)>, Option<Parsed<Edn<'e>>>);
	type SetContext = Vec<Parsed<Edn<'e>>>;

	fn atom(&self, atom: Atom<'e>, span: Span) -> Self::Item {
		let _ = span;
		match atom {
			Atom::Key(key) => Edn::Key(key),
			Atom::Symbol(symbol) => Edn::Symbol(symbol),
			Atom::Str(str) => Edn::Str(str),
			Atom::Int(int) => Edn::Int(int),
			#[cfg(feature = "floats")]
			Atom::Double(double) => Edn::Double(double),
			Atom::Rational(rational) => Edn::Rational(rational),
			#[cfg(feature = "arbitrary-nums")]
			Atom::BigInt(big_int) => Edn::BigInt(big_int),
			#[cfg(feature = "arbitrary-nums")]
			Atom::BigDec(big_dec) => Edn::BigDec(big_dec),
			Atom::Char(ch) => Edn::Char(ch),
			Atom::Bool(bool) => Edn::Bool(bool),
			Atom::Nil => Edn::Nil,
		}
	}

	fn with_leading_discards(
		&self,
		item: Self::Item,
		_leading_discards: Vec<Self::Discard>,
	) -> Self::Item {
		item
	}

	fn new_vector_context(&self) -> Self::VectorContext {
		Vec::new()
	}

	fn new_list_context(&self) -> Self::ListContext {
		Vec::new()
	}

	fn new_map_context(&self) -> Self::MapContext {
		(Vec::new(), None)
	}

	fn new_set_context(&self) -> Self::SetContext {
		Vec::new()
	}

	fn add_to_vector(
		&self,
		ctx: &mut Self::VectorContext,
		parsed: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(parsed.item);
		Ok(())
	}

	fn add_to_list(
		&self,
		ctx: &mut Self::ListContext,
		parsed: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(parsed.item);
		Ok(())
	}

	fn add_to_map(
		&self,
		ctx: &mut Self::MapContext,
		parsed: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		let (entries, pending) = ctx;
		if let Some(key) = pending.take() {
			entries.push((key, parsed));
		} else {
			*pending = Some(parsed);
		}
		Ok(())
	}

	fn add_to_set(
		&self,
		ctx: &mut Self::SetContext,
		parsed: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(parsed);
		Ok(())
	}

	fn finish_vector(
		&self,
		ctx: Self::VectorContext,
		_trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		Ok(Parsed::new(Edn::Vector(ctx), span))
	}

	fn finish_set(
		&self,
		ctx: Self::SetContext,
		_trailing_discards: Vec<Self::Discard>,
		validate: bool,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		let mut set = BTreeSet::new();
		for item in ctx {
			if !set.insert(item.item) && validate {
				return Err(Error::from_position(Code::SetDuplicateKey, item.span.1));
			}
		}
		Ok(Parsed::new(Edn::Set(set), span))
	}

	fn finish_map(
		&self,
		ctx: Self::MapContext,
		_trailing_discards: Vec<Self::Discard>,
		validate: bool,
		close_pos: Position,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		if ctx.1.is_some() {
			return Err(Error::from_position(Code::UnexpectedEOF, close_pos));
		}
		let mut map = BTreeMap::new();
		for (key, value) in ctx.0 {
			if map.insert(key.item, value.item).is_some() && validate {
				return Err(Error::from_position(Code::HashMapDuplicateKey, value.span.1));
			}
		}
		Ok(Parsed::new(Edn::Map(map), span))
	}

	fn finish_list(
		&self,
		ctx: Self::ListContext,
		_trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		Ok(Parsed::new(Edn::List(ctx), span))
	}

	fn tag(
		&self,
		tag: &'e str,
		_tag_span: Span,
		value: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
		span: Span,
	) -> Parsed<Self::Item> {
		Parsed::new(Edn::Tagged(tag, Box::new(value.item)), span)
	}

	fn discard(
		&self,
		_value: Parsed<Self::Item>,
		_leading_discards: Vec<Self::Discard>,
		_discard_span: Span,
	) -> Self::Discard {
	}
}

struct NodeBuilder;

impl<'e> InternalParser<'e> for NodeBuilder {
	type Item = Node<'e>;
	type Discard = Discard<'e>;
	type VectorContext = Vec<Node<'e>>;
	type ListContext = Vec<Node<'e>>;
	type MapContext = (Vec<(Node<'e>, Node<'e>)>, Option<Node<'e>>);
	type SetContext = Vec<Node<'e>>;

	fn atom(&self, atom: Atom<'e>, span: Span) -> Self::Item {
		let kind = match atom {
			Atom::Key(key) => NodeKind::Key(key),
			Atom::Symbol(symbol) => NodeKind::Symbol(symbol),
			Atom::Str(str) => NodeKind::Str(str),
			Atom::Int(int) => NodeKind::Int(int),
			#[cfg(feature = "floats")]
			Atom::Double(double) => NodeKind::Double(double),
			Atom::Rational(rational) => NodeKind::Rational(rational),
			#[cfg(feature = "arbitrary-nums")]
			Atom::BigInt(big_int) => NodeKind::BigInt(big_int),
			#[cfg(feature = "arbitrary-nums")]
			Atom::BigDec(big_dec) => NodeKind::BigDec(big_dec),
			Atom::Char(ch) => NodeKind::Char(ch),
			Atom::Bool(bool) => NodeKind::Bool(bool),
			Atom::Nil => NodeKind::Nil,
		};

		Node::no_discards(kind, span)
	}

	fn with_leading_discards(
		&self,
		mut item: Self::Item,
		leading_discards: Vec<Self::Discard>,
	) -> Self::Item {
		item.leading_discards = leading_discards;
		item
	}

	fn new_vector_context(&self) -> Self::VectorContext {
		Vec::new()
	}

	fn new_list_context(&self) -> Self::ListContext {
		Vec::new()
	}

	fn new_map_context(&self) -> Self::MapContext {
		(Vec::new(), None)
	}

	fn new_set_context(&self) -> Self::SetContext {
		Vec::new()
	}

	fn add_to_vector(
		&self,
		ctx: &mut Self::VectorContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(self.with_leading_discards(parsed.item, leading_discards));
		Ok(())
	}

	fn add_to_list(
		&self,
		ctx: &mut Self::ListContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(self.with_leading_discards(parsed.item, leading_discards));
		Ok(())
	}

	fn add_to_map(
		&self,
		ctx: &mut Self::MapContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		let parsed = self.with_leading_discards(parsed.item, leading_discards);
		if let Some(key) = ctx.1.take() {
			ctx.0.push((key, parsed));
		} else {
			ctx.1 = Some(parsed);
		}
		Ok(())
	}

	fn add_to_set(
		&self,
		ctx: &mut Self::SetContext,
		parsed: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
	) -> Result<(), Error> {
		ctx.push(self.with_leading_discards(parsed.item, leading_discards));
		Ok(())
	}

	fn finish_vector(
		&self,
		ctx: Self::VectorContext,
		trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		Ok(Parsed::new(Node::no_discards(NodeKind::Vector(ctx, trailing_discards), span), span))
	}

	fn finish_set(
		&self,
		ctx: Self::SetContext,
		trailing_discards: Vec<Self::Discard>,
		_validate: bool,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		Ok(Parsed::new(Node::no_discards(NodeKind::Set(ctx, trailing_discards), span), span))
	}

	fn finish_map(
		&self,
		ctx: Self::MapContext,
		trailing_discards: Vec<Self::Discard>,
		_validate: bool,
		close_pos: Position,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		if ctx.1.is_some() {
			return Err(Error::from_position(Code::UnexpectedEOF, close_pos));
		}
		Ok(Parsed::new(Node::no_discards(NodeKind::Map(ctx.0, trailing_discards), span), span))
	}

	fn finish_list(
		&self,
		ctx: Self::ListContext,
		trailing_discards: Vec<Self::Discard>,
		span: Span,
	) -> Result<Parsed<Self::Item>, Error> {
		Ok(Parsed::new(Node::no_discards(NodeKind::List(ctx, trailing_discards), span), span))
	}

	fn tag(
		&self,
		tag: &'e str,
		tag_span: Span,
		value: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
		span: Span,
	) -> Parsed<Self::Item> {
		let value = self.with_leading_discards(value.item, leading_discards);
		Parsed::new(Node::no_discards(NodeKind::Tagged(tag, tag_span, Box::new(value)), span), span)
	}

	fn discard(
		&self,
		value: Parsed<Self::Item>,
		leading_discards: Vec<Self::Discard>,
		discard_span: Span,
	) -> Self::Discard {
		Discard(self.with_leading_discards(value.item, leading_discards), discard_span)
	}
}

fn parse_with<'r, 'e: 'r, B: InternalParser<'e>>(
	reader: &'r mut SourceReader<'e>,
	builder: &B,
) -> Result<B::Item, Error> {
	let start_pos = reader.read_pos;
	let mut walker = Walker::new(reader);
	Ok(
		parse_internal(&mut walker, builder)?
			.unwrap_or_else(|| builder.nil(walker.reader.span_from(start_pos))),
	)
}

#[expect(clippy::mem_replace_with_default)]
const fn take_discards<D>(discards: &mut Vec<D>) -> Vec<D> {
	replace(discards, Vec::new())
}

#[inline]
fn add_to_context<'e, B: InternalParser<'e>>(
	context: &mut Option<&mut ParseContext<'e, B>>,
	builder: &B,
	parsed: Parsed<B::Item>,
) -> Result<(), Error> {
	match context.as_mut() {
		Some(ParseContext { kind: ContextKind::Vector(ctx, _), discards }) => {
			builder.add_to_vector(ctx, parsed, take_discards(discards))?;
		}
		Some(ParseContext { kind: ContextKind::List(ctx, _), discards }) => {
			builder.add_to_list(ctx, parsed, take_discards(discards))?;
		}
		Some(ParseContext { kind: ContextKind::Map(ctx, _), discards }) => {
			builder.add_to_map(ctx, parsed, take_discards(discards))?;
		}
		Some(ParseContext { kind: ContextKind::Set(ctx, _), discards }) => {
			builder.add_to_set(ctx, parsed, take_discards(discards))?;
		}
		_ => {}
	}
	Ok(())
}

#[inline]
fn handle_open_delimiter<'e, B: InternalParser<'e>>(
	walker: &mut Walker<'e, '_, B>,
	builder: &B,
	delim: OpenDelimiter,
) -> Result<(), Error> {
	let pos_start = walker.pos();
	match delim {
		OpenDelimiter::Vector => {
			let _ = walker.reader.nibble_next();
			walker.push_context(ParseContext::no_discards(ContextKind::Vector(
				builder.new_vector_context(),
				pos_start,
			)));
		}
		OpenDelimiter::List => {
			let _ = walker.reader.nibble_next();
			walker.push_context(ParseContext::no_discards(ContextKind::List(
				builder.new_list_context(),
				pos_start,
			)));
		}
		OpenDelimiter::Map => {
			let _ = walker.reader.nibble_next();
			walker.push_context(ParseContext::no_discards(ContextKind::Map(
				builder.new_map_context(),
				pos_start,
			)));
		}
		OpenDelimiter::Hash => {
			let _ = walker.reader.nibble_next();
			match walker.reader.peek_next() {
				Some('{') => {
					let _ = walker.reader.nibble_next();
					walker.push_context(ParseContext::no_discards(ContextKind::Set(
						builder.new_set_context(),
						pos_start,
					)));
				}
				Some('_') => {
					let _ = walker.reader.nibble_next();
					walker.push_context(ParseContext::no_discards(ContextKind::Discard(pos_start)));
				}
				_ => {
					let tag_pos_start = walker.pos();
					let tag = walker.reader.slurp_tag()?;
					let tag_span = walker.span_from(tag_pos_start);

					walker.reader.nibble_whitespace();
					walker
						.push_context(ParseContext::no_discards(ContextKind::Tag(tag, tag_span, pos_start)));
				}
			}
		}
	}
	Ok(())
}

fn wrap_pending_tags<'e, 'r, B: InternalParser<'e>>(
	walker: &mut Walker<'e, 'r, B>,
	builder: &B,
	mut parsed: Parsed<B::Item>,
) -> Parsed<B::Item> {
	while matches!(walker.stack.last(), Some(ParseContext { kind: ContextKind::Tag(..), .. })) {
		let ParseContext { kind: ContextKind::Tag(tag, tag_span, pos_start), discards } =
			walker.pop_context().expect("tag context should exist")
		else {
			unreachable!("tag context should be on top of the stack");
		};

		parsed = builder.tag(tag, tag_span, parsed, discards, walker.span_from(pos_start));
	}

	parsed
}

fn under_discard<'e, B: InternalParser<'e>>(walker: &Walker<'e, '_, B>) -> bool {
	walker.stack.iter().any(|ctx| matches!(ctx.kind, ContextKind::Discard(..)))
}

fn complete_value<'e, 'r, B: InternalParser<'e>>(
	walker: &mut Walker<'e, 'r, B>,
	builder: &B,
	parsed: Parsed<B::Item>,
) -> Result<Option<B::Item>, Error> {
	let parsed = wrap_pending_tags(walker, builder, parsed);

	if walker.stack_len() == 1 {
		let leading_discards =
			take_discards(walker.last_context_discards().expect("Top should be there"));
		return Ok(Some(builder.with_leading_discards(parsed.item, leading_discards)));
	}

	if let Some(ParseContext { kind: ContextKind::Discard(pos_start), discards }) =
		walker.stack.last_mut()
	{
		let pos_start = *pos_start;
		let end_pos = parsed.span.1;
		let discarded = builder.discard(parsed, take_discards(discards), Span(pos_start, end_pos));
		walker.pop_context();
		walker.stack.last_mut().expect("Top should be there").discards.push(discarded);
		return Ok(None);
	}

	add_to_context(&mut walker.stack.last_mut(), builder, parsed)?;
	Ok(None)
}

#[inline]
fn handle_close_delimiter<'e, B: InternalParser<'e>>(
	walker: &mut Walker<'e, '_, B>,
	builder: &B,
	delimiter: char,
) -> Result<Option<B::Item>, Error> {
	if walker.stack_len() <= 1 {
		return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
	}

	let expected = match walker.stack.last().expect("Len > 1 is never empty") {
		ParseContext { kind: ContextKind::Vector(..), .. } => ']',
		ParseContext { kind: ContextKind::List(..), .. } => ')',
		ParseContext { kind: ContextKind::Map(..) | ContextKind::Set(..), .. } => '}',
		_ => {
			return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
		}
	};

	if delimiter != expected {
		return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
	}

	let parsed = match walker.pop_context() {
		Some(ParseContext { kind: ContextKind::Vector(ctx, pos_start), discards }) => {
			let _ = walker.reader.nibble_next();
			builder.finish_vector(ctx, discards, walker.span_from(pos_start))?
		}
		Some(ParseContext { kind: ContextKind::List(ctx, pos_start), discards }) => {
			let _ = walker.reader.nibble_next();
			builder.finish_list(ctx, discards, walker.span_from(pos_start))?
		}
		Some(ParseContext { kind: ContextKind::Map(ctx, pos_start), discards }) => {
			let validate = !under_discard(walker);
			let close_pos = walker.pos();
			let _ = walker.reader.nibble_next();
			builder.finish_map(ctx, discards, validate, close_pos, walker.span_from(pos_start))?
		}
		Some(ParseContext { kind: ContextKind::Set(ctx, pos_start), discards }) => {
			let validate = !under_discard(walker);
			let _ = walker.reader.nibble_next();
			builder.finish_set(ctx, discards, validate, walker.span_from(pos_start))?
		}
		_ => {
			return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
		}
	};

	complete_value(walker, builder, parsed)
}

fn parse_internal<'e, B: InternalParser<'e>>(
	walker: &mut Walker<'e, '_, B>,
	builder: &B,
) -> Result<Option<B::Item>, Error> {
	let mut result = None;
	loop {
		walker.reader.nibble_whitespace();
		match walker.reader.peek_next() {
			Some(';') => walker.reader.nibble_newline(),
			Some('[') => handle_open_delimiter(walker, builder, OpenDelimiter::Vector)?,
			Some('(') => handle_open_delimiter(walker, builder, OpenDelimiter::List)?,
			Some('{') => handle_open_delimiter(walker, builder, OpenDelimiter::Map)?,
			Some('#') => handle_open_delimiter(walker, builder, OpenDelimiter::Hash)?,
			Some(d) if matches!(d, ']' | ')' | '}') => {
				if let Some(parsed) = handle_close_delimiter(walker, builder, d)? {
					result = Some(parsed);
					break;
				}
			}
			Some(c) => {
				let pos_start = walker.reader.read_pos;
				let atom = match c {
					'\\' => parse_char(walker.reader.slurp_char()).map(Atom::Char),
					'"' => Ok(Atom::Str(walker.reader.slurp_str()?)),
					_ => edn_literal(walker.reader.slurp_literal()),
				}
				.map_err(|code| Error::from_position(code, pos_start))?;
				let span = walker.reader.span_from(pos_start);
				let parsed = Parsed::new(builder.atom(atom, span), span);

				if let Some(parsed) = complete_value(walker, builder, parsed)? {
					result = Some(parsed);
					break;
				}
			}
			None => {
				if walker.stack_len() > 1 {
					return Err(walker.make_error(Code::UnexpectedEOF));
				}
				break;
			}
		}
	}
	Ok(result)
}

#[inline]
fn edn_literal(literal: &str) -> Result<Atom<'_>, Code> {
	fn numeric(s: &str) -> bool {
		let (first, second) = {
			let mut s = s.chars();
			(s.next(), s.next())
		};

		let first = first.expect("Empty str is previously caught as nil");
		if first.is_numeric() {
			return true;
		}

		if (first == '-' || first == '+')
			&& let Some(s) = second
			&& s.is_numeric()
		{
			return true;
		}

		false
	}

	Ok(match literal {
		"nil" => Atom::Nil,
		"true" => Atom::Bool(true),
		"false" => Atom::Bool(false),
		k if k.starts_with(':') => {
			if k.len() <= 1 {
				return Err(Code::InvalidKeyword);
			}
			Atom::Key(&k[1..])
		}
		n if numeric(n) => parse_number(n)?,
		_ => Atom::Symbol(literal),
	})
}

#[inline]
fn parse_char(lit: &str) -> Result<char, Code> {
	let lit = &lit[1..]; // ignore the leading '\\'
	match lit {
		"newline" => Ok('\n'),
		"return" => Ok('\r'),
		"tab" => Ok('\t'),
		"space" => Ok(' '),
		c if c.len() == 1 => Ok(c.chars().next().expect("c must be len of 1")),
		_ => Err(Code::InvalidChar),
	}
}

#[inline]
fn parse_number(lit: &str) -> Result<Atom<'_>, Code> {
	let mut chars = lit.chars().peekable();
	let (number, radix, polarity) = {
		let mut num_ptr_start = 0;
		let polarity = chars.peek().map_or(1i8, |c| {
			if *c == '-' {
				num_ptr_start += 1;
				-1i8
			} else if *c == '+' {
				// The EDN spec allows for a redundant '+' symbol, we just ignore it.
				num_ptr_start += 1;
				1i8
			} else {
				1i8
			}
		});

		let mut number = &lit[num_ptr_start..];

		if number.to_lowercase().starts_with("0x") {
			number = &number[2..];
			(number, 16, polarity)
		} else if let Some(index) = number.to_lowercase().find('r') {
			let radix = (number[0..index]).parse::<u8>();

			match radix {
				Ok(r) => {
					// from_str_radix panics if radix is not in the range from 2 to 36
					if !(2..=36).contains(&r) {
						return Err(Code::InvalidRadix(Some(r)));
					}

					number = &number[(index + 1)..];
					(number, r, polarity)
				}
				Err(_) => {
					return Err(Code::InvalidRadix(None));
				}
			}
		} else {
			(number, 10, polarity)
		}
	};

	if let Ok(n) = i64::from_str_radix(number, radix.into()) {
		return Ok(Atom::Int(n * i64::from(polarity)));
	}
	if radix == 10
		&& let Some(index) = number.find('/')
	{
		let (num, den) = number.split_at(index);
		let num = num.parse::<i64>();
		let den = den[1..].parse::<i64>();

		if let (Ok(n), Ok(d)) = (num, den) {
			return Ok(Atom::Rational((n * i64::from(polarity), d)));
		}
	}

	#[cfg(feature = "arbitrary-nums")]
	if let Some(n) = big_int_from_slice(number, radix, polarity) {
		return Ok(Atom::BigInt(n));
	}
	#[cfg(feature = "floats")]
	if radix == 10
		&& let Ok(n) = number.parse::<f64>()
	{
		return Ok(Atom::Double((n * f64::from(polarity)).into()));
	}
	#[cfg(feature = "arbitrary-nums")]
	if let Some(n) = big_dec_from_slice(number, radix, polarity) {
		return Ok(Atom::BigDec(n));
	}

	Err(Code::InvalidNumber)
}

#[inline]
#[cfg(feature = "arbitrary-nums")]
fn big_int_from_slice(slice: &str, radix: u8, polarity: i8) -> Option<num_bigint::BigInt> {
	// strip ending N, if it exists
	let slice = slice.strip_suffix('N').map_or(slice, |slice| slice);
	let num = num_bigint::BigInt::parse_bytes(slice.as_bytes(), radix.into())?;
	Some(num * polarity)
}

#[inline]
#[cfg(feature = "arbitrary-nums")]
fn big_dec_from_slice(slice: &str, radix: u8, polarity: i8) -> Option<bigdecimal::BigDecimal> {
	// strip ending M, if it exists
	let slice = slice.strip_suffix('M').map_or(slice, |slice| slice);
	let num = bigdecimal::BigDecimal::parse_bytes(slice.as_bytes(), radix.into())?;
	Some(num * polarity)
}
