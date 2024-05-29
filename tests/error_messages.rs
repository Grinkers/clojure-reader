#[cfg(test)]
mod test {
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
    }

    #[test]
    fn unimplemented() {
        assert_eq!(
            err_as_string("#inst \"1985-04-12T23:20:50.52Z\""),
            "EdnError { code: Unimplemented(\"Tagged Element\"), line: Some(1), column: Some(2), ptr: Some(1) }"
        );
    }

    #[test]
    fn unbalanced_forms() {
        assert_eq!(
            err_as_string("(car (cdr) cdrrdrdrr (so (many (parens ())))"),
            "EdnError { code: UnexpectedEOF, line: Some(1), column: Some(45), ptr: Some(44) }"
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
        assert_eq!(err_as_string("42rabcxzy"), "EdnError { code: InvalidRadix(Some(42)), line: Some(1), column: Some(1), ptr: Some(0) }");
        assert_eq!(
            err_as_string("42crazyrabcxzy"),
            "EdnError { code: InvalidRadix(None), line: Some(1), column: Some(1), ptr: Some(0) }"
        );
    }
}
