#[cfg(feature = "floats")]
mod test {
	extern crate alloc;

	use alloc::collections::BTreeMap;

	use clojure_reader::edn::{self, Edn};

	#[test]
	fn read_floats() {
		assert_eq!(edn::read_string("-43.5143").unwrap(), Edn::Double((-43.5143).into()));
		assert_eq!(edn::read_string("999999999999999999999.0").unwrap(), Edn::Double(1e21f64.into()));
	}

	#[test]
	fn read_symbolic_values() {
		use clojure_reader::error::Code;

		let cases = [("##Inf", f64::INFINITY), ("##-Inf", f64::NEG_INFINITY), ("##NaN", f64::NAN)];
		for (input, expected) in cases {
			assert_eq!(edn::read_string(input).unwrap(), Edn::Double(expected.into()), "{input}");
		}
		assert_eq!(
			edn::read_string("[##Inf]").unwrap(),
			Edn::Vector(vec![Edn::Double(f64::INFINITY.into())])
		);

		// Clojure reads the symbol as the next form, so whitespace and discards may precede it.
		for input in ["## Inf", "## #_x Inf"] {
			assert_eq!(edn::read_string(input).unwrap(), Edn::Double(f64::INFINITY.into()), "{input}");
		}

		for input in
			["##inf", "##+Inf", "##Foo", "##Inf1", "##:Inf", r#"##"Inf""#, "##[Inf]", "####Inf"]
		{
			assert_eq!(edn::read_string(input).unwrap_err().code, Code::InvalidNumber, "{input}");
		}
		assert_eq!(edn::read_string("##").unwrap_err().code, Code::UnexpectedEOF);
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
				(Edn::Key("cat".into()), Edn::Str("猫".into())),
				(Edn::Key("num".into()), Edn::Int(-36930)),
				(Edn::Double((40.42).into()), Edn::Str("forty dot forty-two".into())),
				(
					Edn::Map(BTreeMap::from([(Edn::Key("foo".into()), Edn::Str("bar".into()))])),
					Edn::Str("foobar".into())
				),
				(Edn::Key("r".into()), Edn::Rational((42, 4242))),
				(Edn::Key("lisp".into()), Edn::List(vec![Edn::List(vec![])])),
			]))
		);
	}
}
