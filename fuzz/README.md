# fuzzing

## Clojure (differential)

Generates EDN with Clojure's test generators, then round trips it through clojure-reader and
compares the result against `clojure.edn/read-string`.

```bash
bb run-dev       # rebuild the JNI library, then 4209042 iterations
lein run 100000  # custom iteration count, after `bb build-rs`
```

## libfuzzer-sys (crashes)

```bash
bb libfuzz
```

Feeds UTF-8 into `edn::read_string`, looking for crashes.
Requires [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).

Failing inputs are written to `artifacts/fuzz_target_1/`.

## TODO

Fuzz non-ednable.

Non-linux.

## License

See latest Clojure license for
[generators.clj](src/clojure/test_clojure/generators.clj)

For everything else in this repo see [../LICENSE](../LICENSE)
