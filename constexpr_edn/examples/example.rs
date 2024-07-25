use clojure_reader::edn::Edn;
use constexpr_edn::edn;

use std::collections::{BTreeMap, BTreeSet};

fn main() {
  let edn: Edn = edn!([1 2 3]);
  // Vector([Int(1), Int(2), Int(3)])
  println!("{edn:?}");

  let edn: Edn = edn!(#{ 1 2 3 #{4 5 6}});
  // Set({Set({Int(4), Int(5), Int(6)}), Int(1), Int(2), Int(3)})
  println!("{edn:?}");

  let edn: Edn = edn!({:foo #{42 43 44}});
  // Map(Map({":bar": List(List([UInt(1), UInt(2), UInt(3)])), ":foo": Set(Set({UInt(42), UInt(43), UInt(44)}))}))
  println!("{edn:?}");

  let edn: Edn = edn!([42, "foobar",,,,, ,, ; 424242
                         #_ ignoreme
                         ,, yaycats 16r9001]);
  // Vector([Int(42), Str("foobar"), Symbol("yaycats"), Int(36865)])
  println!("{edn:?}");

  let edn: Edn = edn!(42 43 44);
  // Int(42)
  println!("{edn:?}");
}
