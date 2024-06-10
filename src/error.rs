use core::fmt::{self, Debug};
use core::str;

pub struct Error {
  pub code: Code,
  /// Counting from 1.
  pub line: Option<usize>,
  /// This is a utf-8 char count. Counting from 1.
  pub column: Option<usize>,
  /// This is a pointer into the str trying to be parsed, not a utf-8 char offset
  pub ptr: Option<usize>,
}

#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Code {
  /// Parse errors
  HashMapDuplicateKey,
  SetDuplicateKey,
  InvalidChar,
  InvalidEscape,
  InvalidKeyword,
  InvalidNumber,
  InvalidRadix(Option<u8>),
  UnexpectedEOF,
  UnmatchedDelimiter(char),

  /// Feature errors
  NoFloatFeature,

  /// An alternate to panics/todo
  Unimplemented(&'static str),
}

impl Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "EdnError {{ code: {:?}, line: {:?}, column: {:?}, ptr: {:?} }}",
      self.code, self.line, self.column, self.ptr
    )
  }
}
