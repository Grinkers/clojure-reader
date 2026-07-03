use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{Display, Write};

use serde::{Serialize, ser};

use crate::error::{Code, Error, Result};

#[derive(Debug)]
pub struct Serializer {
  output: String,
  compound_is_empty: Vec<bool>,
}

impl Serializer {
  fn start_compound(&mut self, opener: &str) {
    self.output += opener;
    self.compound_is_empty.push(true);
  }

  fn write_separator(&mut self, separator: &str) -> Result<()> {
    let compound_is_empty = self
      .compound_is_empty
      .last_mut()
      .ok_or_else(|| ser::Error::custom("serializer compound state missing"))?;
    if *compound_is_empty {
      *compound_is_empty = false;
    } else {
      self.output += separator;
    }
    Ok(())
  }

  fn end_compound(&mut self, closer: &str) -> Result<()> {
    self
      .compound_is_empty
      .pop()
      .ok_or_else(|| ser::Error::custom("serializer compound state missing"))?;
    self.output += closer;
    Ok(())
  }
}

impl ser::Error for Error {
  #[cold]
  fn custom<T: Display>(msg: T) -> Self {
    Self { code: Code::Serde(msg.to_string()), line: None, column: None, ptr: None }
  }
}

/// Serializer for creating an EDN formatted String
///
/// # Errors
///
/// See [`crate::error::Error`].
/// Always returns `Code::Serde`.
pub fn to_string<T>(value: &T) -> Result<String>
where
  T: Serialize,
{
  let mut serializer =
    Serializer { output: String::with_capacity(128), compound_is_empty: Vec::new() };
  value.serialize(&mut serializer)?;
  Ok(serializer.output)
}

impl ser::Serializer for &mut Serializer {
  type Ok = ();
  type Error = Error;

  type SerializeSeq = Self;
  type SerializeTuple = Self;
  type SerializeTupleStruct = Self;
  type SerializeTupleVariant = Self;
  type SerializeMap = Self;
  type SerializeStruct = Self;
  type SerializeStructVariant = Self;

  fn serialize_bool(self, v: bool) -> Result<()> {
    self.output += if v { "true" } else { "false" };
    Ok(())
  }

  // EDN is always an i64 for integers, so all integers will be serialized as i64.
  fn serialize_i8(self, v: i8) -> Result<()> {
    self.serialize_i64(i64::from(v))
  }

  fn serialize_i16(self, v: i16) -> Result<()> {
    self.serialize_i64(i64::from(v))
  }

  fn serialize_i32(self, v: i32) -> Result<()> {
    self.serialize_i64(i64::from(v))
  }

  fn serialize_i64(self, v: i64) -> Result<()> {
    // Infallible: String::write_fmt never errors, but handle for correctness.
    self
      .output
      .write_fmt(format_args!("{v}"))
      .map_err(|e| ser::Error::custom(format!("failed to format {v}: {e}")))?;
    Ok(())
  }

  fn serialize_u8(self, v: u8) -> Result<()> {
    self.serialize_u64(u64::from(v))
  }

  fn serialize_u16(self, v: u16) -> Result<()> {
    self.serialize_u64(u64::from(v))
  }

  fn serialize_u32(self, v: u32) -> Result<()> {
    self.serialize_u64(u64::from(v))
  }

  fn serialize_u64(self, v: u64) -> Result<()> {
    if let Ok(v) = i64::try_from(v) {
      return self.serialize_i64(v);
    }

    #[cfg(not(feature = "arbitrary-nums"))]
    {
      Err(ser::Error::custom(format!(
        "can't serialize {v} as a round-trippable EDN integer without arbitrary-nums"
      )))
    }

    #[cfg(feature = "arbitrary-nums")]
    {
      // Infallible: String::write_fmt never errors, but handle for correctness.
      self
        .output
        .write_fmt(format_args!("{v}N"))
        .map_err(|e| ser::Error::custom(format!("failed to format {v}: {e}")))?;
      Ok(())
    }
  }

