#![expect(clippy::inline_always)]

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::primitive::str;

use crate::edn::Edn;
use crate::error::{Code, Error};

pub fn parse(edn: &str) -> Result<(Edn<'_>, &str), Error> {
  let mut walker = Walker::new(edn);
  let internal_parse = parse_internal(&mut walker)?;
  internal_parse
    .map_or_else(|| Ok((Edn::Nil, &edn[walker.ptr..])), |ip| Ok((ip, &edn[walker.ptr..])))
}

const DELIMITERS: [char; 8] = [',', ']', '}', ')', ';', '(', '[', '{'];

#[derive(Debug)]
struct Walker<'e> {
  slice: &'e str,
  ptr: usize,
  column: usize,
  line: usize,
  stack: Vec<ParseContext<'e>>,
}

impl<'e> Walker<'e> {
  fn new(slice: &'e str) -> Self {
    Self { slice, ptr: 0, column: 1, line: 1, stack: alloc::vec![ParseContext::Top] }
  }
}

impl<'e> Walker<'e> {
  // Slurps until whitespace or delimiter, returning the slice.
  #[inline(always)]
  fn slurp_literal(&mut self) -> &'e str {
    let token = self.slice[self.ptr..]
      .split(|c: char| c.is_whitespace() || DELIMITERS.contains(&c) || c == '"')
      .next()
      .expect("Expected at least an empty slice");

    self.ptr += token.len();
    self.column += token.len();
    token
  }

  // Slurps a char. Special handling for chars that happen to be delimiters
  #[inline(always)]
  fn slurp_char(&mut self) -> &'e str {
    let starting_ptr = self.ptr;

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
    let starting_ptr = self.ptr;
    let mut escape = false;
    loop {
      if let Some(c) = self.nibble_next() {
        if escape {
          match c {
            't' | 'r' | 'n' | '\\' | '\"' => (),
            _ => {
              return Err(Error {
                code: Code::InvalidEscape,
                column: Some(self.column),
                line: Some(self.line),
                ptr: Some(self.ptr),
              });
            }
          }
          escape = false;
        } else if c == '\"' {
          return Ok(&self.slice[starting_ptr..self.ptr - 1]);
        } else {
          escape = c == '\\';
        }
      } else {
        return Err(Error {
          code: Code::UnexpectedEOF,
          column: Some(self.column),
          line: Some(self.line),
          ptr: Some(self.ptr),
        });
      }
    }
  }

  #[inline(always)]
  fn slurp_tag(&mut self) -> Result<&'e str, Error> {
    self.nibble_whitespace();
    let starting_ptr = self.ptr;

    loop {
      if let Some(c) = self.peek_next() {
        if c.is_whitespace() || DELIMITERS.contains(&c) {
          return Ok(&self.slice[starting_ptr..self.ptr]);
        }
        let _ = self.nibble_next();
      } else {
        return Err(Error {
          code: Code::UnexpectedEOF,
          column: Some(self.column),
          line: Some(self.line),
          ptr: Some(self.ptr),
        });
      }
    }
  }

  // Nibbles away until the next new line
  #[inline(always)]
  fn nibble_newline(&mut self) {
    let len = self.slice[self.ptr..].split('\n').next().expect("Expected at least an empty slice");
    self.ptr += len.len();
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
    let char = self.slice[self.ptr..].chars().next();
    if let Some(c) = char {
      self.ptr += c.len_utf8();
      if c == '\n' {
        self.line += 1;
        self.column = 1;
      } else {
        self.column += 1;
      }
    }
    char
  }

  // Peek into the next char
  #[inline(always)]
  fn peek_next(&self) -> Option<char> {
    self.slice[self.ptr..].chars().next()
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
    Error { code, line: Some(self.line), column: Some(self.column), ptr: Some(self.ptr) }
  }
}

