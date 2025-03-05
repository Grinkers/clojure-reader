use alloc::string::{String, ToString};
use core::fmt::Display;

use serde::{Serialize, ser};

use crate::error::{Code, Error, Result};

#[derive(Debug)]
pub struct Serializer {
  output: String,
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
  let mut serializer = Serializer { output: String::new() };
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
    self.output += &v.to_string();
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
    self.output += &v.to_string();
    Ok(())
  }

  fn serialize_f32(self, v: f32) -> Result<()> {
    self.serialize_f64(f64::from(v))
  }

  fn serialize_f64(self, v: f64) -> Result<()> {
    self.output += &v.to_string();
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

  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    self.output += "[";
    Ok(self)
  }

  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    self.output += "[";
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
    self.output += " [";
    Ok(self)
  }

  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    self.output += "{";
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
    self.output += " {";
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
    if !self.output.ends_with('[') {
      self.output += " ";
    }
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "]";
    Ok(())
  }
}

impl ser::SerializeTuple for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('[') {
      self.output += " ";
    }
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "]";
    Ok(())
  }
}

impl ser::SerializeTupleStruct for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('[') {
      self.output += " ";
    }
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "]";
    Ok(())
  }
}

impl ser::SerializeTupleVariant for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('[') {
      self.output += " ";
    }
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "]";
    Ok(())
  }
}

impl ser::SerializeMap for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_key<T>(&mut self, key: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('{') {
      self.output += ", ";
    }

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
    self.output += "}";
    Ok(())
  }
}

impl ser::SerializeStruct for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('{') {
      self.output += ", ";
    }
    self.output += ":";
    self.output += key;
    self.output += " ";
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "}";
    Ok(())
  }
}

impl ser::SerializeStructVariant for &mut Serializer {
  type Ok = ();
  type Error = Error;

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    if !self.output.ends_with('{') {
      self.output += ", ";
    }
    self.output += ":";
    self.output += key;
    self.output += " ";
    value.serialize(&mut **self)
  }

  fn end(self) -> Result<()> {
    self.output += "}";
    Ok(())
  }
}
