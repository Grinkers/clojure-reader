#[cfg(feature = "arbitrary-nums")]
mod test {
  extern crate alloc;

  use alloc::collections::BTreeMap;

  use clojure_reader::edn::{self, Edn};

  #[test]
  fn read_big_floats() {
    assert_eq!(
      edn::read_string("42M").unwrap(),
      Edn::BigDec(bigdecimal::BigDecimal::parse_bytes("42".as_bytes(), 10).unwrap())
    );

    assert_eq!(
      edn::read_string("-1.3996481571841251E-152M").unwrap(),
      Edn::BigDec(
        bigdecimal::BigDecimal::parse_bytes("-1.3996481571841251E-152".as_bytes(), 10).unwrap()
      )
    );

    assert_eq!(
      edn::read_string("-2.360455011938172525674E205M").unwrap(),
      Edn::BigDec(
        bigdecimal::BigDecimal::parse_bytes("-2.360455011938172525674E205".as_bytes(), 10).unwrap()
      )
    );

    assert_eq!(
      edn::read_string(
        "-9304655354170190535034066704702217243422800801915302659810707651400462815375513.5421M"
      )
      .unwrap(),
      Edn::BigDec(
        bigdecimal::BigDecimal::parse_bytes(
          "-9304655354170190535034066704702217243422800801915302659810707651400462815375513.5421"
            .as_bytes(),
          10
        )
        .unwrap()
      )
    );

    let edn = edn::read_string("448138248963982549519911902981549055732145970445988935547439171442425790389691873602344939683141699676854492220827805407836404504312657719667023727556018710747852192967933692542410686755603469346615056047764561389802216981090217938202581834897656872748419584976531741178547186975230381501.1183603421322982705219439895643091723101190954075187371012430M").unwrap();
    assert_eq!(
      edn,
      Edn::BigDec(bigdecimal::BigDecimal::parse_bytes(b"448138248963982549519911902981549055732145970445988935547439171442425790389691873602344939683141699676854492220827805407836404504312657719667023727556018710747852192967933692542410686755603469346615056047764561389802216981090217938202581834897656872748419584976531741178547186975230381501.1183603421322982705219439895643091723101190954075187371012430", 10).unwrap())
    );
    assert_eq!(
      format!("{edn}"),
      "448138248963982549519911902981549055732145970445988935547439171442425790389691873602344939683141699676854492220827805407836404504312657719667023727556018710747852192967933692542410686755603469346615056047764561389802216981090217938202581834897656872748419584976531741178547186975230381501.1183603421322982705219439895643091723101190954075187371012430M"
    );

    let edn =
      edn::read_string("-4.5348033558837389934098639785990458404017342027290056E-21M").unwrap();
    assert_eq!(
      edn,
      Edn::BigDec(
        bigdecimal::BigDecimal::parse_bytes(
          b"-4.5348033558837389934098639785990458404017342027290056E-21",
          10
        )
        .unwrap()
      )
    );
    assert_eq!(format!("{edn}"), "-4.5348033558837389934098639785990458404017342027290056E-21M");
  }

  #[test]
  fn read_big_ints() {
    assert_eq!(
      edn::read_string("-0x42N").unwrap(),
      Edn::BigInt(num_bigint::BigInt::parse_bytes(b"-42", 16).unwrap())
    );

    assert_eq!(
      edn::read_string("-6185933704010480393063595516995722243717761522869573").unwrap(),
      Edn::BigInt(
        num_bigint::BigInt::parse_bytes(
          b"-6185933704010480393063595516995722243717761522869573",
          10
        )
        .unwrap()
      )
    );
    assert_eq!(
      edn::read_string("17992570537833404926607477972651097").unwrap(),
      Edn::BigInt(
        num_bigint::BigInt::parse_bytes(b"17992570537833404926607477972651097", 10).unwrap()
      )
    );
    assert_eq!(
      edn::read_string("-6185933704010480393063595516995722243717761522869573N").unwrap(),
      Edn::BigInt(
        num_bigint::BigInt::parse_bytes(
          b"-6185933704010480393063595516995722243717761522869573",
          10
        )
        .unwrap()
      )
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
        (Edn::Key("cat"), Edn::Str("猫")),
        (Edn::Key("num"), Edn::Int(-36930)),
        (Edn::Double((40.42).into()), Edn::Str("forty dot forty-two")),
        (Edn::Map(BTreeMap::from([(Edn::Key("foo"), Edn::Str("bar"))])), Edn::Str("foobar")),
        (Edn::Key("r"), Edn::Rational((42, 4242))),
        (Edn::Key("lisp"), Edn::List(vec![Edn::List(vec![])])),
      ]))
    );
  }
}
