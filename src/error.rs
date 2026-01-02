use core::error;
use core::fmt::{self, Debug};

pub type Result<T> = core::result::Result<T, Error>;

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
  /// Elaboation errors
  HashMapDuplicateKey,
  SetDuplicateKey,

  /// Parse errors
  InvalidChar,
  InvalidEscape,
  InvalidKeyword,
  InvalidNumber,
  InvalidRadix(Option<u8>),
  UnexpectedEOF,
  UnmatchedDelimiter(char),

  /// Feature errors
  NoFloatFeature,

  /// Serde
  #[cfg(feature = "serde")]
  Serde(alloc::string::String),
}

impl Error {
  pub(crate) const fn from_position(code: Code, position: crate::parse::Position) -> Self {
    Self { code, line: Some(position.line), column: Some(position.column), ptr: Some(position.ptr) }
  }
}

impl error::Error for Error {}

impl Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "EdnError {{ code: {:?}, line: {:?}, column: {:?}, ptr: {:?} }}",
      self.code, self.line, self.column, self.ptr
    )
  }
}

impl alloc::fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{self:?}")
  }
}
