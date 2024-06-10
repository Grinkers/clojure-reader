#[cfg(test)]
mod test {
  use clojure_reader::edn;

  fn display(edn: &str, disp: &str) {
    let edn = edn::read_string(edn).unwrap();
    assert_eq!(format!("{edn}"), disp);
  }

  #[test]
  fn empty() {
    display("", "nil");
    display("#_42", "nil");
    display("[]", "[]");
    display("()", "()");
    display("{}", "{}");
    display("#{}", "#{}");
  }

  #[test]
  fn chars() {
    display(
      "[\\newline 1 \\return \\a \\space cat \\tab]",
      "[\\newline 1 \\return \\a \\space cat \\tab]",
    );
  }

  #[test]
  fn collections() {
    #[cfg(feature = "floats")]
    display("(42.42 -0x42 4/2)", "(42.42 -66 4/2)");

    display("(-0x42 [false true] 4/2 \"space cat\")", "(-66 [false true] 4/2 \"space cat\")");
    display("{:cat [1 2 3] :猫　\"cat\"}", "{:cat [1 2 3], :猫 \"cat\"}");
    display("#{:cat [1 2 3]}", "#{[1 2 3] :cat}");
  }
}