#[derive(Debug, Clone, Copy)]
enum OpenDelimiter {
  Vector,
  List,
  Map,
  Hash,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseContext<'e> {
  Top,
  Vector(Vec<Edn<'e>>),
  List(Vec<Edn<'e>>),
  Map(BTreeMap<Edn<'e>, Edn<'e>>, Option<Edn<'e>>),
  Set(BTreeSet<Edn<'e>>),
  Tag(&'e str),
  Discard,
}

#[inline]
fn parse_element<'e>(walker: &mut Walker<'e>, next: char) -> Result<Edn<'e>, Error> {
  let column_start = walker.column;
  let ptr_start = walker.ptr;
  let line_start = walker.line;
  match next {
    '\\' => match parse_char(walker.slurp_char()) {
      Ok(edn) => Ok(edn),
      Err(code) => Err(Error {
        code,
        line: Some(line_start),
        column: Some(column_start),
        ptr: Some(ptr_start),
      }),
    },
    '\"' => Ok(Edn::Str(walker.slurp_str()?)),
    _ => match edn_literal(walker.slurp_literal()) {
      Ok(edn) => Ok(edn),
      Err(code) => Err(Error {
        code,
        line: Some(line_start),
        column: Some(column_start),
        ptr: Some(ptr_start),
      }),
    },
  }
}

#[inline]
fn add_to_context<'e>(
  context: &mut Option<&mut ParseContext<'e>>,
  edn: Edn<'e>,
) -> Result<(), Code> {
  match context.as_mut() {
    Some(ParseContext::Vector(vec) | ParseContext::List(vec)) => vec.push(edn),
    Some(ParseContext::Map(map, pending)) => {
      if let Some(key) = pending.take() {
        if map.insert(key, edn).is_some() {
          return Err(Code::HashMapDuplicateKey);
        }
      } else {
        *pending = Some(edn);
      }
    }
    Some(ParseContext::Set(set)) => {
      if !set.insert(edn) {
        return Err(Code::SetDuplicateKey);
      }
    }
    _ => {} // Do nothing. Errors will bubble up elsewhere.
  }
  Ok(())
}

#[inline]
fn handle_open_delimiter(walker: &mut Walker<'_>, delim: OpenDelimiter) -> Result<(), Error> {
  match delim {
    OpenDelimiter::Vector => {
      let _ = walker.nibble_next();
      walker.push_context(ParseContext::Vector(Vec::new()));
    }
    OpenDelimiter::List => {
      let _ = walker.nibble_next();
      walker.push_context(ParseContext::List(Vec::new()));
    }
    OpenDelimiter::Map => {
      let _ = walker.nibble_next();
      walker.push_context(ParseContext::Map(BTreeMap::new(), None));
    }
    OpenDelimiter::Hash => {
      let _ = walker.nibble_next();
      match walker.peek_next() {
        Some('{') => {
          let _ = walker.nibble_next();
          walker.push_context(ParseContext::Set(BTreeSet::new()));
        }
        Some('_') => {
          let _ = walker.nibble_next();
          walker.push_context(ParseContext::Discard);
        }
        _ => {
          let tag = walker.slurp_tag()?;
          walker.nibble_whitespace();
          walker.push_context(ParseContext::Tag(tag));
        }
      }
    }
  }
  Ok(())
}

#[inline]
fn handle_close_delimiter<'e>(
  walker: &mut Walker<'e>,
  delimiter: char,
) -> Result<Option<Edn<'e>>, Error> {
  if walker.stack_len() <= 1 {
    return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
  }
  let expected = match walker.stack.last().expect("Len > 1 is never empty") {
    ParseContext::Vector(_) => ']',
    ParseContext::List(_) => ')',
    ParseContext::Map(_, _) | ParseContext::Set(_) => '}',
    _ => {
      return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
    }
  };
  if delimiter != expected {
    return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
  }
  let mut edn = match walker.pop_context() {
    Some(ParseContext::Vector(vec)) => Edn::Vector(vec),
    Some(ParseContext::List(vec)) => Edn::List(vec),
    Some(ParseContext::Map(map, pending)) => {
      if pending.is_some() {
        return Err(walker.make_error(Code::UnexpectedEOF));
      }
      Edn::Map(map)
    }
    Some(ParseContext::Set(set)) => Edn::Set(set),
    _ => {
      // this should be impossible, due to checking for unmatched delimiters above
      return Err(walker.make_error(Code::UnmatchedDelimiter(delimiter)));
    }
  };
  let _ = walker.nibble_next();

