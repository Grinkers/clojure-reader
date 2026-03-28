use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt::Display;

use crate::edn::{self, Edn};

use serde::de::{
  self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::{Deserialize, forward_to_deserialize_any};

use crate::error::{Code, Error, Result};

/// Deserializer for a EDN formatted &str.
///
/// # Errors
///
/// See [`crate::error::Error`].
/// Always returns `Code::Serde`.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
  T: Deserialize<'a>,
{
  let edn = edn::read_string(s)?;
  let t = T::deserialize(edn)?;
  Ok(t)
}

impl de::Error for Error {
  #[cold]
  fn custom<T: Display>(msg: T) -> Self {
    Self { code: Code::Serde(msg.to_string()), line: None, column: None, ptr: None }
  }
}

fn get_int_from_edn(edn: &Edn<'_>) -> Result<i64> {
  if let Edn::Int(i) = edn {
    return Ok(*i);
  }
  Err(de::Error::custom(format!("cannot convert {edn:?} to i64")))
}

fn get_bytes_from_edn(edn: &Edn<'_>) -> Result<Vec<u8>> {
  match edn {
    Edn::Vector(list) | Edn::List(list) => list
      .iter()
      .map(|item| {
        let int = get_int_from_edn(item)?;
        u8::try_from(int).map_err(|_| de::Error::custom(format!("can't convert {int} into u8")))
      })
      .collect(),
    _ => Err(de::Error::custom(format!("can't convert {edn:?} into bytes"))),
  }
}

impl<'de> de::Deserializer<'de> for Edn<'de> {
  type Error = Error;

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self {
      Edn::Key(k) => visitor.visit_borrowed_str(k),
      Edn::Str(s) | Edn::Symbol(s) => visitor.visit_borrowed_str(s),
      Edn::Int(i) => visitor.visit_i64(i),
      #[cfg(feature = "floats")]
      Edn::Double(d) => visitor.visit_f64(*d),
      Edn::Char(c) => visitor.visit_char(c),
      Edn::Bool(b) => visitor.visit_bool(b),
      Edn::Nil => visitor.visit_unit(),
      Edn::Vector(mut list) | Edn::List(mut list) => {
        list.reverse();
        Ok(visitor.visit_seq(SeqEdn::new(list))?)
      }
      Edn::Map(map) => {
        if map.is_empty() {
          visitor.visit_unit()
        } else {
          visitor.visit_map(MapEdn::new(map))
        }
      }
      Edn::Set(set) => {
        let mut s: Vec<Edn<'_>> = set.into_iter().collect();
        s.reverse();
        Ok(visitor.visit_seq(SeqEdn::new(s))?)
      }
      // Things like rational numbers and custom tags can't be represented in rust types
      _ => Err(de::Error::custom(format!("Don't know how to convert {self:?} into any"))),
    }
  }

  forward_to_deserialize_any! {
    bool i64 f64 char str unit map ignored_any seq tuple_struct
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = i8::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into i8"))),
      |i| visitor.visit_i8(i),
    )
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = i16::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into i16"))),
      |i| visitor.visit_i16(i),
    )
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = i32::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into i32"))),
      |i| visitor.visit_i32(i),
    )
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = u8::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into u8"))),
      |i| visitor.visit_u8(i),
    )
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = u16::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into u16"))),
      |i| visitor.visit_u16(i),
    )
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = u32::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into u32"))),
      |i| visitor.visit_u32(i),
    )
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let int = u64::try_from(get_int_from_edn(&self)?);
    int.map_or_else(
      |_| Err(de::Error::custom(format!("can't convert {int:?} into u64"))),
      |i| visitor.visit_u64(i),
    )
  }

  fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let _ = visitor; // hush clippy
    #[cfg(feature = "floats")]
    if let Edn::Double(f) = self {
      #[expect(clippy::cast_possible_truncation)]
      return visitor.visit_f32(*f as f32);
    }
    Err(de::Error::custom(format!("can't convert {self:?} into f32")))
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    struct BytesFromBuf<V>(V);
    impl<'de, V: Visitor<'de>> Visitor<'de> for BytesFromBuf<V> {
      type Value = V::Value;
      fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.expecting(f)
      }
      fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> core::result::Result<Self::Value, E> {
        self.0.visit_bytes(&v)
      }
    }
    self.deserialize_byte_buf(BytesFromBuf(visitor))
  }

  fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let buf = get_bytes_from_edn(&self)?;
    visitor.visit_byte_buf(buf)
  }

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    if self == Edn::Nil { visitor.visit_none() } else { visitor.visit_some(self) }
  }

  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_unit(visitor)
  }

  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_struct<V>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_map(visitor)
  }

  fn deserialize_enum<V>(
    self,
    name: &'static str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let Edn::Tagged(tag, edn) = self else {
      return Err(de::Error::custom(format!("can't convert {self:?} into Tagged for enum")));
    };

    let mut split = tag.split('/');
    let (Some(tag_first), Some(tag_second)) = (split.next(), split.next()) else {
      return Err(de::Error::custom(format!("Expected namespace in {tag} for Tagged for enum")));
    };

    if name != tag_first {
      return Err(de::Error::custom(format!("namespace in {tag} can't be matched to {name}")));
    }

    visitor.visit_enum(EnumEdn::new(*edn, tag_second))
  }

  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }
}

