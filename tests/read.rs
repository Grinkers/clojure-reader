extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use clojure_reader::edn::{self, Edn};

#[test]
fn parse_empty() {
  assert_eq!(edn::read_string("").unwrap(), Edn::Nil);
  assert_eq!(edn::read_string("#_42").unwrap(), Edn::Nil);
  assert_eq!(edn::read_string("[]").unwrap(), Edn::Vector(Vec::new()));
  assert_eq!(edn::read_string("()").unwrap(), Edn::List(Vec::new()));
  assert_eq!(edn::read_string("{}").unwrap(), Edn::Map(BTreeMap::new()));
}

#[test]
fn strings() {
  assert_eq!(edn::read_string("\"猫 are 猫\"").unwrap(), Edn::Str("猫 are 猫"));

  assert_eq!(edn::read_string(r#""foo\rbar""#).unwrap(), Edn::Str("foo\\rbar"));
}

#[test]
fn maps() {
  let e = "{
        :cat \"猫\" ; this is utf-8
        :num -0x9042
        :r 42/4242
        #_#_:num 9042
        {:foo \"bar\"} \"foobar\"
        ; dae paren
        :lisp (())
    }";
  assert_eq!(
    edn::read_string(e).unwrap(),
    Edn::Map(BTreeMap::from([
      (Edn::Key("cat"), Edn::Str("猫")),
      (Edn::Key("num"), Edn::Int(-36930)),
      (Edn::Map(BTreeMap::from([(Edn::Key("foo"), Edn::Str("bar"))])), Edn::Str("foobar")),
      (Edn::Key("r"), Edn::Rational((42, 4242))),
      (Edn::Key("lisp"), Edn::List(vec![Edn::List(vec![])])),
    ]))
  );
}

#[test]
fn whitespace() {
  let expected_result = Edn::Map(BTreeMap::from([(
    Edn::Key("somevec"),
    Edn::Vector(vec![Edn::Map(BTreeMap::from([(Edn::Key("value"), Edn::Int(42))]))]),
  )]));

  let e = "{:somevec
 [{:value 42},]
    }";
  assert_eq!(edn::read_string(e).unwrap(), expected_result);

  let e = "{:somevec
 [{:value 42}
]
    }";
  assert_eq!(edn::read_string(e).unwrap(), expected_result);

  let e = "{:somevec
 [ {:value 42} ; lol
]
    }";
  assert_eq!(edn::read_string(e).unwrap(), expected_result);
}

#[test]
fn sets() {
  let e = "#{:cat 1 true #{:cat true} 2 [42]}";
  assert_eq!(
    edn::read_string(e).unwrap(),
    Edn::Set(BTreeSet::from([
      Edn::Key("cat"),
      Edn::Int(1),
      Edn::Bool(true),
      Edn::Set(BTreeSet::from([Edn::Key("cat"), Edn::Bool(true)])),
      Edn::Int(2),
      (Edn::Vector(vec![Edn::Int(42)])),
    ]))
  );
}

#[test]
fn numbers() {
  assert_eq!(edn::read_string("43/5143").unwrap(), Edn::Rational((43, 5143)));
  assert_eq!(
    edn::read_string("-1190128294822145183/3023870813131455535").unwrap(),
    Edn::Rational((-1190128294822145183, 3023870813131455535))
  );
  assert_eq!(
    edn::read_string("-2477641376863858799/-8976013293400652448").unwrap(),
    Edn::Rational((-2477641376863858799, -8976013293400652448))
  );
}

#[test]
fn parse_0x_ints() {
  assert_eq!(edn::read_string("0x2a").unwrap(), Edn::Int(42));
  assert_eq!(edn::read_string("-0X2A").unwrap(), Edn::Int(-42));
  // leading plus character
  assert_eq!(edn::read_string("+42").unwrap(), Edn::Int(42));
  assert_eq!(edn::read_string("+0x2a").unwrap(), Edn::Int(42));
}

#[test]
fn parse_radix_ints() {
  assert_eq!(edn::read_string("16r2a").unwrap(), Edn::Int(42));
  assert_eq!(edn::read_string("8r63").unwrap(), Edn::Int(51));
  assert_eq!(edn::read_string("36rabcxyz").unwrap(), Edn::Int(623_741_435));
  assert_eq!(edn::read_string("-16r2a").unwrap(), Edn::Int(-42));
  assert_eq!(edn::read_string("-32rFOObar").unwrap(), Edn::Int(-529_280_347));
}

