use clojure_reader::edn;

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
}

#[test]
fn chars() {
  display!("[\\newline 1 \\return \\a \\space cat \\tab]");
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