struct SeqEdn<'de> {
  de: Vec<Edn<'de>>,
}

impl<'de> SeqEdn<'de> {
  const fn new(de: Vec<Edn<'de>>) -> Self {
    SeqEdn { de }
  }
}

impl<'de> SeqAccess<'de> for SeqEdn<'de> {
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
  where
    T: DeserializeSeed<'de>,
  {
    let s = self.de.pop();
    match s {
      Some(e) => Ok(Some(seed.deserialize(e)?)),
      None => Ok(None),
    }
  }
}

struct MapEdn<'de> {
  de: BTreeMap<Edn<'de>, Edn<'de>>,
  pending_value: Option<Edn<'de>>,
}

impl<'de> MapEdn<'de> {
  const fn new(de: BTreeMap<Edn<'de>, Edn<'de>>) -> Self {
    MapEdn { de, pending_value: None }
  }
}

impl<'de> MapAccess<'de> for MapEdn<'de> {
  type Error = Error;

  fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
  where
    K: DeserializeSeed<'de>,
  {
    while let Some((k, _)) = self.de.first_key_value() {
      match k {
        Edn::Key(_) | Edn::Symbol(_) | Edn::Str(_) => {
          let (k, v) = self.de.pop_first().expect("key exists, we just checked");
          self.pending_value = Some(v);
          return Ok(Some(seed.deserialize(k)?));
        }
        _ => {
          self.de.pop_first();
        }
      }
    }
    Ok(None)
  }

  fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
  where
    V: DeserializeSeed<'de>,
  {
    // Infallible: serde always calls next_key_seed before next_value_seed.
    let v = self.pending_value.take().ok_or_else(|| {
      de::Error::custom("value missing: next_value_seed called without next_key_seed")
    })?;
    seed.deserialize(v)
  }
}

#[derive(Debug)]
struct EnumEdn<'de> {
  de: Edn<'de>,
  variant: &'de str,
}

impl<'de> EnumEdn<'de> {
  const fn new(de: Edn<'de>, variant: &'de str) -> Self {
    EnumEdn { de, variant }
  }
}

impl<'de> EnumAccess<'de> for EnumEdn<'de> {
  type Error = Error;
  type Variant = Self;

  fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
  where
    V: DeserializeSeed<'de>,
  {
    let val = seed.deserialize(self.variant.into_deserializer())?;
    Ok((val, self))
  }
}

impl<'de> VariantAccess<'de> for EnumEdn<'de> {
  type Error = Error;

  fn unit_variant(self) -> Result<()> {
    Ok(())
  }

  fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
  where
    T: DeserializeSeed<'de>,
  {
    seed.deserialize(self.de)
  }

  fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    de::Deserializer::deserialize_seq(self.de, visitor)
  }

  fn struct_variant<V>(
    self,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> core::result::Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    de::Deserializer::deserialize_map(self.de, visitor)
  }
}
