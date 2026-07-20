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
fn read_nil_and_eof() {
  assert_eq!(edn::read("nil").unwrap(), (Edn::Nil, ""));
  assert_eq!(edn::read("#_42 nil").unwrap(), (Edn::Nil, ""));
  assert!(edn::read("").is_err());
  assert!(edn::read("#_42").is_err());
}

#[test]
fn strings() {
  use alloc::borrow::Cow;

  let plain = edn::read_string("\"猫 are 猫\"").unwrap();
  assert_eq!(plain, Edn::Str("猫 are 猫".into()));
  assert!(matches!(plain, Edn::Str(Cow::Borrowed(_))));

  let escaped = edn::read_string(r#""foo\rbar\n\t\\\"""#).unwrap();
  assert_eq!(escaped, Edn::Str("foo\rbar\n\t\\\"".into()));
  assert!(matches!(escaped, Edn::Str(Cow::Owned(_))));
}

#[test]
fn strings_without_escapes_borrow() {
  use alloc::borrow::Cow;

  // The empty string, plain ASCII, and multi-byte UTF-8 all borrow from the input.
  for input in ["\"\"", "\"abc\"", "\"猫\""] {
    let edn = edn::read_string(input).unwrap();
    assert!(matches!(edn, Edn::Str(Cow::Borrowed(_))), "{input} should borrow");
  }

  // A literal (non-escaped) newline inside the string is not an escape sequence,
  // so the value is still borrowed verbatim.
  let multiline = edn::read_string("\"a\nb\"").unwrap();
  assert_eq!(multiline, Edn::Str("a\nb".into()));
  assert!(matches!(multiline, Edn::Str(Cow::Borrowed(_))));
}

#[test]
fn strings_each_escape_decodes() {
  use alloc::borrow::Cow;

  // Every supported escape decodes to its control character and forces an owned Cow.
  let cases =
    [(r#""\t""#, "\t"), (r#""\r""#, "\r"), (r#""\n""#, "\n"), (r#""\\""#, "\\"), (r#""\"""#, "\"")];
  for (input, expected) in cases {
    let edn = edn::read_string(input).unwrap();
    assert_eq!(edn, Edn::Str(expected.into()), "decoding {input}");
    assert!(matches!(edn, Edn::Str(Cow::Owned(_))), "{input} should own");
  }

  // Escapes at the start, middle, and end of a string, plus consecutive escapes.
  assert_eq!(edn::read_string(r#""\nabc""#).unwrap(), Edn::Str("\nabc".into()));
  assert_eq!(edn::read_string(r#""a\nb""#).unwrap(), Edn::Str("a\nb".into()));
  assert_eq!(edn::read_string(r#""abc\n""#).unwrap(), Edn::Str("abc\n".into()));
  assert_eq!(edn::read_string(r#""\\\\""#).unwrap(), Edn::Str("\\\\".into()));
  assert_eq!(edn::read_string(r#""a\"b""#).unwrap(), Edn::Str("a\"b".into()));
}

#[test]
fn strings_owned_and_borrowed_compare_equal() {
  // A decoded (owned) string must be equal to and hash/order the same as an
  // identical borrowed string. This matters because `Edn` is used as a map/set key.
  let owned = edn::read_string(r#""a\nb""#).unwrap();
  let borrowed = edn::read_string("\"a\nb\"").unwrap();
  assert_eq!(owned, borrowed);

  let map = edn::read_string(r#"{"a\nb" 1}"#).unwrap();
  assert_eq!(map.get(&Edn::Str("a\nb".into())), Some(&Edn::Int(1)));
}

#[test]
fn strings_invalid_escapes_are_rejected() {
  use clojure_reader::error::Code;

  // Unsupported escape sequences (including \b and \f which Clojure allows but this
  // crate does not, and \uNNNN unicode escapes) must error rather than silently pass.
  for input in [r#""\x""#, r#""\f""#, r#""\b""#, r#""\0""#, r#""\u0041""#, "\"\\猫\""] {
    let err = edn::read_string(input).unwrap_err();
    assert_eq!(err.code, Code::InvalidEscape, "{input} should be InvalidEscape");
  }

  // A trailing lone backslash (escape with nothing after it) hits EOF.
  assert_eq!(edn::read_string("\"abc\\").unwrap_err().code, Code::UnexpectedEOF);
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
      (Edn::Key("cat"), Edn::Str("猫".into())),
      (Edn::Key("num"), Edn::Int(-36930)),
      (
        Edn::Map(BTreeMap::from([(Edn::Key("foo"), Edn::Str("bar".into()))])),
        Edn::Str("foobar".into())
      ),
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

  let e = "{:somevec,[{:value,42}]}";
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
  assert_eq!(edn::read_string("42 43").unwrap(), Edn::Int(42));
  assert_eq!(edn::read_string("-9223372036854775808").unwrap(), Edn::Int(i64::MIN));
  assert_eq!(
    edn::read_string("-1190128294822145183/3023870813131455535").unwrap(),
    Edn::Rational((-1190128294822145183, 3023870813131455535))
  );

  #[cfg(not(feature = "arbitrary-nums"))]
  assert!(edn::read_string("9223372036854775808").is_err());
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
fn numeric_like_symbols_keywords() {
  assert_eq!(edn::read_string("-foobar").unwrap(), Edn::Symbol("-foobar"));
  assert_eq!(edn::read_string("-:thi#n=g").unwrap(), Edn::Symbol("-:thi#n=g"));
  assert_eq!(edn::read_string(":thi#n=g").unwrap(), Edn::Key("thi#n=g"));

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
  assert_eq!(edn::read_string("\\猫").unwrap(), Edn::Char('猫'));

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
fn comments_can_end_with_cr() {
  assert_eq!(edn::read_string(";comment\r42").unwrap(), Edn::Int(42));
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
    Edn::Tagged("inst", Box::new(Edn::Str("1985-04-12T23:20:50.52Z".into())))
  );
  assert_eq!(edn::read_string(r"#Unit nil").unwrap(), Edn::Tagged("Unit", Box::new(Edn::Nil)));
  assert_eq!(edn::read_string("#foo/bar nil").unwrap(), Edn::Tagged("foo/bar", Box::new(Edn::Nil)));
  assert_eq!(edn::read_string("#tag42 nil").unwrap(), Edn::Tagged("tag42", Box::new(Edn::Nil)));
  assert_eq!(edn::read_string("#foo:bar nil").unwrap(), Edn::Tagged("foo:bar", Box::new(Edn::Nil)));
  assert_eq!(edn::read_string("#foo#bar nil").unwrap(), Edn::Tagged("foo#bar", Box::new(Edn::Nil)));
  assert_eq!(
    edn::read_string("#foo/-bar nil").unwrap(),
    Edn::Tagged("foo/-bar", Box::new(Edn::Nil))
  );
  assert_eq!(
    edn::read_string("#:foo {}").unwrap(),
    Edn::Tagged(":foo", Box::new(Edn::Map(BTreeMap::new())))
  );
  assert_eq!(
    edn::read_string("#foo\"bar\"").unwrap(),
    Edn::Tagged("foo", Box::new(Edn::Str("bar".into())))
  );

  assert_eq!(
    edn::read_string("#pow2 #pow3 2").unwrap(),
    Edn::Tagged("pow2", Box::new(Edn::Tagged("pow3", Box::new(Edn::Int(2)))))
  );

  assert_eq!(
    edn::read_string("#foo #bar #ニャンキャット {:baz #tag42 \"wut\"}").unwrap(),
    Edn::Tagged(
      "foo",
      Box::new(Edn::Tagged(
        "bar",
        Box::new(Edn::Tagged(
          "ニャンキャット",
          Box::new(Edn::Map(BTreeMap::from([(
            Edn::Key("baz"),
            Edn::Tagged("tag42", Box::new(Edn::Str("wut".into())))
          )])))
        ))
      ))
    )
  );
}

#[test]
fn discard_tagged_values() {
  assert_eq!(edn::read_string("[#_ #foo 1]").unwrap(), Edn::Vector(vec![]));
  assert_eq!(edn::read_string("[#_ #foo 1 2]").unwrap(), Edn::Vector(vec![Edn::Int(2)]));
  assert_eq!(edn::read_string("#_ #foo 1").unwrap(), Edn::Nil);

  assert_eq!(edn::read_string("[#_ #{1 1}]").unwrap(), Edn::Vector(vec![]));
  assert_eq!(edn::read_string("#_ {:a 1 :a 2}").unwrap(), Edn::Nil);
  assert_eq!(edn::read_string("#_ #foo #{1 1}").unwrap(), Edn::Nil);
  assert_eq!(edn::read_string("#_ [#{1 1}]").unwrap(), Edn::Nil);
}
