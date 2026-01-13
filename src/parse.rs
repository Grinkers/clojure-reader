//! An EDN syntax parser in Rust.
#![expect(clippy::inline_always)]

use alloc::boxed::Box;
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
/// **NOTE:** The vector of items in [`Node::Set`] may contain duplicate items
/// **NOTE:** The vector of entries in [`Node::Map`] may contain entries with duplicate keys
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
  Tagged(&'e str, Box<Node<'e>>),
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
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Discard<'e>(pub Node<'e>, pub Span);

/// Concrete EDN syntax-tree
///
/// Once [`parsed`](parse), it can then be converted to an [`Edn`] via [`TryFrom`](Edn::try_from)
///
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node<'e> {
  pub kind: NodeKind<'e>,
  pub span: Span,
  pub leading_discards: Vec<Discard<'e>>,
}

// Node without discards
type SpannedNode<'e> = (NodeKind<'e>, Span);

impl Node<'_> {
  #[inline]
  pub const fn span(&self) -> Span {
    self.span
  }
}

impl<'e> Node<'e> {
  /// Construct a `node` with its kind and span but having no leading-discards
  pub const fn no_discards(kind: NodeKind<'e>, span: Span) -> Self {
    Self { kind, span, leading_discards: Vec::new() }
  }
}

/// Parse a single `Node` from a [`SourceReader`] without consuming it
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
pub fn parse<'r, 'e: 'r>(reader: &'r mut SourceReader<'e>) -> Result<Node<'e>, Error> {
  let start_pos = reader.read_pos;
  let mut walker = Walker::new(reader);
  let internal_parse = parse_internal(&mut walker)?;
  Ok(
    internal_parse
      .unwrap_or_else(|| Node::no_discards(NodeKind::Nil, walker.reader.span_from(start_pos))),
  )
}

