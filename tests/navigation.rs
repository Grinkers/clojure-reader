extern crate alloc;

use alloc::collections::BTreeMap;

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

#[test]
fn default_map_namespace_syntax() {
  // see https://github.com/Grinkers/clojure-reader/issues/2
  let variations = [
    "{:thingy #:foo{:bar \"baz\"} :more \"stuff\"}",
    "{:thingy #:foo {:bar \"baz\"} :more \"stuff\"}",
    "{:more \"stuff\" :thingy #:foo{:bar \"baz\"}}",
    "{:more \"stuff\" :thingy # :foo{:bar \"baz\"}}",
  ];
  for v in variations {
    let cfg = edn::read_string(&v).unwrap();

    let Edn::Map(cfg) = cfg else { panic!() };
    assert_eq!(
      cfg.get(&Edn::Key("thingy")),
      Some(&Edn::Tagged(
        ":foo",
        Box::new(Edn::Map(BTreeMap::from([(Edn::Key("bar"), Edn::Str("baz"))])))
      ))
    );
    assert_eq!(cfg.get(&Edn::Key("more")), Some(&Edn::Str("stuff")));
  }

  // without keyword `:` symbol.
  // the tag is parsed/preserved, but we don't support custom readers
  let variations = [
    "{:thingy #foo{:bar \"baz\"} :more \"stuff\"}",
    "{:thingy #foo {:bar \"baz\"} :more \"stuff\"}",
    "{:more \"stuff\" :thingy #foo{:bar \"baz\"}}",
    "{:more \"stuff\" :thingy # foo{:bar \"baz\"}}",
  ];
  for v in variations {
    let cfg = edn::read_string(&v).unwrap();

    let Edn::Map(cfg) = cfg else { panic!() };
    assert_eq!(
      cfg.get(&Edn::Key("thingy")),
      Some(&Edn::Tagged(
        "foo",
        Box::new(Edn::Map(BTreeMap::from([(Edn::Key("bar"), Edn::Str("baz"))])))
      ))
    );
    assert_eq!(cfg.get(&Edn::Key("more")), Some(&Edn::Str("stuff")));
  }
}

#[test]
fn namespace_syntax_edge_cases() {
  let edn_data = edn::read_string(r#"#:thingy {:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();

  assert_eq!(edn_data.get(&Edn::Key("thingy/f#猫o")), Some(&Edn::Str("bar")));
  assert_eq!(edn_data.get(&Edn::Key("baz/bar")), Some(&Edn::Str("qux")));
  assert_eq!(edn_data.get(&Edn::Key("foo")), None);
  assert_eq!(edn_data.get(&Edn::Key("baz")), None);
  assert_eq!(edn_data.get(&Edn::Key(":baz/bar")), None);
  assert_eq!(edn_data.get(&Edn::Key("thingy/")), None);
  assert_eq!(edn_data.get(&Edn::Key("thingy")), None);
  assert_eq!(edn_data.get(&Edn::Key("thingything")), None);

  let edn_data = edn::read_string(r#"#thingy {:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
  assert_eq!(edn_data.get(&Edn::Key("thingy/f#猫o")), None);
  assert_eq!(edn_data.get(&Edn::Key("baz/bar")), None);
}

#[test]
fn get_contains() {
  let edn_data = edn::read_string(r#"{:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
  assert_eq!(edn_data.get(&Edn::Key("f#猫o")), Some(&Edn::Str("bar")));
  assert_eq!(edn_data.contains(&Edn::Key("f#猫o")), true);
  assert_eq!(edn_data.get(&Edn::Key("foo")), None);
  assert_eq!(edn_data.contains(&Edn::Key("foo")), false);

  let edn_data = edn::read_string(r#"#{:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
  assert_eq!(edn_data.contains(&Edn::Key("f#猫o")), true);
  assert_eq!(edn_data.contains(&Edn::Int(42)), true);
  assert_eq!(edn_data.contains(&Edn::Key("foo")), false);

  let edn_data = edn::read_string(r#"[:f#猫o "bar" :baz/bar "qux" 42 24]"#).unwrap();
  assert_eq!(edn_data.contains(&Edn::Key("f#猫o")), true);
  assert_eq!(edn_data.contains(&Edn::Int(42)), true);
  assert_eq!(edn_data.contains(&Edn::Key("foo")), false);

  let edn_data = edn::read_string(r#"(:f#猫o "bar" :baz/bar "qux" 42 24)"#).unwrap();
  assert_eq!(edn_data.contains(&Edn::Key("f#猫o")), true);
  assert_eq!(edn_data.contains(&Edn::Int(42)), true);
  assert_eq!(edn_data.contains(&Edn::Key("foo")), false);

  let edn_data = edn::read_string(r#"42"#).unwrap();
  assert_eq!(edn_data.contains(&Edn::Key("f#猫o")), false);
  assert_eq!(edn_data.contains(&Edn::Int(42)), false);
  assert_eq!(edn_data.contains(&Edn::Key("foo")), false);
}
