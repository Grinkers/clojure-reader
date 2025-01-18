#[cfg(test)]
#[cfg(feature = "serde")]
mod test {
  extern crate alloc;

  use alloc::borrow::ToOwned;
  use alloc::string::String;
  use alloc::string::ToString;
  use alloc::vec;
  use alloc::vec::Vec;

  use clojure_reader::de::from_str;
  use serde_derive::Deserialize;

  #[test]
  fn super_simple_types() {
    assert_eq!(42, from_str::<u8>("42").unwrap());
    assert_eq!(42, from_str::<i64>("42").unwrap());
    assert_eq!(424242, from_str::<i64>("424242").unwrap());

    let res = from_str::<u8>("424242");
    let Err(res) = res else { panic!() };
    let expected = "EdnError { code: Serde(\"can't convert Err(TryFromIntError(())) into u8\"), line: None, column: None, ptr: None }";
    assert_eq!(format!("{res}"), expected);

    assert_eq!("lol cats", from_str::<String>(r#""lol cats""#).unwrap());
    assert_eq!("lol 猫s", from_str::<&str>(r#""lol 猫s""#).unwrap());
    assert_eq!(false, from_str("false").unwrap());
  }

  #[test]
  fn maybe() {
    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "kebab-case")]
    struct Test {
      #[serde(alias = "maybe_int")]
      maybe_int: Option<u32>,
      maybe_str: Option<String>,
    }

    let res = from_str::<Test>(r#"{:maybe-int 42, 42 "neko", :maybe-str "gato"}"#).unwrap();
    assert_eq!(res, Test { maybe_int: Some(42), maybe_str: Some("gato".to_string()) });
  }

  #[test]
  fn errors() {
    let edn_str = r"cat in your nums";
    let res = from_str::<u8>(edn_str);
    let expected = r#"Err(EdnError { code: Serde("cannot convert Symbol(\"cat\") to i64"), line: None, column: None, ptr: None })"#;
    assert!(res.is_err());
    assert_eq!(format!("{res:?}"), expected);

    let res = from_str::<f32>(edn_str);
    assert!(res.is_err());
  }

  #[test]
  fn seq() {
    let expected: [i64; 4] = [1, 4, 42, 3];
    let res = from_str::<[i64; 4]>("[1 4 42 3]");
    assert_eq!(expected, res.unwrap());

    let expected: Vec<u16> = vec![1, 4, 42, 3];
    let res = from_str::<Vec<u16>>("[1 4 42 3]");
    assert_eq!(expected, res.unwrap());

    let expected: Vec<u16> = vec![1, 3, 4, 42];
    let res = from_str::<Vec<u16>>("#{1 4 42 3}");
    assert_eq!(expected, res.unwrap());
  }

  #[test]
  fn unit_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Unit;
    assert_eq!(Unit {}, from_str::<Unit>("").unwrap());
    assert_eq!(Unit {}, from_str::<Unit>("{}").unwrap());

    #[derive(Deserialize, PartialEq, Debug)]
    struct UnitNone {
      unit: Unit,
      nothing: Option<()>,
    }
    assert_eq!(
      UnitNone { unit: Unit, nothing: None },
      from_str::<UnitNone>(r#"{"unit" {} "nothing" nil}"#).unwrap()
    );
  }

  #[test]
  fn new_type() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Meters(i64);

    assert_eq!(Meters(420), from_str(r#"420"#).unwrap());
  }

  #[test]
  fn tuple_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
      int: (u32, i64),
    }

    assert_eq!(Test { int: (42, -420) }, from_str(r#"{"int" (42 -420)}"#).unwrap());
  }

  #[test]
  fn test_simple_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
      int: u32,
      c: char,
    }

    assert_eq!(Test { int: 42, c: 'c' }, from_str(r#"{"int" 42 :c \c}"#).unwrap());
    assert_eq!(Test { int: 42, c: 'n' }, from_str(r#"{:int 42 "c" \n}"#).unwrap());
  }

  #[test]
  fn test_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
      int: u32,
      seq: Vec<String>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Tests {
      tests: Vec<Test>,
    }

    let edn_str = r#"{"int" 1, "seq" ["a","b"]}"#;
    let expected = Test { int: 1, seq: vec!["a".to_owned(), "b".to_owned()] };
    assert_eq!(expected, from_str(edn_str).unwrap());

    // allow both "int" and :int
    let edn_str = r#"{:int 1, "seq" ["a","b"]}"#;
    assert_eq!(expected, from_str(edn_str).unwrap());

    let edn_str = r#"{:tests [{:int 1, "seq" ["a","b"]} {:int 2, "seq" ["a","b"]}]}"#;
    let expected = Tests {
      tests: vec![
        Test { int: 1, seq: vec!["a".to_owned(), "b".to_owned()] },
        Test { int: 2, seq: vec!["a".to_owned(), "b".to_owned()] },
      ],
    };
    assert_eq!(expected, from_str(edn_str).unwrap());
  }

  #[test]
  fn complex_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Nums {
      a: i8,
      b: i16,
      cat: i32,
      double: f64,
      trunk: f32,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
      int: u64,
      nums: Nums,
    }

    let edn_str = r#"{:int 1, :nums {:a 4, :b 2, :cat 42, :double 42.42, :trunk 42.0}}"#;
    let expected = Test { int: 1, nums: Nums { a: 4, b: 2, cat: 42, double: 42.42, trunk: 42.0 } };
    assert_eq!(expected, from_str(edn_str).unwrap());
  }

  #[test]
  fn test_enum() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum E {
      Unit,
      AnotherUnit,
      Newtype(u32),
      Tuple(u32, u32),
      Struct { a: u32, b: usize },
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
      e: E,
    }

    assert_eq!(Test { e: E::Unit }, from_str::<Test>(r#"{:e #E/Unit nil}"#).unwrap());
    assert_eq!(E::Unit, from_str::<E>(r#"#E/Unit nil"#).unwrap());
    assert_eq!(E::Unit, from_str::<E>(r#"#E/Unit :Unit"#).unwrap());
    assert_eq!(E::Unit, from_str::<E>(r#"#E/Unit sillycat"#).unwrap());
    assert_eq!(E::AnotherUnit, from_str::<E>(r#"#E/AnotherUnit nil"#).unwrap());
    assert_eq!(E::Newtype(1), from_str::<E>(r#"#E/Newtype 1"#).unwrap());
    assert_eq!(E::Tuple(1, 2), from_str::<E>(r#"#E/Tuple [1 2]"#).unwrap());
    assert_eq!(E::Struct { a: 1, b: 42 }, from_str::<E>(r#"#E/Struct {:a 1, :b 42}"#,).unwrap());

    assert_eq!(format!("{:?}", from_str::<E>(r#"#B/Unit sillycat"#)), "Err(EdnError { code: Serde(\"namespace in B/Unit can't be matched to E\"), line: None, column: None, ptr: None })");
    assert_eq!(format!("{:?}", from_str::<E>(r#""#)), "Err(EdnError { code: Serde(\"can't convert Nil into Tagged for enum\"), line: None, column: None, ptr: None })");
    assert_eq!(format!("{:?}", from_str::<E>(r#"#BUnit sillycat"#)), "Err(EdnError { code: Serde(\"Expected namespace in BUnit for Tagged for enum\"), line: None, column: None, ptr: None })");
  }

  #[test]
  fn serde_errors() {
    assert_eq!(format!("{:?}", from_str::<String>(r#"#E/Tuple [4/2]"#)),
               "Err(EdnError { code: Serde(\"Don't know how to convert Tagged(\\\"E/Tuple\\\", Vector([Rational((4, 2))])) into any\"), line: None, column: None, ptr: None })");

    #[derive(Deserialize, PartialEq, Debug)]
    struct SomeBytes<'a> {
      data: &'a [u8],
    }
    assert_eq!(format!("{:?}", from_str::<SomeBytes<'_>>(r#"[4/2]"#)),
               "Err(EdnError { code: Serde(\"deserialize_bytes is unimplemented/unused\"), line: None, column: None, ptr: None })");
  }
}
