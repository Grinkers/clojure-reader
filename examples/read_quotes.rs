use clojure_reader::edn::{self, Edn};

// Recursively traverse the Edn struct and wrap quote around all quoted items.
fn wrap_quote(edn: Edn<'_>) -> Edn<'_> {
  match edn {
    Edn::Symbol(sym) => sym.strip_prefix('\'').map_or_else(
      || edn::read_string(sym).unwrap(),
      |strip| Edn::List(vec![Edn::Symbol("quote"), edn::read_string(strip).unwrap()]),
    ),
    Edn::List(edn) => {
      let mut list = vec![];
      let mut edn = edn.into_iter();

      while let Some(e) = edn.next() {
        if e == Edn::Symbol("'") {
          if let Some(e) = edn.next() {
            list.push(Edn::List(vec![Edn::Symbol("quote"), wrap_quote(e)]));
          } else {
            list.push(Edn::Symbol("quote"));
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

  let edn = if edn == Edn::Symbol("'") {
    Edn::List(vec![Edn::Symbol("quote"), edn::read_string(rest).unwrap()])
  } else {
    edn
  };

  let edn = wrap_quote(edn);
  edn
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
