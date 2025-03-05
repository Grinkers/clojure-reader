use clojure_reader::edn::{self};

#[test]
fn invalid_edn() {
  assert!(edn::read_string("{:foo 42 :foo 43}").is_err());
  assert!(edn::read_string("{:[0x42] 42}").is_err());
  assert!(edn::read_string("{:foo 42 :bar").is_err());
  assert!(edn::read_string("{:foo 42 :bar)").is_err());
  assert!(edn::read_string("#{1 2 3]").is_err());
  assert!(edn::read_string("#{1 2 3").is_err());
  assert!(edn::read_string("#_").is_err());
  assert!(edn::read_string(r#""\foo""#).is_err());
  assert!(edn::read_string(r#""foo"#).is_err());
  assert!(edn::read_string("\\cats").is_err());
  assert!(edn::read_string("42/").is_err());

  let edn = "{
              :cat \"猫\"
              :num -0x9042
              :floating-num 9042.9420
              :data [1 4 2]
              :lisp (car (cdr) cdrrdrdrr (so (many (parens ())))}";
  assert!(edn::read_string(edn).is_err());
}
