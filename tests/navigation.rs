extern crate alloc;

use alloc::collections::BTreeMap;

use clojure_reader::edn::{self, Edn};

#[test]
fn get() {
	let e = edn::read_string("{:foo 4 :bar 2}").unwrap();

	assert_eq!(e.get(&Edn::Key("foo".into())), Some(&Edn::Int(4)));
	assert_eq!(e.get(&Edn::Str("foo".into())), None);
	assert_eq!(e.get(&Edn::Symbol(":foo".into())), None);
	assert_eq!(e.nth(0), None);
}

#[test]
fn nth() {
	let e = edn::read_string("[1 2 3 42 3 2 1]").unwrap();

	assert_eq!(e.nth(3), Some(&Edn::Int(42)));
	assert_eq!(e.nth(42), None);
	assert_eq!(e.get(&Edn::Str(":foo".into())), None);

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
	];
	for v in variations {
		let cfg = edn::read_string(&v).unwrap();

		let Edn::Map(cfg) = cfg else { panic!() };
		assert_eq!(
			cfg.get(&Edn::Key("thingy".into())),
			Some(&Edn::Tagged(
				":foo".into(),
				Box::new(Edn::Map(BTreeMap::from([(Edn::Key("bar".into()), Edn::Str("baz".into()))])))
			))
		);
		assert_eq!(cfg.get(&Edn::Key("more".into())), Some(&Edn::Str("stuff".into())));
	}

	// without keyword `:` symbol.
	// the tag is parsed/preserved, but we don't support custom readers
	let variations = [
		"{:thingy #foo{:bar \"baz\"} :more \"stuff\"}",
		"{:thingy #foo {:bar \"baz\"} :more \"stuff\"}",
		"{:more \"stuff\" :thingy #foo{:bar \"baz\"}}",
	];
	for v in variations {
		let cfg = edn::read_string(&v).unwrap();

		let Edn::Map(cfg) = cfg else { panic!() };
		assert_eq!(
			cfg.get(&Edn::Key("thingy".into())),
			Some(&Edn::Tagged(
				"foo".into(),
				Box::new(Edn::Map(BTreeMap::from([(Edn::Key("bar".into()), Edn::Str("baz".into()))])))
			))
		);
		assert_eq!(cfg.get(&Edn::Key("more".into())), Some(&Edn::Str("stuff".into())));
	}
}

#[test]
fn namespace_syntax_edge_cases() {
	let edn_data = edn::read_string(r#"#:thingy {:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();

	assert_eq!(edn_data.get(&Edn::Key("thingy/f#猫o".into())), Some(&Edn::Str("bar".into())));
	assert_eq!(edn_data.get(&Edn::Key("baz/bar".into())), Some(&Edn::Str("qux".into())));
	assert_eq!(edn_data.get(&Edn::Key("foo".into())), None);
	assert_eq!(edn_data.get(&Edn::Key("baz".into())), None);
	assert_eq!(edn_data.get(&Edn::Key(":baz/bar".into())), None);
	assert_eq!(edn_data.get(&Edn::Key("thingy/".into())), None);
	assert_eq!(edn_data.get(&Edn::Key("thingy".into())), None);
	assert_eq!(edn_data.get(&Edn::Key("thingything".into())), None);

	let edn_data = edn::read_string(r#"#thingy {:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
	assert_eq!(edn_data.get(&Edn::Key("thingy/f#猫o".into())), None);
	assert_eq!(edn_data.get(&Edn::Key("baz/bar".into())), None);

	let edn_data = edn::read_string(r#"#:thingy {:foo 1}"#).unwrap();
	assert_eq!(edn_data.get(&Edn::Key("thingy/foo".into())), Some(&Edn::Int(1)));
	assert_eq!(edn_data.get(&Edn::Key("thingyfoo".into())), None);
}

#[test]
fn namespace_lookup_accepts_owned_tag_and_keys() {
	use alloc::borrow::Cow;
	use alloc::string::String;

	let tagged = Edn::Tagged(
		Cow::Owned(String::from(":thingy")),
		Box::new(Edn::Map(BTreeMap::from([(Edn::Key(Cow::Owned(String::from("foo"))), Edn::Int(42))]))),
	);

	assert_eq!(tagged.get(&Edn::Key(Cow::Owned(String::from("thingy/foo")))), Some(&Edn::Int(42)));
	assert!(tagged.contains(&Edn::Key(Cow::Owned(String::from("thingy/foo")))));
	assert_eq!(tagged.get(&Edn::Key(Cow::Borrowed("thingy/foo"))), Some(&Edn::Int(42)));
}

#[test]
fn get_contains() {
	let edn_data = edn::read_string(r#"{:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
	assert_eq!(edn_data.get(&Edn::Key("f#猫o".into())), Some(&Edn::Str("bar".into())));
	assert_eq!(edn_data.contains(&Edn::Key("f#猫o".into())), true);
	assert_eq!(edn_data.get(&Edn::Key("foo".into())), None);
	assert_eq!(edn_data.contains(&Edn::Key("foo".into())), false);

	let edn_data = edn::read_string(r#"#{:f#猫o "bar" :baz/bar "qux" 42 24}"#).unwrap();
	assert_eq!(edn_data.contains(&Edn::Key("f#猫o".into())), true);
	assert_eq!(edn_data.contains(&Edn::Int(42)), true);
	assert_eq!(edn_data.contains(&Edn::Key("foo".into())), false);

	let edn_data = edn::read_string(r#"[:f#猫o "bar" :baz/bar "qux" 42 24]"#).unwrap();
	assert_eq!(edn_data.contains(&Edn::Key("f#猫o".into())), true);
	assert_eq!(edn_data.contains(&Edn::Int(42)), true);
	assert_eq!(edn_data.contains(&Edn::Key("foo".into())), false);

	let edn_data = edn::read_string(r#"(:f#猫o "bar" :baz/bar "qux" 42 24)"#).unwrap();
	assert_eq!(edn_data.contains(&Edn::Key("f#猫o".into())), true);
	assert_eq!(edn_data.contains(&Edn::Int(42)), true);
	assert_eq!(edn_data.contains(&Edn::Key("foo".into())), false);

	let edn_data = edn::read_string(r#"42"#).unwrap();
	assert_eq!(edn_data.contains(&Edn::Key("f#猫o".into())), false);
	assert_eq!(edn_data.contains(&Edn::Int(42)), false);
	assert_eq!(edn_data.contains(&Edn::Key("foo".into())), false);
}
