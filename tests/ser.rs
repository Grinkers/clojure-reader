#[cfg(feature = "serde")]
mod test {
  extern crate alloc;

  use alloc::collections::BTreeMap;
  use alloc::vec::Vec;

  use clojure_reader::ser::to_string;
  use serde_derive::Serialize;

  #[test]
  fn maybe() {
    #[derive(Serialize)]
    struct Empty {}

    #[derive(Serialize)]
    struct UnitStruct;

    #[derive(Serialize)]
    struct MaybeEmpty {
      maybe: Option<bool>,
    }

    #[derive(Serialize)]
    struct MetricUnits(i64);

    #[derive(Serialize)]
    struct MuricaUnits(i64, i64);

    assert_eq!(to_string::<Option<()>>(&None).unwrap(), "nil");
    assert_eq!(to_string(&vec![1, 2, 3]).unwrap(), "[1 2 3]");
    assert_eq!(to_string(&Empty {}).unwrap(), "{}");
    assert_eq!(to_string(&UnitStruct).unwrap(), "nil");
    assert_eq!(to_string(&MetricUnits(424242)).unwrap(), "424242");
    assert_eq!(to_string(&MuricaUnits(424242, 19847)).unwrap(), "[424242 19847]");
    assert_eq!(to_string(&MaybeEmpty { maybe: None }).unwrap(), "{:maybe nil}");
    assert_eq!(to_string(&MaybeEmpty { maybe: Some(true) }).unwrap(), "{:maybe true}");
  }

  #[test]
  fn test_struct() {
    #[derive(Serialize)]
    struct Test {
      int: u32,
    }

    assert_eq!("{:int 1}", to_string(&Test { int: 1 }).unwrap());

    #[derive(Serialize)]
    struct FooBar {
      tests: Vec<Test>,
    }

    let test = FooBar { tests: alloc::vec![Test { int: 4 }, Test { int: 2 }] };
    assert_eq!("{:tests [{:int 4} {:int 2}]}", to_string(&test).unwrap());
  }

  #[test]
  fn complex_struct() {
    #[derive(Serialize)]
    struct Nums {
      num_i16: i16,
      num_i32: i32,
      num_f32: f32,
      num_f64: f64,
    }

    #[derive(Serialize)]
    struct Seqs {
      tup: (u8, String),
      empty: (),
    }

    #[derive(Serialize)]
    struct Test {
      int: u32,
      silly_cat: bool,
      foo: BTreeMap<u8, i8>,
      bar: Vec<u16>,
      some_nums: Nums,
      character: char,
      fancy_char: char,
      seqs: Seqs,
    }

    let test = Test {
      int: 42,
      silly_cat: true,
      foo: BTreeMap::from([(1, -1), (2, -42)]),
      bar: vec![1, 2, 42, 3],
      some_nums: Nums { num_i16: 42, num_i32: 9042, num_f32: 9000.42f32, num_f64: 904200.42f64 },
      character: 'c',
      fancy_char: '\n',
      seqs: Seqs { tup: (42, "猫".to_string()), empty: () },
    };

    let expected = "{:int 42, :silly_cat true, \
                     :foo {1 -1, 2 -42}, :bar [1 2 42 3], \
                     :some_nums {:num_i16 42, :num_i32 9042, :num_f32 9000.419921875, :num_f64 904200.42}, \
                     :character \\c, :fancy_char \\newline, :seqs {:tup [42 \"猫\"], :empty nil}}";
    assert_eq!(expected, to_string(&test).unwrap());
  }

  #[test]
  fn test_enum() {
    #[derive(Serialize)]
    enum E {
      Unit,
      Newtype(u32),
      Tuple(u32, u32),
      Struct { a: u32, b: usize },
    }

    assert_eq!(r#"#E/Unit nil"#, to_string(&E::Unit).unwrap());
    assert_eq!(r#"#E/Newtype 1"#, to_string(&E::Newtype(1)).unwrap());
    assert_eq!(r#"#E/Tuple [1 2]"#, to_string(&E::Tuple(1, 2)).unwrap());
    assert_eq!(r#"#E/Struct {:a 1, :b 42}"#, to_string(&E::Struct { a: 1, b: 42 }).unwrap());
  }

  #[test]
  fn bytes() {
    #[derive(Serialize)]
    struct Refs<'a> {
      bytes: &'a [u8],
      owned_bytes: [u8; 4],
    }

    let s = String::from("yay cats");
    let refs = Refs { bytes: s.as_bytes(), owned_bytes: [1, 2, 3, 4] };
    let expected = "{:bytes [121 97 121 32 99 97 116 115], :owned_bytes [1 2 3 4]}";
    assert_eq!(expected, to_string(&refs).unwrap());
  }
}
