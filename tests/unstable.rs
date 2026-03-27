#[cfg(feature = "unstable")]
mod test {
  use clojure_reader::{
    edn::{self, Edn},
    error::Code,
    parse::{self, Node, NodeKind, Position, SourceReader, Span},
  };

  #[test]
  fn node_try_into_edn_matches_read_string() {
    let input = r#"#_ :ignored {:foo [1 #_2 {:bar nil}] :baz #{true #_false :qux} :tagged #inst "1985-04-12T23:20:50.52Z"} trailing"#;

    let mut reader = SourceReader::new(input);
    let node = parse::parse(&mut reader).unwrap();
    let edn = Edn::try_from(node).unwrap();

    assert_eq!(edn, edn::read_string(input).unwrap());
    assert_eq!(reader.remaining(), " trailing");
  }

  #[test]
  fn node_try_into_edn_reports_duplicate_errors() {
    let map_node = parse::parse(&mut SourceReader::new("{:a 1 :a [2]}")).unwrap();
    let map_err = Edn::try_from(map_node).unwrap_err();
    assert_eq!(map_err.code, Code::HashMapDuplicateKey);
    assert_eq!(map_err.line, Some(1));
    assert_eq!(map_err.column, Some(13));
    assert_eq!(map_err.ptr, Some(12));

    let set_node = parse::parse(&mut SourceReader::new("#{:cat 1 2 [42] 2}")).unwrap();
    let set_err = Edn::try_from(set_node).unwrap_err();
    assert_eq!(set_err.code, Code::SetDuplicateKey);
    assert_eq!(set_err.line, Some(1));
    assert_eq!(set_err.column, Some(18));
    assert_eq!(set_err.ptr, Some(17));
  }

  #[test]
  fn node_try_into_edn_covers_remaining_variants() {
    let node = Node::no_discards(
      NodeKind::List(
        vec![
          Node::no_discards(NodeKind::Symbol("sym"), Span::default()),
          Node::no_discards(NodeKind::Rational((3, 2)), Span::default()),
          Node::no_discards(NodeKind::Char('z'), Span::default()),
        ],
        vec![],
      ),
      Span::default(),
    );
    assert_eq!(
      Edn::try_from(node).unwrap(),
      Edn::List(vec![Edn::Symbol("sym"), Edn::Rational((3, 2)), Edn::Char('z')])
    );

    #[cfg(feature = "floats")]
    assert_eq!(
      Edn::try_from(Node::no_discards(NodeKind::Double((2.5).into()), Span::default())).unwrap(),
      Edn::Double((2.5).into())
    );

    #[cfg(feature = "arbitrary-nums")]
    {
      use bigdecimal::BigDecimal;
      use num_bigint::BigInt;

      assert_eq!(
        Edn::try_from(Node::no_discards(NodeKind::BigInt(BigInt::from(42)), Span::default()))
          .unwrap(),
        Edn::BigInt(BigInt::from(42))
      );
      assert_eq!(
        Edn::try_from(Node::no_discards(NodeKind::BigDec(BigDecimal::from(42)), Span::default(),))
          .unwrap(),
        Edn::BigDec(BigDecimal::from(42))
      );
    }
  }

  #[test]
  fn parse_rejects_maps_with_odd_elements() {
    let err = parse::parse(&mut SourceReader::new("{:a}")).unwrap_err();

    assert_eq!(err.code, Code::UnexpectedEOF);
    assert_eq!(err.line, Some(1));
    assert_eq!(err.column, Some(4));
    assert_eq!(err.ptr, Some(3));
  }

  #[test]
  fn source_reader_finish_keeps_source_and_position() {
    let mut reader = SourceReader::new("(cat) [42]");
    let _ = parse::parse(&mut reader).unwrap();

    let (position, source) = reader.finish();

    assert_eq!(position, Position { line: 1, column: 6, ptr: 5 });
    assert_eq!(source, "(cat) [42]");
    assert_eq!(&source[position.ptr..], " [42]");
  }
}