#[test]
fn lisp_quoted() {
  assert_eq!(
    edn::read_string("('(symbol))").unwrap(),
    Edn::List(vec![Edn::Symbol("'"), Edn::List(vec![Edn::Symbol("symbol"),])])
  );

  assert_eq!(
    edn::read_string("(apply + '(1 2 3))").unwrap(),
    Edn::List(vec![
      Edn::Symbol("apply"),
      Edn::Symbol("+"),
      Edn::Symbol("'"),
      Edn::List(vec![Edn::Int(1), Edn::Int(2), Edn::Int(3),])
    ])
  );

  assert_eq!(
    edn::read_string("('(''symbol'foo''bar''))").unwrap(),
    Edn::List(vec![Edn::Symbol("'"), Edn::List(vec![Edn::Symbol("''symbol'foo''bar''"),])])
  );
}

#[test]
fn numeric_like_symbols() {
  assert_eq!(edn::read_string("-foobar").unwrap(), Edn::Symbol("-foobar"));

  assert_eq!(
    edn::read_string("(+foobar +foo+bar+ +'- '-+)").unwrap(),
    Edn::List(vec![
      Edn::Symbol("+foobar"),
      Edn::Symbol("+foo+bar+"),
      Edn::Symbol("+'-"),
      Edn::Symbol("'-+"),
    ])
  );

  assert!(edn::read_string("(-foo( ba").is_err());
}

#[test]
fn special_chars() {
  assert_eq!(edn::read_string("\\c[lolthisisvalidedn").unwrap(), Edn::Char('c'),);

  let edn = "[\\space \\@ \\` \\tab \\return \\newline \\# \\% \\' \\g \\( \\* \\j \\+ \\, \\l \\- \\. \\/ \\0 \\2 \\r \\: \\; \\< \\\\ \\] \\} \\~ \\? \\_]";

  assert_eq!(
    edn::read_string(edn).unwrap(),
    Edn::Vector(vec![
      Edn::Char(' '),
      Edn::Char('@'),
      Edn::Char('`'),
      Edn::Char('\t'),
      Edn::Char('\r'),
      Edn::Char('\n'),
      Edn::Char('#'),
      Edn::Char('%'),
      Edn::Char('\''),
      Edn::Char('g'),
      Edn::Char('('),
      Edn::Char('*'),
      Edn::Char('j'),
      Edn::Char('+'),
      Edn::Char(','),
      Edn::Char('l'),
      Edn::Char('-'),
      Edn::Char('.'),
      Edn::Char('/'),
      Edn::Char('0'),
      Edn::Char('2'),
      Edn::Char('r'),
      Edn::Char(':'),
      Edn::Char(';'),
      Edn::Char('<'),
      Edn::Char('\\'),
      Edn::Char(']'),
      Edn::Char('}'),
      Edn::Char('~'),
      Edn::Char('?'),
      Edn::Char('_'),
    ])
  );
}

#[test]
fn read_forms() {
  let s = "(def foo 42)(sum '(1 2 3)) #_(foo the bar (cat)) 42 nil 2";
  let (e, s) = edn::read(s).unwrap();
  assert_eq!(e, Edn::List(vec![Edn::Symbol("def"), Edn::Symbol("foo"), Edn::Int(42)]));

  let (e, s) = edn::read(s).unwrap();
  assert_eq!(
    e,
    Edn::List(vec![
      Edn::Symbol("sum"),
      Edn::Symbol("'"),
      Edn::List(vec![Edn::Int(1), Edn::Int(2), Edn::Int(3)])
    ])
  );

  let (e, s) = edn::read(s).unwrap();
  assert_eq!(e, Edn::Int(42));

  let (e, s) = edn::read(s).unwrap();
  assert_eq!(e, Edn::Nil);

  let (e, s) = edn::read(s).unwrap();
  assert_eq!(e, Edn::Int(2));

  // EOF error
  assert!(edn::read(s).is_err());
}

#[test]
fn tagged() {
  assert_eq!(
    edn::read_string("#inst \"1985-04-12T23:20:50.52Z\"").unwrap(),
    Edn::Tagged("inst", Box::new(Edn::Str("1985-04-12T23:20:50.52Z")))
  );
  assert_eq!(edn::read_string(r#"#Unit nil"#).unwrap(), Edn::Tagged("Unit", Box::new(Edn::Nil)));
}
