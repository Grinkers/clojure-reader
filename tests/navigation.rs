use clojure_reader::edn::{self, Edn};

#[test]
fn get() {
  let e = edn::read_string("{:foo 4 :bar 2}").unwrap();

  assert_eq!(e.get(&Edn::Key("foo")), Some(&Edn::Int(4)));
  assert_eq!(e.get(&Edn::Str("foo")), None);
  assert_eq!(e.get(&Edn::Symbol(":foo")), None);
  assert_eq!(e.nth(0), None);
}

#[test]
fn nth() {
  let e = edn::read_string("[1 2 3 42 3 2 1]").unwrap();

  assert_eq!(e.nth(3), Some(&Edn::Int(42)));
  assert_eq!(e.nth(42), None);
  assert_eq!(e.get(&Edn::Str(":foo")), None);

  let e = edn::read_string("(1 2 3 42 3 2 1)").unwrap();

  assert_eq!(e.nth(3), Some(&Edn::Int(42)));
  assert_eq!(e.nth(42), None);
}
