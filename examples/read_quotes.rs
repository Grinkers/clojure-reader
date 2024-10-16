use clojure_reader::edn::{self, Edn};

// Recursively traverse the Edn struct.
// To keep this example short, this only handles lists and literals.
fn wrap_quote(edn: Edn<'_>) -> Edn<'_> {
  match edn {
    Edn::Symbol(s) => s.strip_prefix('\'').map_or(Edn::Symbol(s), |stripped| {
      Edn::List(vec![Edn::Symbol("quote"), Edn::Symbol(stripped)])
    }),
    Edn::List(mut edn) => {
      edn.reverse();
      let mut list = vec![];

      while let Some(e) = edn.pop() {
        if e == Edn::Symbol("'") {
          list.push(Edn::List(vec![Edn::Symbol("quote"), wrap_quote(edn.pop().unwrap())]));
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