  if walker.stack_len() == 1 {
    return Ok(Some(edn));
  }
  while let Some(context) = walker.pop_context() {
    match context {
      ParseContext::Tag(t) => {
        edn = Edn::Tagged(t, Box::new(edn));
      }
      other => {
        walker.push_context(other);
        break;
      }
    }
  }

  if walker.stack_len() == 1 {
    return Ok(Some(edn));
  } else if walker.stack.last() == Some(&ParseContext::Discard) {
    walker.pop_context();
  } else if let Err(code) = add_to_context(&mut walker.stack.last_mut(), edn) {
    return Err(walker.make_error(code));
  }
  Ok(None)
}

#[inline]
fn handle_element<'e>(walker: &mut Walker<'e>, next: char) -> Result<Option<Edn<'e>>, Error> {
  let edn = parse_element(walker, next)?;
  if walker.stack_len() == 1 {
    return Ok(Some(edn));
  }
  let edn = match walker.stack.last() {
    Some(ParseContext::Tag(tag)) => {
      let mut tag = Edn::Tagged(tag, Box::new(edn));
      walker.pop_context();
      while let Some(ParseContext::Tag(t)) = walker.stack.last() {
        tag = Edn::Tagged(t, Box::new(tag));
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
    _ => edn,
  };
  if let Err(code) = add_to_context(&mut walker.stack.last_mut(), edn) {
    return Err(walker.make_error(code));
  }
  Ok(None)
}

fn parse_internal<'e>(walker: &mut Walker<'e>) -> Result<Option<Edn<'e>>, Error> {
  let mut result: Option<Edn<'e>> = None;
  loop {
    walker.nibble_whitespace();
    match walker.peek_next() {
      Some(';') => walker.nibble_newline(),
      Some('[') => handle_open_delimiter(walker, OpenDelimiter::Vector)?,
      Some('(') => handle_open_delimiter(walker, OpenDelimiter::List)?,
      Some('{') => handle_open_delimiter(walker, OpenDelimiter::Map)?,
      Some('#') => handle_open_delimiter(walker, OpenDelimiter::Hash)?,
      Some(d) if matches!(d, ']' | ')' | '}') => {
        if let Some(edn) = handle_close_delimiter(walker, d)? {
          result = Some(edn);
          break;
        }
      }
      Some(c) => {
        if let Some(edn) = handle_element(walker, c)? {
          result = Some(edn);
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
fn edn_literal(literal: &str) -> Result<Edn<'_>, Code> {
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
    "nil" => Edn::Nil,
    "true" => Edn::Bool(true),
    "false" => Edn::Bool(false),
    k if k.starts_with(':') => {
      if k.len() <= 1 {
        return Err(Code::InvalidKeyword);
      }
      Edn::Key(&k[1..])
    }
    n if numeric(n) => parse_number(n)?,
    _ => Edn::Symbol(literal),
  })
}

#[inline]
fn parse_char(lit: &str) -> Result<Edn<'_>, Code> {
  let lit = &lit[1..]; // ignore the leading '\\'
  match lit {
    "newline" => Ok(Edn::Char('\n')),
    "return" => Ok(Edn::Char('\r')),
    "tab" => Ok(Edn::Char('\t')),
    "space" => Ok(Edn::Char(' ')),
    c if c.len() == 1 => Ok(Edn::Char(c.chars().next().expect("c must be len of 1"))),
    _ => Err(Code::InvalidChar),
  }
}

#[inline]
fn parse_number(lit: &str) -> Result<Edn<'_>, Code> {
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
    return Ok(Edn::Int(n * i64::from(polarity)));
  }
  if radix == 10
    && let Some((n, d)) = num_den_from_slice(number, polarity)
  {
    return Ok(Edn::Rational((n, d)));
  }

  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_int_from_slice(number, radix, polarity) {
    return Ok(Edn::BigInt(n));
  }
  #[cfg(feature = "floats")]
  if radix == 10
    && let Ok(n) = number.parse::<f64>()
  {
    return Ok(Edn::Double((n * f64::from(polarity)).into()));
  }
  #[cfg(feature = "arbitrary-nums")]
  if let Some(n) = big_dec_from_slice(number, radix, polarity) {
    return Ok(Edn::BigDec(n));
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
