#![no_main]

use clojure_reader::edn;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  if let Ok(s) = std::str::from_utf8(data) {
    let _ = edn::read_string(s);
  }
});
