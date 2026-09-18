use std::collections::{BTreeMap, BTreeSet};
use std::thread;

use clojure_reader::edn::Edn;

/// Run `f` on a 512 KB stack, failing the test if it overflows.
fn small_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
	thread::Builder::new()
		.stack_size(512 * 1024)
		.spawn(f)
		.expect("spawn small-stack thread")
		.join()
		.expect("operation overflowed the stack")
}

fn nested(leaf: i64, depth: usize) -> Edn<'static> {
	let mut value = Edn::Int(leaf);
	for _ in 0..depth {
		value = Edn::Vector(vec![value]);
	}
	value
}

#[cfg(feature = "serde")]
#[test]
fn trailing_form_is_rejected_without_overflowing_the_stack() {
	let input = format!("42 {}0{}", "[".repeat(20_000), "]".repeat(20_000));
	let error = small_stack(move || clojure_reader::from_str::<u8>(&input).unwrap_err());
	drop(error);
}

#[cfg(feature = "serde")]
#[test]
fn deep_trailing_discard_is_ignored_without_overflowing_the_stack() {
	let input = format!("42 #_{}0{}", "[".repeat(20_000), "]".repeat(20_000));
	let value = small_stack(move || clojure_reader::from_str::<u8>(&input).unwrap());
	assert_eq!(value, 42);
}

#[test]
fn into_owned_set_rebuild_does_not_recurse_through_keys() {
	let input = thread::Builder::new()
		.stack_size(64 * 1024 * 1024)
		.spawn(|| Edn::Set(BTreeSet::from([nested(0, 5_000), nested(1, 5_000)])))
		.unwrap()
		.join()
		.unwrap();

	let output = small_stack(move || input.into_owned());
	// Keep this test about conversion, not recursive destruction.
	std::mem::forget(output);
}

#[test]
fn into_owned_map_rebuild_does_not_recurse_through_keys() {
	let input = thread::Builder::new()
		.stack_size(64 * 1024 * 1024)
		.spawn(|| {
			Edn::Map(BTreeMap::from([(nested(0, 5_000), Edn::Nil), (nested(1, 5_000), Edn::Nil)]))
		})
		.unwrap()
		.join()
		.unwrap();

	let output = small_stack(move || input.into_owned());
	std::mem::forget(output);
}

#[test]
fn dropping_a_deep_owned_value_does_not_overflow_the_stack() {
	small_stack(|| {
		let mut value = Edn::key("deep");
		for _ in 0..10_000 {
			value = Edn::Vector(vec![value]);
		}
		drop(value.into_owned());
	});
}
