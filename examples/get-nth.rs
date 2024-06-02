use std::collections::BTreeMap;

use clojure_reader::edn::{self, Edn};

fn maybe_forty_two<'a>(edn: &'a Edn<'a>) -> Option<&Edn<'a>> {
    // This roughly tries to match clojure's get and nth
    // (-> (clojure.edn/read-string "{:foo {猫 {{:foo :bar} [1 2 42 3]}}}")
    //   (get :foo)
    //   (get (symbol "猫"))
    //   (get {:foo :bar})
    //   (nth 2))
    edn.get(&Edn::Key(":foo"))?
        .get(&Edn::Symbol("猫"))?
        .get(&Edn::Map(BTreeMap::from([(
            Edn::Key(":foo"),
            Edn::Key(":bar"),
        )])))?
        .nth(2)
}

fn main() {
    let e = edn::read_string("{:foo {猫 {{:foo :bar} [1 2 42 3]}}}").unwrap();
    println!("{:?}", maybe_forty_two(&e));
}