/// Parses an [`Edn`] by first parsing a [`Node`], and then fallibly converting it
///
/// # Errors
///
/// See [`crate::error::Error`].
pub fn parse_as_edn(edn: &str) -> Result<(Edn<'_>, &str), Error> {
  let mut source_reader = SourceReader::new(edn);
  let parsed = parse(&mut source_reader)?;
  let span = parsed.span();
  Ok((parsed.try_into()?, &edn[span.1.ptr..]))
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

#[derive(Debug)]
struct Walker<'e, 'r> {
  reader: &'r mut SourceReader<'e>,
  stack: Vec<ParseContext<'e>>,
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
  #[cfg_attr(not(feature = "unstable"), expect(dead_code))]
  // ^ Since this is private when `unstable` isn't enabled
  pub fn remaining(&self) -> &'e str {
    &self.slice[self.read_pos.ptr..]
  }

  // Slurps until whitespace or delimiter, returning the slice.
  #[inline(always)]
  fn slurp_literal(&mut self) -> &'e str {
    let token = self.slice[self.read_pos.ptr..]
      .split(|c: char| c.is_whitespace() || DELIMITERS.contains(&c) || c == '"')
      .next()
      .expect("Expected at least an empty slice");

    self.read_pos.ptr += token.len();
    self.read_pos.column += token.len();
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
            't' | 'r' | 'n' | '\\' | '\"' => (),
            _ => {
              return Err(Error::from_position(Code::InvalidEscape, self.read_pos));
            }
          }
          escape = false;
        } else if c == '\"' {
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

impl<'e, 'r> Walker<'e, 'r> {
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
  fn push_context(&mut self, ctx: ParseContext<'e>) {
    self.stack.push(ctx);
  }

  #[inline(always)]
  fn pop_context(&mut self) -> Option<ParseContext<'e>> {
    self.stack.pop()
  }

  #[inline(always)]
  const fn stack_len(&self) -> usize {
    self.stack.len()
  }

  const fn make_error(&self, code: Code) -> Error {
    Error::from_position(code, self.pos())
  }

  fn last_context_discards(&mut self) -> Option<&mut Vec<Discard<'e>>> {
    match self.stack.last_mut() {
      Some(ParseContext { kind: _, discards }) => Some(discards),
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

// `Postion`, wherever present, contains the start position of that context
#[derive(Debug)]
enum ContextKind<'e> {
  Top,
  Vector(Vec<Node<'e>>, Position),
  List(Vec<Node<'e>>, Position),
  Map(Vec<(Node<'e>, Node<'e>)>, Option<Node<'e>>, Position),
  Set(Vec<Node<'e>>, Position),
  Tag(&'e str, Position),
  Discard(Position),
}

#[derive(Debug)]
struct ParseContext<'e> {
  kind: ContextKind<'e>,
  discards: Vec<Discard<'e>>,
}

impl<'e> ParseContext<'e> {
  const fn no_discards(kind: ContextKind<'e>) -> Self {
    Self { kind, discards: Vec::new() }
  }
}

#[inline]
fn parse_element<'e>(reader: &mut SourceReader<'e>, next: char) -> Result<SpannedNode<'e>, Error> {
  let pos_start = reader.read_pos;
  match next {
    '\\' => match parse_char(reader.slurp_char()) {
      Ok(node) => Ok((NodeKind::Char(node), reader.span_from(pos_start))),
      Err(code) => Err(Error::from_position(code, pos_start)),
    },
    '\"' => {
      let str = reader.slurp_str()?;
      Ok((NodeKind::Str(str), reader.span_from(pos_start)))
    }
    _ => {
      let lit = reader.slurp_literal();
      match edn_literal(lit, reader.span_from(pos_start)) {
        Ok(node) => Ok(node),
        Err(code) => Err(Error::from_position(code, pos_start)),
      }
    }
  }
}

#[expect(clippy::mem_replace_with_default)]
const fn take_discards<'e>(discards: &mut Vec<Discard<'e>>) -> Vec<Discard<'e>> {
  replace(discards, Vec::new())
}

#[inline]
fn add_to_context<'e>(context: &mut Option<&mut ParseContext<'e>>, (kind, span): SpannedNode<'e>) {
  match context.as_mut() {
    Some(ParseContext {
      kind: ContextKind::Vector(vec, _) | ContextKind::List(vec, _) | ContextKind::Set(vec, _),
      discards,
    }) => {
      let leading_discards = take_discards(discards);
      vec.push(Node { kind, span, leading_discards });
    }
    Some(ParseContext { kind: ContextKind::Map(map, pending, _), discards }) => {
      let leading_discards = take_discards(discards);
      let node = Node { kind, span, leading_discards };
      if let Some(key) = pending.take() {
        map.push((key, node));
      } else {
        *pending = Some(node);
      }
    }
    _ => {} // Do nothing. Errors will bubble up elsewhere.
  }
}

#[inline]
fn handle_open_delimiter(walker: &mut Walker<'_, '_>, delim: OpenDelimiter) -> Result<(), Error> {
  let pos_start = walker.pos();
  match delim {
    OpenDelimiter::Vector => {
      let _ = walker.reader.nibble_next();
      walker.push_context(ParseContext::no_discards(ContextKind::Vector(Vec::new(), pos_start)));
    }
    OpenDelimiter::List => {
      let _ = walker.reader.nibble_next();
      walker.push_context(ParseContext::no_discards(ContextKind::List(Vec::new(), pos_start)));
    }
    OpenDelimiter::Map => {
      let _ = walker.reader.nibble_next();
      walker.push_context(ParseContext::no_discards(ContextKind::Map(Vec::new(), None, pos_start)));
    }
    OpenDelimiter::Hash => {
      let _ = walker.reader.nibble_next();
      match walker.reader.peek_next() {
        Some('{') => {
          let _ = walker.reader.nibble_next();
          walker.push_context(ParseContext::no_discards(ContextKind::Set(Vec::new(), pos_start)));
        }
        Some('_') => {
          let _ = walker.reader.nibble_next();
          walker.push_context(ParseContext::no_discards(ContextKind::Discard(pos_start)));
        }
        _ => {
          let tag = walker.reader.slurp_tag()?;
          walker.reader.nibble_whitespace();
          walker.push_context(ParseContext::no_discards(ContextKind::Tag(tag, pos_start)));
        }
      }
    }
  }
  Ok(())
}

