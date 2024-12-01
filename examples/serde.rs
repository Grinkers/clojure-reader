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

fn main() {
  let test = Test { int: 1, v: vec![1, 2, 3, 42], foo_bar: 32, b: false, 猫: "silly" };
  let expected = r#"{:int 1, :v [1 2 3 42], :foo-bar 32, :b false, :猫 "silly"}"#;

  let ser = to_string(&test).unwrap();
  assert_eq!(ser, expected);

  let deser = from_str::<Test>(expected).unwrap();
  assert_eq!(deser, test);
}

#[test]
fn run() {
  main();
}
