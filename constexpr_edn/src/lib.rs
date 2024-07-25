use proc_macro::TokenStream;
use std::str::FromStr;

use clojure_reader::edn::{self, Edn};

fn as_code(edn: Edn) -> String {
  match edn {
    Edn::Vector(vec) => {
      let mut code = String::from("Edn::Vector(vec![");
      for v in vec {
        code.push_str(&as_code(v));
        code.push(',');
      }
      code.push_str("])");
      code
    }
    Edn::Set(set) => {
      let mut code = String::from("Edn::Set(BTreeSet::from([");
      for s in set {
        code.push_str(&as_code(s));
        code.push(',');
      }
      code.push_str("]))");
      code
    }
    Edn::Map(map) => {
      let mut code = String::from("Edn::Map(BTreeMap::from([");
      let mut it = map.iter().peekable();
      while let Some(m) = it.next() {
        let (k, v) = m;
        code.push('(');
        code.push_str(&as_code(k.clone()));
        code.push(',');
        code.push_str(&as_code(v.clone()));
        code.push(')');
        if it.peek().is_some() {
          code.push(',');
        } else {
          code.push_str("]))");
        }
      }
      code
    }
    Edn::List(list) => {
      let mut code = String::from("Edn::List(vec![");
      for l in list {
        code.push_str(&as_code(l));
        code.push(',');
      }
      code.push_str("])");
      code
    }
    Edn::Symbol(sy) => format!("Edn::Symbol(\"{sy}\")"),
    Edn::Key(k) => format!("Edn::Key(\"{k}\")"),
    Edn::Str(s) => format!("Edn::Str(\"{s}\")"),
    Edn::Int(i) => format!("Edn::Int({i})"),
    Edn::Double(d) => format!("Edn::Double({d})"),
    Edn::Rational((n, d)) => format!("Edn::Rational({n}, {d})"),
    Edn::Bool(b) => format!("Edn::Bool({b})"),
    Edn::Char(c) => format!("Edn::Char({c})"),
    Edn::Nil => String::from("Edn::Nil"),
    e => todo!("Unexpected non-exhaustive Edn value: {e:?}."),
  }
}

#[proc_macro]
pub fn edn(input: TokenStream) -> TokenStream {
  let mut edn_str = String::new();
  for i in input.into_iter() {
    let s = i.span().source_text().unwrap();
    edn_str.push_str(&s);
    if !(s.starts_with('#') || s.starts_with(':')) {
      edn_str.push(' ');
    }
  }

  let edn = edn::read_string(&edn_str).unwrap();
  let code = as_code(edn);

  proc_macro::TokenStream::from_str(&code).unwrap()
}
