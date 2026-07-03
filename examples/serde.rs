use serde_derive::{Deserialize, Serialize};

use clojure_reader::{de::from_str, ser::to_string};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")] // Clojure mostly uses kebab-case
struct Test {
  int: u32,
  v: Vec<u32>,
  #[serde(alias = "foo_bar")] // This will allow :foo_bar to also work
  foo_bar: u8,
  b: bool,
  猫: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cafe", rename_all = "kebab-case", rename_all_fields = "kebab-case")]
enum CatCafe {
  BeforeOpening,
  HerdingKittens { count: u8 },
  ServingTreats { bowls: u8, favorite_flavor: String },
  NapTime { sunny_spots: u8 },
}

fn main() {
  let test = Test { int: 1, v: vec![1, 2, 3, 42], foo_bar: 32, b: false, 猫: "silly" };
  let expected = r#"{:int 1, :v [1 2 3 42], :foo-bar 32, :b false, :猫 "silly"}"#;

  let ser = to_string(&test).unwrap();
  assert_eq!(ser, expected);

  let deser = from_str::<Test>(expected).unwrap();
  assert_eq!(deser, test);

  let shift = CatCafe::ServingTreats { bowls: 3, favorite_flavor: "tuna".to_string() };
  let expected_shift = r#"{:cafe "serving-treats", :bowls 3, :favorite-flavor "tuna"}"#;

  let ser = to_string(&shift).unwrap();
  assert_eq!(ser, expected_shift);

  let deser = from_str::<CatCafe>(expected_shift).unwrap();
  assert_eq!(deser, shift);
}

#[test]
fn run() {
  main();
}
