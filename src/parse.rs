#![expect(clippy::inline_always)]

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::primitive::str;

use crate::edn::Edn;
use crate::error::{Code, Error};

#[cfg(feature = "arbitrary-nums")]
use bigdecimal::BigDecimal;
#[cfg(feature = "arbitrary-nums")]
use num_bigint::BigInt;
#[cfg(feature = "floats")]
use ordered_float::OrderedFloat;

/// Concrete Clojure syntax-tree with span data associated with each kind of node.
///
/// Once [`parsed`](parse), it can then be converted to an [`Edn`](Edn::try_from)
///
/// **NOTE:** The vector of items in [`Node::Set`] may contain duplicate items
/// **NOTE:** The vector of entires in [`Node::Map`] may contain entries with duplicate keys
#[derive(Debug)]
pub enum Node<'e> {
  Vector(Vec<Self>, Span),
  Set(Vec<Self>, Span),
  Map(Vec<(Self, Self)>, Span),
  List(Vec<Self>, Span),
  Key(&'e str, Span),
  Symbol(&'e str, Span),
  Str(&'e str, Span),
  Int(i64, Span),
  Tagged(&'e str, Box<Self>, Span),
  #[cfg(feature = "floats")]
  Double(OrderedFloat<f64>, Span),
  Rational((i64, i64), Span),
  #[cfg(feature = "arbitrary-nums")]
  BigInt(BigInt, Span),
  #[cfg(feature = "arbitrary-nums")]
  BigDec(BigDecimal, Span),
  Char(char, Span),
  Bool(bool, Span),
  Nil(Span),
}

impl Node<'_> {
  pub const fn span(&self) -> Span {
    *match self {
      Node::Vector(_, s) => s,
      Node::Set(_, s) => s,
      Node::Map(_, s) => s,
      Node::List(_, s) => s,
      Node::Key(_, s) => s,
      Node::Symbol(_, s) => s,
      Node::Str(_, s) => s,
      Node::Int(_, s) => s,
      Node::Tagged(_, _, s) => s,
      #[cfg(feature = "floats")]
      Node::Double(_, s) => s,
      Node::Rational(_, s) => s,
      #[cfg(feature = "arbitrary-nums")]
      Node::BigInt(_, s) => s,
      #[cfg(feature = "arbitrary-nums")]
      Node::BigDec(_, s) => s,
      Node::Char(_, s) => s,
      Node::Bool(_, s) => s,
      Node::Nil(s) => s,
    }
  }
}

pub fn parse<'r, 'e: 'r>(reader: &'r mut SourceReader<'e>) -> Result<Node<'e>, Error> {
  let start_pos = reader.read_pos;
  let mut walker = Walker::new(reader);
  let internal_parse = parse_internal(&mut walker)?;
  Ok(internal_parse.unwrap_or_else(|| Node::Nil(walker.reader.span_from(start_pos))))
}

pub fn parse_as_edn(edn: &'_ str) -> Result<(Edn<'_>, &'_ str), Error> {
  let mut source_reader = SourceReader::new(edn);
  let parsed = parse(&mut source_reader)?;
  let span = parsed.span();
  Ok((parsed.try_into()?, &edn[span.1.ptr..]))
}

const DELIMITERS: [char; 8] = [',', ']', '}', ')', ';', '(', '[', '{'];

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Default, Clone, Copy)]
pub struct Span(pub Position, pub Position);

