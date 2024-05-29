use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let edn = "{
        :cat \"猫\"
        :num -0x9042
        :floating-num 9042.9420
        :data [1 4 2]
        :lisp (car (cdr) cdrrdrdrr (so (many (parens ()))))
    }";
    c.bench_function("parse", |b| {
        b.iter(|| clojure_reader::edn::read_string(edn))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
