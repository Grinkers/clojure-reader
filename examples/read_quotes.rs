use alloc::borrow::Cow;

use clojure_reader::edn::{self, Edn};

extern crate alloc;

fn wrap_symbol(sym: Cow<'_, str>) -> Edn<'_> {
	match sym {
		Cow::Borrowed(sym) => sym.strip_prefix('\'').map_or(Edn::Symbol(Cow::Borrowed(sym)), |strip| {
			Edn::List(vec![Edn::symbol("quote"), edn::read_string(strip).unwrap()])
		}),
		Cow::Owned(sym) => {
			let quoted =
				sym.strip_prefix('\'').map(|strip| edn::read_string(strip).unwrap().into_owned());
			quoted.map_or(Edn::Symbol(Cow::Owned(sym)), |quoted| {
				Edn::List(vec![Edn::symbol("quote"), quoted])
			})
		}
	}
}

// Recursively traverse the Edn struct and wrap quote around all quoted items.
fn wrap_quote(edn: Edn<'_>) -> Edn<'_> {
	match edn {
		Edn::Symbol(sym) => wrap_symbol(sym),
		Edn::List(edn) => {
			let mut list = vec![];
			let mut edn = edn.into_iter();

			while let Some(e) = edn.next() {
				if e == Edn::symbol("'") {
					if let Some(e) = edn.next() {
						list.push(Edn::List(vec![Edn::symbol("quote"), wrap_quote(e)]));
					} else {
						list.push(Edn::symbol("quote"));
					}
				} else {
					list.push(wrap_quote(e));
				}
			}

			Edn::List(list)
		}
		_ => edn,
	}
}

// Use `read` to handle the leading ' symbol.
fn quotify(s: &str) -> Edn<'_> {
	let (edn, rest) = edn::read(s).unwrap();

	let edn = if edn == Edn::symbol("'") {
		Edn::List(vec![Edn::symbol("quote"), edn::read_string(rest).unwrap()])
	} else {
		edn
	};

	wrap_quote(edn)
}

fn main() {
	let quoted = quotify("'(foo (bar '(a 'b)))");
	assert_eq!(format!("{quoted}"), "(quote (foo (bar (quote (a (quote b))))))");

	let quoted = quotify("(foo '(a))");
	assert_eq!(format!("{quoted}"), "(foo (quote (a)))");

	let quoted = quotify("'(foo the 'bar)");
	assert_eq!(format!("{quoted}"), "(quote (foo the (quote bar)))");

	let quoted = quotify("(foo the 'bar)");
	assert_eq!(format!("{quoted}"), "(foo the (quote bar))");

	let quoted = quotify("(foo the bar)");
	assert_eq!(format!("{quoted}"), "(foo the bar)");
}

#[test]
fn run() {
	main();
}
