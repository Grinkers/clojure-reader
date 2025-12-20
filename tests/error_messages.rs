use clojure_reader::edn;

fn err_as_string(s: &str) -> String {
  let err = edn::read_string(s).err().unwrap();
  format!("{err:?}")
}

#[test]
fn duplicates() {
  assert_eq!(
    err_as_string(
      "{:cat 42
                  :cat 0x42}"
    ),
    "EdnError { code: HashMapDuplicateKey, line: Some(2), column: Some(28), ptr: Some(36) }"
  );
  assert_eq!(
    err_as_string("#{:cat 1 2 [42] 2}"),
    "EdnError { code: SetDuplicateKey, line: Some(1), column: Some(18), ptr: Some(17) }"
  );

  assert_eq!(
    err_as_string("{#foo :a 1 #foo :a 2}"),
    "EdnError { code: HashMapDuplicateKey, line: Some(1), column: Some(21), ptr: Some(20) }"
  );

  assert_eq!(
    err_as_string("#{#bar :x #bar :x}"),
    "EdnError { code: SetDuplicateKey, line: Some(1), column: Some(18), ptr: Some(17) }"
  );

  assert_eq!(
    err_as_string("{:a 1 :a [2]}"),
    "EdnError { code: HashMapDuplicateKey, line: Some(1), column: Some(13), ptr: Some(12) }"
  );
}

#[test]
fn unbalanced_forms() {
  assert_eq!(
    err_as_string("(car (cdr) cdrrdrdrr (so (many (parens ())))"),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(45), ptr: Some(44) }"
  );

  assert_eq!(
    err_as_string("{:foo 42 :bar)"),
    "EdnError { code: UnmatchedDelimiter(')'), line: Some(1), column: Some(14), ptr: Some(13) }"
  );

  assert_eq!(
    err_as_string("#inst"),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(6), ptr: Some(5) }"
  );
}

#[test]
fn parse_invalid_ints() {
  assert_eq!(
    err_as_string("42invalid123"),
    "EdnError { code: InvalidNumber, line: Some(1), column: Some(1), ptr: Some(0) }"
  );
  assert_eq!(
    err_as_string("0xxyz123"),
    "EdnError { code: InvalidNumber, line: Some(1), column: Some(1), ptr: Some(0) }"
  );
  assert_eq!(
    err_as_string("42rabcxzy"),
    "EdnError { code: InvalidRadix(Some(42)), line: Some(1), column: Some(1), ptr: Some(0) }"
  );
  assert_eq!(
    err_as_string("42crazyrabcxzy"),
    "EdnError { code: InvalidRadix(None), line: Some(1), column: Some(1), ptr: Some(0) }"
  );
}

#[test]
fn test_invalid_radix() {
  assert_eq!(
    err_as_string("4r42"),
    "EdnError { code: InvalidNumber, line: Some(1), column: Some(1), ptr: Some(0) }"
  );
  assert_eq!(
    err_as_string("0x4.20"),
    "EdnError { code: InvalidNumber, line: Some(1), column: Some(1), ptr: Some(0) }"
  );
}

#[test]
fn parse_tag_no_end() {
  assert_eq!(
    err_as_string(r"#Unit"),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(6), ptr: Some(5) }"
  );
  assert_eq!(
    err_as_string(r#"#Unit "lolnoendingquote"#),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(24), ptr: Some(23) }"
  );
  assert_eq!(
    err_as_string(r"#Unit ;"),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(7), ptr: Some(7) }"
  );
}

#[test]
fn parse_symbol_with_quotes() {
  assert_eq!(
    err_as_string(r#"[thingy" c]"#),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(12), ptr: Some(11) }"
  );

  assert_eq!(
    err_as_string(r#"{"thingy c}"#),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(12), ptr: Some(11) }"
  );

  assert_eq!(
    err_as_string(
      r#"[thingy\"
c]"#
    ),
    "EdnError { code: UnexpectedEOF, line: Some(2), column: Some(3), ptr: Some(12) }"
  );
}

#[test]
fn unmatched_closing_delimiter_at_top_level() {
  assert_eq!(
    err_as_string("}"),
    "EdnError { code: UnmatchedDelimiter('}'), line: Some(1), column: Some(1), ptr: Some(0) }"
  );
}

#[test]
fn unmatched_delimiter_in_tag_context() {
  assert_eq!(
    err_as_string("#tag }"),
    "EdnError { code: UnmatchedDelimiter('}'), line: Some(1), column: Some(6), ptr: Some(5) }"
  );
}

#[test]
fn map_with_odd_elements() {
  assert_eq!(
    err_as_string("{:a}"),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(4), ptr: Some(3) }"
  );
}

#[test]
fn test_unexpected_eof_in_tag() {
  assert_eq!(
    err_as_string("#tag "),
    "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(6), ptr: Some(5) }"
  );
}
