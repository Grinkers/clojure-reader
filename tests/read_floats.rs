#[cfg(test)]
#[cfg(feature = "floats")]
mod test {
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
}
