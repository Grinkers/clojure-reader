#[cfg(feature = "serde")]
mod test {
	extern crate alloc;

	use alloc::collections::BTreeMap;
	use alloc::string::String;
	use alloc::vec::Vec;

	use clojure_reader::ser::{to_string, to_string_pretty};
	use serde::ser;
	use serde_derive::Serialize;

	#[test]
	fn pretty() {
		#[derive(Serialize)]
		struct Person {
			name: String,
			roles: Vec<String>,
		}

		#[derive(Serialize)]
		struct Team {
			name: String,
			people: Vec<Person>,
			metadata: BTreeMap<String, String>,
		}

		let team = Team {
			name: "Readers".to_string(),
			people: vec![
				Person {
					name: "caTEXAS".to_string(),
					roles: vec!["admin".to_string(), "user".to_string()],
				},
				Person { name: "CAt".to_string(), roles: Vec::new() },
			],
			metadata: BTreeMap::from([
				("region".to_string(), "Chat Land".to_string()),
				("tier".to_string(), "silly".to_string()),
			]),
		};
		let expected = r#"{
	:name "Readers",
	:people [
		{
			:name "caTEXAS",
			:roles [
				"admin"
				"user"
			]
		}
		{
			:name "CAt",
			:roles []
		}
	],
	:metadata {
		"region" "Chat Land",
		"tier" "silly"
	}
}"#;

		let pretty = to_string_pretty(&team).unwrap();
		assert_eq!(pretty, expected);
		assert_eq!(clojure_reader::to_string_pretty(&team).unwrap(), expected);
		assert!(clojure_reader::edn::read_string(&pretty).is_ok());
	}

	#[derive(Serialize)]
	#[serde(untagged)]
	enum Nested {
		Scalar(i64),
		Sequence(Vec<Self>),
		Map(BTreeMap<String, Self>),
	}

	fn nested_sequences(mut value: Nested, depth: usize) -> Nested {
		for _ in 0..depth {
			value = Nested::Sequence(vec![value]);
		}
		value
	}

	#[test]
	fn pretty_falls_back_to_compact_formatting() {
		let pretty = to_string_pretty(&nested_sequences(Nested::Scalar(0), 45)).unwrap();
		let compact_line = alloc::format!("{}[[[0]]]", "\t".repeat(42));

		assert!(pretty.lines().any(|line| line == compact_line));
		assert_eq!(
			pretty.lines().map(|line| line.chars().take_while(|c| *c == '\t').count()).max(),
			Some(42)
		);
		assert!(clojure_reader::edn::read_string(&pretty).is_ok());
		assert_eq!(to_string(&nested_sequences(Nested::Scalar(0), 3)).unwrap(), "[[[0]]]");

		let deeper = to_string_pretty(&nested_sequences(Nested::Scalar(0), 145)).unwrap();
		assert_eq!(deeper.len() - pretty.len(), 200);
	}

	#[test]
	fn compact_fallback_preserves_collection_separators() {
		let indent = "\t".repeat(42);
		let sequence =
			nested_sequences(Nested::Sequence(vec![Nested::Scalar(1), Nested::Scalar(2)]), 42);
		let map = nested_sequences(
			Nested::Map(BTreeMap::from([
				("a".to_string(), Nested::Scalar(1)),
				("b".to_string(), Nested::Scalar(2)),
			])),
			42,
		);

		for (value, compact_line) in [(sequence, "[1 2]"), (map, r#"{"a" 1, "b" 2}"#)] {
			let pretty = to_string_pretty(&value).unwrap();
			assert!(pretty.lines().any(|line| line == alloc::format!("{indent}{compact_line}")));
			assert!(clojure_reader::edn::read_string(&pretty).is_ok());
		}
	}

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
	fn strings_are_escaped() {
		let value = "a\"b\\c\n\r\t\u{0001}";
		assert_eq!(to_string(&value).unwrap(), "\"a\\\"b\\\\c\\n\\r\\t\u{0001}\"");
	}

	#[test]
	fn empty_string_serializes() {
		assert_eq!(to_string(&"").unwrap(), "\"\"");
	}

	#[test]
	fn each_escape_serializes() {
		// Each character that needs escaping is emitted with its backslash sequence.
		assert_eq!(to_string(&"\t").unwrap(), r#""\t""#);
		assert_eq!(to_string(&"\r").unwrap(), r#""\r""#);
		assert_eq!(to_string(&"\n").unwrap(), r#""\n""#);
		assert_eq!(to_string(&"\\").unwrap(), r#""\\""#);
		assert_eq!(to_string(&"\"").unwrap(), r#""\"""#);
		// Ordinary and multi-byte characters pass through untouched.
		assert_eq!(to_string(&"猫").unwrap(), "\"猫\"");
	}

	#[test]
	fn sequence_separator_uses_state() {
		assert_eq!(to_string(&vec!['[', 'x']).unwrap(), r#"[\[ \x]"#);
	}

	#[test]
	fn large_unsigned_values() {
		#[cfg(feature = "arbitrary-nums")]
		assert_eq!(to_string(&u64::MAX).unwrap(), "18446744073709551615N");

		#[cfg(not(feature = "arbitrary-nums"))]
		assert!(to_string(&u64::MAX).is_err());
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
	fn internally_tagged_enum() {
		#[derive(Serialize)]
		#[serde(tag = "cafe", rename_all = "kebab-case", rename_all_fields = "kebab-case")]
		enum CatCafe {
			BeforeOpening,
			HerdingKittens { count: u8 },
			ServingTreats { bowls: u8, favorite_flavor: String },
			NapTime { sunny_spots: u8 },
		}

		assert_eq!(r#"{:cafe "before-opening"}"#, to_string(&CatCafe::BeforeOpening).unwrap());
		assert_eq!(
			r#"{:cafe "herding-kittens", :count 7}"#,
			to_string(&CatCafe::HerdingKittens { count: 7 }).unwrap()
		);
		assert_eq!(
			r#"{:cafe "serving-treats", :bowls 3, :favorite-flavor "salmon"}"#,
			to_string(&CatCafe::ServingTreats { bowls: 3, favorite_flavor: "salmon".to_string() })
				.unwrap()
		);
		assert_eq!(
			r#"{:cafe "nap-time", :sunny-spots 2}"#,
			to_string(&CatCafe::NapTime { sunny_spots: 2 }).unwrap()
		);
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

	#[test]
	fn direct_serialize_bytes() {
		struct Bytes([u8; 3]);

		impl serde::Serialize for Bytes {
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: serde::Serializer,
			{
				serializer.serialize_bytes(&self.0)
			}
		}

		assert_eq!("[1 2 3]", to_string(&Bytes([1, 2, 3])).unwrap());
	}

	#[test]
	fn serialize_custom_error() {
		struct Fails;

		impl serde::Serialize for Fails {
			fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
			where
				S: serde::Serializer,
			{
				Err(ser::Error::custom("silly cats"))
			}
		}

		assert_eq!(
			format!("{:?}", to_string(&Fails)),
			"Err(EdnError { code: Serde(\"silly cats\"), line: None, column: None, ptr: None })"
		);
	}
}
