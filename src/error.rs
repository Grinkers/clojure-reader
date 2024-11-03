use core::fmt::{self, Debug};

pub struct Error {
  /// Error code. This is `non_exhaustive`.
  pub code: Code,
  /// Line number, counting from 1.
  pub line: Option<usize>,
  /// Column number, counting from 1. The count is utf-8 chars.
  pub column: Option<usize>,
  /// This is a pointer offset of the str trying to be parsed, not a utf-8 char offset
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
