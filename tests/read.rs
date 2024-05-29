#[cfg(test)]
mod test {
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
        assert_eq!(
            edn::read_string("\"猫 are 猫\"").unwrap(),
            Edn::Str("猫 are 猫")
        );

        assert_eq!(
            edn::read_string(r#""foo\rbar""#).unwrap(),
            Edn::Str("foo\\rbar")
        );
    }

    #[test]
    fn maps() {
        let e = "{
        :cat \"猫\" ; this is utf-8
        :num -0x9042
        #_#_:num 9042
        ; dae paren
        :lisp (())
    }";
        assert_eq!(
            edn::read_string(e).unwrap(),
            Edn::Map(BTreeMap::from([
                (Edn::Key(":cat"), Edn::Str("猫")),
                (Edn::Key(":num"), Edn::Int(-36930)),
                (Edn::Key(":lisp"), Edn::List(vec![Edn::List(vec![])])),
            ]))
        );
    }

    #[test]
    fn sets() {
        let e = "#{:cat 1 2 [42]}";
        assert_eq!(
            edn::read_string(e).unwrap(),
            Edn::Set(BTreeSet::from([
                Edn::Key(":cat"),
                Edn::Int(1),
                Edn::Int(2),
                (Edn::Vector(vec![Edn::Int(42)])),
            ]))
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            edn::read_string("43/5143").unwrap(),
            Edn::Rational((43, 5143))
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
        assert_eq!(
            edn::read_string("36rabcxyz").unwrap(),
            Edn::Int(623_741_435)
        );
        assert_eq!(edn::read_string("-16r2a").unwrap(), Edn::Int(-42));
        assert_eq!(
            edn::read_string("-32rFOObar").unwrap(),
            Edn::Int(-529_280_347)
        );
    }

    #[test]
    fn lisp_quoted() {
        assert_eq!(
            edn::read_string("('(symbol))").unwrap(),
            Edn::List(vec![
                Edn::Symbol("'"),
                Edn::List(vec![Edn::Symbol("symbol"),])
            ])
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
            Edn::List(vec![
                Edn::Symbol("'"),
                Edn::List(vec![Edn::Symbol("''symbol'foo''bar''"),])
            ])
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
        assert_eq!(
            edn::read_string("\\c[lolthisisvalidedn").unwrap(),
            Edn::Char('c'),
        );

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
}
