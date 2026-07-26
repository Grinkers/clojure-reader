use std::collections::BTreeMap;

use clojure_reader::edn::{self, Edn};

#[macro_export]
macro_rules! display {
	($input:expr) => {
		let edn = edn::read_string($input).unwrap();
		assert_eq!($input, format!("{edn}"));
	};
}

#[macro_export]
macro_rules! display_diff {
	($expected:expr, $input:expr) => {
		let edn = edn::read_string($input).unwrap();
		assert_eq!($expected, format!("{edn}"));
	};
}

#[test]
fn empty() {
	display_diff!("nil", "");
	display_diff!("nil", "#_42");
	display!("[]");
	display!("()");
	display!("{}");
	display!("#{}");
	assert_eq!(format!("{:#}", Edn::Map(BTreeMap::new())), "{}");
}

#[test]
fn chars() {
	display!("[\\newline 1 \\return \\a \\space cat \\tab]");
}

#[test]
fn strings_are_not_escaped() {
	let value = "a\"b\\c\n\r\t\u{0001}";
	assert_eq!(format!("{}", Edn::Str(value)), format!("\"{value}\""));
}

#[test]
fn pretty() {
	let edn = edn::read_string(
    r#"#app/config {:empty [], :items [{:name "Gato", :roles #{:admin :user}} {:name "Nyanko", :roles #{}}], :pair (1 2)}"#,
  )
  .unwrap();
	let expected = r#"#app/config {
	:empty [],
	:items [
		{
			:name "Gato",
			:roles #{
				:admin
				:user
			}
		}
		{
			:name "Nyanko",
			:roles #{}
		}
	],
	:pair (
		1
		2
	)
}"#;

	let pretty = format!("{edn:#}");
	assert_eq!(pretty, expected);
	assert_eq!(edn::read_string(&pretty).unwrap(), edn);
}

fn nested_vectors(mut edn: Edn<'static>, depth: usize) -> Edn<'static> {
	for _ in 0..depth {
		edn = Edn::Vector(vec![edn]);
	}
	edn
}

#[test]
fn pretty_falls_back_to_compact_formatting() {
	let edn = nested_vectors(Edn::Int(0), 45);
	let pretty = format!("{edn:#}");
	let compact_line = format!("{}[[[0]]]", "\t".repeat(42));

	assert!(pretty.lines().any(|line| line == compact_line));
	assert_eq!(
		pretty.lines().map(|line| line.chars().take_while(|c| *c == '\t').count()).max(),
		Some(42)
	);
	assert_eq!(edn::read_string(&pretty).unwrap(), edn);

	let deeper = format!("{:#}", nested_vectors(Edn::Int(0), 145));
	assert_eq!(deeper.len() - pretty.len(), 200);
}

#[test]
fn compact_fallback_preserves_collection_separators() {
	let indent = "\t".repeat(42);
	let sequence = nested_vectors(Edn::Vector(vec![Edn::Int(1), Edn::Int(2)]), 42);
	let map = nested_vectors(
		Edn::Map(BTreeMap::from([(Edn::Key("a"), Edn::Int(1)), (Edn::Key("b"), Edn::Int(2))])),
		42,
	);

	for (edn, compact_line) in [(sequence, "[1 2]"), (map, "{:a 1, :b 2}")] {
		let pretty = format!("{edn:#}");
		assert!(pretty.lines().any(|line| line == format!("{indent}{compact_line}")));
		assert_eq!(edn::read_string(&pretty).unwrap(), edn);
	}
}

#[test]
fn collections() {
	#[cfg(feature = "floats")]
	display_diff!("(42.42 -66 4/2)", "(42.42 -0x42 4/2)");

	display_diff!("(-66 [false true] 4/2 \"space cat\")", "(-0x42 [false true] 4/2 \"space cat\")");
	display_diff!("{:cat [1 2 3], :猫 \"cat\"}", "{:cat [1 2 3] :猫　\"cat\"}");
	display_diff!("#{[1 2 3] :cat}", "#{:cat [1 2 3]}");
}

#[test]
#[cfg(feature = "arbitrary-nums")]
fn big_nums() {
	display!(
		"25631065767070977971462822130252989343291119843231829652358861549262445684189654378457649724823121375N"
	);

	display!(
		"45533659404590722935254870489403960444959108372566386371357004239357270213019055901312414981294872683212749959873522868216826382578289817566392464917746662928109689171949217403409185837530932882624331531998632400815620054542713762280785035186327752072979942320295706796108096781665970065634683955918435131704895612661039843567687810536853204638619861042194225357509736803723290261076155277120119270233712439357368913371347215210502655654790616883402061480163224326969915678360740686578777470546892660441110005181166215376621505.4356433869379847093529339406319840574059236551822608991350048836535371M"
	);
}

#[test]
fn tagged() {
	display!("#inst \"1985-04-12T23:20:50.52Z\"");
	display!("#uuid \"f81d4fae-7dec-11d0-a765-00a0c91e6bf6\"");
}
