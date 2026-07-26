use clojure_reader::parse::{self, NodeKind, SourceReader};

fn underline(column: usize, width: usize) -> String {
	let mut out = " ".repeat(column.saturating_sub(1));
	out.push_str(&"^".repeat(width.max(1)));
	out
}

fn main() {
	let source = "(def foo-bar (+ 猫gatoキャット 42))";

	let mut reader = SourceReader::new(source);
	let parsed = parse::parse(&mut reader).expect("failed to parse example input");

	let NodeKind::List(items, _) = parsed.kind else {
		panic!("expected outer list");
	};
	let NodeKind::List(inner, _) = &items[2].kind else {
		panic!("expected inner list");
	};
	let cat = &inner[1];
	let span = cat.span();
	let width = source[span.0.ptr..span.1.ptr]
		.chars()
		.map(|ch| if ch.is_ascii() { 1 } else { 2 })
		.sum::<usize>();

	println!("{source}");
	println!("{} symbol not found", underline(span.0.column, width));
	println!(
		"\nspan: start=(line {}, column {}, ptr {}) end=(line {}, column {}, ptr {})",
		span.0.line, span.0.column, span.0.ptr, span.1.line, span.1.column, span.1.ptr,
	);
}

#[test]
fn run() {
	main();
}
