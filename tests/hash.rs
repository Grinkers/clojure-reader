#[cfg(feature = "std")]
mod test {
  use std::hash::{DefaultHasher, Hash, Hasher};

  use clojure_reader::edn;

  fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
  }

  #[test]
  fn test_name() {
    let edn1 = "{
        :cat \"猫\" ; this is utf-8
        :num -0x9042
        :r 42/4242
        #_#_:num 9042
        {:foo \"bar\"} \"foobar\"
        ; dae paren
        :lisp (())
    }";

    let edn2 = "{
        :num -0x9042
        :r 42/4242
        :cat \"猫\"
        {:foo \"bar\"} \"foobar\"
        :lisp (())
    }";

    let edn_notsame = "{
        :num -0x9043
        :r 42/4242
        :cat \"猫\"
        {:foo \"bar\"} \"foobar\"
        :lisp (())
    }";

    let (edn1, edn2) = (edn::read_string(edn1).unwrap(), edn::read_string(edn2).unwrap());

    assert_eq!(calculate_hash(&edn1), calculate_hash(&edn2));
    assert_ne!(calculate_hash(&edn1), calculate_hash(&edn_notsame));
  }
}
