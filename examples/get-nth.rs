use std::collections::BTreeMap;

use clojure_reader::edn::{self, Edn};

fn maybe_forty_two<'a>(edn: &'a Edn<'a>) -> Option<&'a Edn<'a>> {
  // This roughly tries to match clojure's get and nth
  // (-> (clojure.edn/read-string "{:foo {猫 {{:foo :bar} [1 2 42 3]}}}")
  //   (get :foo)
  //   (get (symbol "猫"))
  //   (get {:foo :bar})
  //   (nth 2))
  edn
    .get(&Edn::Key("foo"))?
    .get(&Edn::Symbol("猫"))?
    .get(&Edn::Map(BTreeMap::from([(Edn::Key("foo"), Edn::Key("bar"))])))?
    .nth(2)
}

fn namespace_get_contains() {
  // (def edn-data (edn/read-string "#:thingy {:foo \"bar\" :baz/bar \"qux\" 42 24}"))
  let edn_data = edn::read_string(r#"#:thingy {:foo "bar" :baz/bar "qux" 42 24}"#).unwrap();

  // (get edn-data 42)          -> 24
  assert_eq!(edn_data.get(&Edn::Int(42)), Some(&Edn::Int(24)));
  // (get edn-data :foo)        -> nil
  assert_eq!(edn_data.get(&Edn::Key("foo")), None);
  // (get edn-data :thingy/foo) -> "bar"
  assert_eq!(edn_data.get(&Edn::Key("thingy/foo")), Some(&Edn::Str("bar")));
  // (get edn-data :baz/bar)    -> "qux"
  assert_eq!(edn_data.get(&Edn::Key("baz/bar")), Some(&Edn::Str("qux")));

  // (contains? edn-data 42) -> true
  assert!(edn_data.contains(&Edn::Int(42)));
  // (contains? edn-data "42") -> false
  assert!(!edn_data.contains(&Edn::Str("42")));
  // (contains? edn-data :foo) -> false
  assert!(!edn_data.contains(&Edn::Key("foo")));
  // (contains? edn-data :thingy/foo) -> true
  assert!(edn_data.contains(&Edn::Key("thingy/foo")));
  // (contains? edn-data :baz/bar) -> true
  assert!(edn_data.contains(&Edn::Key("baz/bar")));
  // (contains? edn-data :bar/baz) -> false
  assert!(!edn_data.contains(&Edn::Key("bar/baz")));
}

fn main() {
  let e = edn::read_string("{:foo {猫 {{:foo :bar} [1 2 42 3]}}}").unwrap();
  let edn = maybe_forty_two(&e).unwrap();
  assert_eq!(edn, &Edn::Int(42));

  namespace_get_contains();
}

#[test]
fn run() {
  main();
}
