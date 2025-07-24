use criterion::{Criterion, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
enum Lisp<'a> {
  Symbol(&'a str),
  List(Vec<Lisp<'a>>),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Data<'a> {
  cat: &'a str,
  num: i64,
  #[serde(rename = "floating-num")]
  floating_num: f64,
  data: Vec<i32>,
  lisp: Vec<Lisp<'a>>,
}

fn criterion_benchmark(c: &mut Criterion) {
  let edn = "{
        :cat \"猫\"
        :num -0x9042
        :floating-num 9042.9420
        :data [1 4 2]
        :lisp (car (cdr) cdrrdrdrr (so (many (parens ()))))
    }";
  c.bench_function("parse", |b| b.iter(|| clojure_reader::edn::read_string(edn)));

  c.bench_function("deserialize", |b| {
    b.iter(|| {
      let _: Data = clojure_reader::from_str(edn).unwrap();
    })
  });

  let data = Data {
    cat: "猫",
    num: -0x9042,
    floating_num: 9042.9420,
    data: vec![1, 4, 2],
    lisp: vec![
      Lisp::Symbol("car"),
      Lisp::List(vec![Lisp::Symbol("cdr")]),
      Lisp::Symbol("cdrrdrdrr"),
      Lisp::List(vec![
        Lisp::Symbol("so"),
        Lisp::List(vec![
          Lisp::Symbol("many"),
          Lisp::List(vec![Lisp::Symbol("parens"), Lisp::List(vec![])]),
        ]),
      ]),
    ],
  };

  c.bench_function("serialize", |b| {
    b.iter(|| {
      let _ = clojure_reader::to_string(&data).unwrap();
    })
  });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