#[inline]
fn handle_close_delimiter<'e>(
  walker: &mut Walker<'e, '_>,
  delimiter: char,
) -> Result<Option<(NodeKind<'e>, Span)>, Error> {
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
  let (mut node, mut span) = match walker.pop_context() {
    Some(ParseContext { kind: ContextKind::Vector(vec, pos_start), discards }) => {
      let _ = walker.reader.nibble_next();
      (NodeKind::Vector(vec, discards), walker.span_from(pos_start))
    }
    Some(ParseContext { kind: ContextKind::List(vec, pos_start), discards }) => {
      let _ = walker.reader.nibble_next();
      (NodeKind::List(vec, discards), walker.span_from(pos_start))
    }
    Some(ParseContext { kind: ContextKind::Map(map, pending, pos_start), discards }) => {
      if pending.is_some() {
        return Err(walker.make_error(Code::UnexpectedEOF));
      }
      let _ = walker.reader.nibble_next();
      (NodeKind::Map(map, discards), walker.span_from(pos_start))
    }
    Some(ParseContext { kind: ContextKind::Set(set, pos_start), discards }) => {
      let _ = walker.reader.nibble_next();
      (NodeKind::Set(set, discards), walker.span_from(pos_start))
    }
    _ => {
      // this should be impossible, due to checking for unmatched delimiters above
      return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
    }
  };

  if walker.stack_len() == 1 {
    return Ok(Some((node, span)));
  }
  while let Some(context) = walker.pop_context() {
    match context {
      ParseContext { kind: ContextKind::Tag(t, pos_start), discards } => {
        node = NodeKind::Tagged(t, Box::new(Node { kind: node, span, leading_discards: discards }));
        span = walker.span_from(pos_start);
      }
      other => {
        walker.push_context(other);
        break;
      }
    }
  }

  if walker.stack_len() == 1 {
    return Ok(Some((node, span)));
  } else if let Some(ParseContext { kind: ContextKind::Discard(pos_start), discards }) =
    walker.stack.last_mut()
  {
    let pos_start = *pos_start;
    let leading_discards = take_discards(discards);
    let discarded =
      Discard(Node { kind: node, span, leading_discards }, walker.span_from(pos_start));
    walker.pop_context();
    walker.stack.last_mut().expect("Top should be there").discards.push(discarded);
  } else {
    add_to_context(&mut walker.stack.last_mut(), (node, span));
  }
  Ok(None)
}

#[inline]
fn handle_element<'e>(walker: &mut Walker<'e, '_>, next: char) -> Result<Option<Node<'e>>, Error> {
  let (node, span) = parse_element(walker.reader, next)?;
  if walker.stack_len() == 1 {
    let last_ctx = walker.stack.last_mut().expect("stack_len() == 1");
    let node = Node { kind: node, span, leading_discards: take_discards(&mut last_ctx.discards) };
    return Ok(Some(node));
  }
  let (node, span) = match walker.stack.last_mut() {
    Some(ParseContext { kind: ContextKind::Tag(tag, pos_start), discards }) => {
      let pos_start = *pos_start;
      let leading_discards = take_discards(discards);
      let mut node = NodeKind::Tagged(tag, Box::new(Node { kind: node, span, leading_discards }));
      let mut span = walker.span_from(pos_start);

      walker.pop_context();
      while let Some(ParseContext { kind: ContextKind::Tag(t, pos_start), discards }) =
        walker.stack.last_mut()
      {
        let pos_start = *pos_start;
        let leading_discards = take_discards(discards);
        node = NodeKind::Tagged(t, Box::new(Node { kind: node, span, leading_discards }));
        span = walker.span_from(pos_start);
        walker.pop_context();
      }
      if walker.stack_len() == 1 {
        let last_ctx = walker.stack.last_mut().expect("stack_len() == 1");
        let node =
          Node { kind: node, span, leading_discards: take_discards(&mut last_ctx.discards) };
        return Ok(Some(node));
      }
      (node, span)
    }
    Some(ParseContext { kind: ContextKind::Discard(pos_start), discards }) => {
      let pos_start = *pos_start;
      let leading_discards = take_discards(discards);
      let discarded =
        Discard(Node { kind: node, span, leading_discards }, walker.span_from(pos_start));
      walker.pop_context();
      walker.stack.last_mut().expect("Top should be there").discards.push(discarded);
      return Ok(None);
    }
    _ => (node, span),
  };
  add_to_context(&mut walker.stack.last_mut(), (node, span));
  Ok(None)
}

