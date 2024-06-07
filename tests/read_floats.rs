#[cfg(test)]
#[cfg(feature = "floats")]
mod test {
    extern crate alloc;

    use alloc::collections::BTreeMap;

    use clojure_reader::edn::{self, Edn};

    #[test]
    fn read_floats() {
        assert_eq!(
            edn::read_string("-43.5143").unwrap(),
            Edn::Double((-43.5143).into())
        );
        assert_eq!(
            edn::read_string("999999999999999999999.0").unwrap(),
            Edn::Double(1e21f64.into())
        );
    }

    #[test]
    fn maps() {
        let e = "{
        :cat \"猫\" ; this is utf-8
        :num -0x9042
        40.42 \"forty dot forty-two\"
        :r 42/4242
        #_#_:num 9042
        {:foo \"bar\"} \"foobar\"
        ; dae paren
        :lisp (())
    }";
        assert_eq!(
            edn::read_string(e).unwrap(),
            Edn::Map(BTreeMap::from([
                (Edn::Key(":cat"), Edn::Str("猫")),
                (Edn::Key(":num"), Edn::Int(-36930)),
                (Edn::Double((40.42).into()), Edn::Str("forty dot forty-two")),
                (
                    Edn::Map(BTreeMap::from([(Edn::Key(":foo"), Edn::Str("bar"))])),
                    Edn::Str("foobar")
                ),
                (Edn::Key(":r"), Edn::Rational((42, 4242))),
                (Edn::Key(":lisp"), Edn::List(vec![Edn::List(vec![])])),
            ]))
        );
    }
}