/// Records how much of a string slice has been read
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
    Self { reader, stack: alloc::vec![ParseContext::Top] }
  }

  #[inline(always)]
  const fn pos(&self) -> Position {
    self.reader.read_pos
  }

  /// Span from some previously marked position to the current position of the walker's reader
  #[inline(always)]
  pub const fn span_from(&self, marker: Position) -> Span {
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
enum ParseContext<'e> {
  Top,
  Vector(Vec<Node<'e>>, Position),
  List(Vec<Node<'e>>, Position),
  Map(Vec<(Node<'e>, Node<'e>)>, Option<Node<'e>>, Position),
  Set(Vec<Node<'e>>, Position),
  Tag(&'e str, Position),
  Discard,
}

#[inline]
fn parse_element<'e>(reader: &mut SourceReader<'e>, next: char) -> Result<Node<'e>, Error> {
  let pos_start = reader.read_pos;
  match next {
    '\\' => match parse_char(reader.slurp_char()) {
      Ok(node) => Ok(Node::Char(node, reader.span_from(pos_start))),
      Err(code) => Err(Error::from_position(code, pos_start)),
    },
    '\"' => {
      let str = reader.slurp_str()?;
      Ok(Node::Str(str, reader.span_from(pos_start)))
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

#[inline]
fn add_to_context<'e>(context: &mut Option<&mut ParseContext<'e>>, node: Node<'e>) {
  match context.as_mut() {
    Some(ParseContext::Vector(vec, _) | ParseContext::List(vec, _) | ParseContext::Set(vec, _)) => {
      vec.push(node);
    }
    Some(ParseContext::Map(map, pending, _)) => {
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
      walker.push_context(ParseContext::Vector(Vec::new(), pos_start));
    }
    OpenDelimiter::List => {
      let _ = walker.reader.nibble_next();
      walker.push_context(ParseContext::List(Vec::new(), pos_start));
    }
    OpenDelimiter::Map => {
      let _ = walker.reader.nibble_next();
      walker.push_context(ParseContext::Map(Vec::new(), None, pos_start));
    }
    OpenDelimiter::Hash => {
      let _ = walker.reader.nibble_next();
      match walker.reader.peek_next() {
        Some('{') => {
          let _ = walker.reader.nibble_next();
          walker.push_context(ParseContext::Set(Vec::new(), pos_start));
        }
        Some('_') => {
          let _ = walker.reader.nibble_next();
          walker.push_context(ParseContext::Discard);
        }
        _ => {
          let tag = walker.reader.slurp_tag()?;
          walker.reader.nibble_whitespace();
          walker.push_context(ParseContext::Tag(tag, pos_start));
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
) -> Result<Option<Node<'e>>, Error> {
  if walker.stack_len() <= 1 {
    return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
  }
  let expected = match walker.stack.last().expect("Len > 1 is never empty") {
    ParseContext::Vector(..) => ']',
    ParseContext::List(..) => ')',
    ParseContext::Map(..) | ParseContext::Set(..) => '}',
    _ => {
      return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
    }
  };
  if delimiter != expected {
    return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
  }
  let mut node = match walker.pop_context() {
    Some(ParseContext::Vector(vec, pos_start)) => {
      let _ = walker.reader.nibble_next();
      Node::Vector(vec, walker.span_from(pos_start))
    }
    Some(ParseContext::List(vec, pos_start)) => {
      let _ = walker.reader.nibble_next();
      Node::List(vec, walker.span_from(pos_start))
    }
    Some(ParseContext::Map(map, pending, pos_start)) => {
      if pending.is_some() {
        return Err(walker.make_error(Code::UnexpectedEOF));
      }
      let _ = walker.reader.nibble_next();
      Node::Map(map, walker.span_from(pos_start))
    }
    Some(ParseContext::Set(set, pos_start)) => {
      let _ = walker.reader.nibble_next();
      Node::Set(set, walker.span_from(pos_start))
    }
    _ => {
      // this should be impossible, due to checking for unmatched delimiters above
      return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
    }
  };

  if walker.stack_len() == 1 {
    return Ok(Some(node));
  }
  while let Some(context) = walker.pop_context() {
    match context {
      ParseContext::Tag(t, pos_start) => {
        node = Node::Tagged(t, Box::new(node), walker.span_from(pos_start));
      }
      other => {
        walker.push_context(other);
        break;
      }
    }
  }

  if walker.stack_len() == 1 {
    return Ok(Some(node));
  } else if matches!(walker.stack.last(), Some(&ParseContext::Discard)) {
    walker.pop_context();
  } else {
    add_to_context(&mut walker.stack.last_mut(), node);
  }
  Ok(None)
}

#[inline]
fn handle_element<'e>(walker: &mut Walker<'e, '_>, next: char) -> Result<Option<Node<'e>>, Error> {
  let node = parse_element(walker.reader, next)?;
  if walker.stack_len() == 1 {
    return Ok(Some(node));
  }
  let node = match walker.stack.last() {
    Some(ParseContext::Tag(tag, pos_start)) => {
      let mut tag = Node::Tagged(tag, Box::new(node), walker.span_from(*pos_start));
      walker.pop_context();
      while let Some(ParseContext::Tag(t, pos_start)) = walker.stack.last() {
        tag = Node::Tagged(t, Box::new(tag), walker.span_from(*pos_start));
        walker.pop_context();
      }
      if walker.stack_len() == 1 {
        return Ok(Some(tag));
      }
      tag
    }
    Some(ParseContext::Discard) => {
      walker.pop_context();
      return Ok(None);
    }
    _ => node,
  };
  add_to_context(&mut walker.stack.last_mut(), node);
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
        if let Some(node) = handle_close_delimiter(walker, d)? {
          result = Some(node);
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
fn edn_literal(literal: &str, span: Span) -> Result<Node<'_>, Code> {
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
    "nil" => Node::Nil(span),
    "true" => Node::Bool(true, span),
    "false" => Node::Bool(false, span),
    k if k.starts_with(':') => {
      if k.len() <= 1 {
        return Err(Code::InvalidKeyword);
      }
      Node::Key(&k[1..], span)
    }
    n if numeric(n) => parse_number(n, span)?,
    _ => Node::Symbol(literal, span),
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
fn parse_number(lit: &str, span: Span) -> Result<Node<'_>, Code> {
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
    return Ok(Node::Int(n * i64::from(polarity), span));
  }
  if radix == 10
    && let Some((n, d)) = num_den_from_slice(number, polarity)
  {
    return Ok(Node::Rational((n, d), span));
  }

  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_int_from_slice(number, radix, polarity) {
    return Ok(Node::BigInt(n, span));
  }
  #[cfg(feature = "floats")]
  if radix == 10
    && let Ok(n) = number.parse::<f64>()
  {
    return Ok(Node::Double((n * f64::from(polarity)).into(), span));
  }
  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_dec_from_slice(number, radix, polarity) {
    return Ok(Node::BigDec(n, span));
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