fn parse_internal<'e>(walker: &mut Walker<'e, '_>) -> Result<Option<Node<'e>>, Error> {
  let mut result: Option<Node<'e>> = None;
  loop {
    walker.reader.nibble_whitespace();
    match walker.reader.peek_next() {
      Some(';') => walker.reader.nibble_newline(),
      Some('[') => handle_open_delimiter(walker, OpenDelimiter::Vector)?,
      Some('(') => handle_open_delimiter(walker, OpenDelimiter::List)?,
      Some('{') => handle_open_delimiter(walker, OpenDelimiter::Map)?,
      Some('#') => handle_open_delimiter(walker, OpenDelimiter::Hash)?,
      Some(d) if matches!(d, ']' | ')' | '}') => {
        if let Some((kind, span)) = handle_close_delimiter(walker, d)? {
          result = Some(Node {
            kind,
            span,
            leading_discards: take_discards(
              walker.last_context_discards().expect("Top should be there"),
            ),
          });
          break;
        }
      }
      Some(c) => {
        if let Some(node) = handle_element(walker, c)? {
          result = Some(node);
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
fn edn_literal(literal: &str, span: Span) -> Result<SpannedNode<'_>, Code> {
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
    "nil" => (NodeKind::Nil, span),
    "true" => (NodeKind::Bool(true), span),
    "false" => (NodeKind::Bool(false), span),
    k if k.starts_with(':') => {
      if k.len() <= 1 {
        return Err(Code::InvalidKeyword);
      }
      (NodeKind::Key(&k[1..]), span)
    }
    n if numeric(n) => parse_number(n, span)?,
    _ => (NodeKind::Symbol(literal), span),
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
fn parse_number(lit: &str, span: Span) -> Result<SpannedNode<'_>, Code> {
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
    return Ok((NodeKind::Int(n * i64::from(polarity)), span));
  }
  if radix == 10
    && let Some((n, d)) = num_den_from_slice(number, polarity)
  {
    return Ok((NodeKind::Rational((n, d)), span));
  }

  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_int_from_slice(number, radix, polarity) {
    return Ok((NodeKind::BigInt(n), span));
  }
  #[cfg(feature = "floats")]
  if radix == 10
    && let Ok(n) = number.parse::<f64>()
  {
    return Ok((NodeKind::Double((n * f64::from(polarity)).into()), span));
  }
  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_dec_from_slice(number, radix, polarity) {
    return Ok((NodeKind::BigDec(n), span));
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

#[inline]
fn num_den_from_slice(slice: &str, polarity: i8) -> Option<(i64, i64)> {
  let index = slice.find('/');

  if let Some(i) = index {
    let (num, den) = slice.split_at(i);
    let num = num.parse::<i64>();
    let den = den[1..].parse::<i64>();

    if let (Ok(n), Ok(d)) = (num, den) {
      return Some((n * i64::from(polarity), d));
    }
  }
  None
}