  fn serialize_f32(self, v: f32) -> Result<()> {
    self.serialize_f64(f64::from(v))
  }

  fn serialize_f64(self, v: f64) -> Result<()> {
    // Infallible: String::write_fmt never errors, but handle for correctness.
    self
      .output
      .write_fmt(format_args!("{v}"))
      .map_err(|e| ser::Error::custom(format!("failed to format {v}: {e}")))?;
    Ok(())
  }

  fn serialize_char(self, v: char) -> Result<()> {
    self.output += "\\";
    if let Some(c) = crate::edn::char_to_edn(v) {
      self.output += c;
    } else {
      self.output.push(v);
    }
    Ok(())
  }

  fn serialize_str(self, v: &str) -> Result<()> {
    self.output += "\"";
    self.output += v;
    self.output += "\"";
    Ok(())
  }

  // as of 2024-11, this is not called by serde
  // https://serde.rs/impl-serialize.html
  fn serialize_bytes(self, v: &[u8]) -> Result<()> {
    use serde::ser::SerializeSeq;

    let mut seq = self.serialize_seq(Some(v.len()))?;
    for byte in v {
      seq.serialize_element(byte)?;
    }
    seq.end()
  }

  fn serialize_none(self) -> Result<()> {
    self.serialize_unit()
  }

  fn serialize_some<T>(self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_unit(self) -> Result<()> {
    self.output += "nil";
    Ok(())
  }

  fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
    self.serialize_unit()
  }

  fn serialize_unit_variant(
    self,
    name: &'static str,
    _variant_index: u32,
    variant: &'static str,
  ) -> Result<()> {
    self.output += "#";
    self.output += name;
    self.output += "/";
    self.output += variant;
    self.output += " ";
    self.serialize_unit()
  }

  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_newtype_variant<T>(
    self,
    name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    value: &T,
  ) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.output += "#";
    self.output += name;
    self.output += "/";
    self.output += variant;
    self.output += " ";
    value.serialize(self)
  }

  fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
    if let Some(len) = len {
      self.output.reserve(len * 16);
    }
    self.start_compound("[");
    Ok(self)
  }

  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    self.start_compound("[");
    Ok(self)
  }

  fn serialize_tuple_struct(
    self,
    _name: &'static str,
    len: usize,
  ) -> Result<Self::SerializeTupleStruct> {
    self.serialize_tuple(len)
  }

  fn serialize_tuple_variant(
    self,
    name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    self.output += "#";
    self.output += name;
    self.output += "/";
    self.output += variant;
    self.output += " ";
    self.start_compound("[");
    Ok(self)
  }

  fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
    if let Some(len) = len {
      self.output.reserve(len * 32);
    }
    self.start_compound("{");
    Ok(self)
  }

  fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
    self.serialize_map(Some(len))
  }

  fn serialize_struct_variant(
    self,
    name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    self.output += "#";
    self.output += name;
    self.output += "/";
    self.output += variant;
    self.output += " ";
    self.start_compound("{");
    Ok(self)
  }
}

impl ser::SerializeSeq for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(" ")?;
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("]")
  }
}

impl ser::SerializeTuple for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(" ")?;
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("]")
  }
}

impl ser::SerializeTupleStruct for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(" ")?;
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("]")
  }
}

impl ser::SerializeTupleVariant for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(" ")?;
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("]")
  }
}

impl ser::SerializeMap for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_key<T>(&mut self, key: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(", ")?;

    key.serialize(&mut **self)
  }

  fn serialize_value<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.output += " ";
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("}")
  }
}

impl ser::SerializeStruct for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(", ")?;
    self.output += ":";
    self.output += key;
    self.output += " ";
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("}")
  }
}

impl ser::SerializeStructVariant for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.write_separator(", ")?;
    self.output += ":";
    self.output += key;
    self.output += " ";
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.end_compound("}")
  }
}
