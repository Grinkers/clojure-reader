// Attempts to correspond to tests/read.rs as much as possible, just with `Node` instead of `Edn`
#[cfg(feature = "unstable")]
mod test {
  use clojure_reader::parse::{self, Discard, Node, NodeKind, SourceReader, Span};

  // Position
  macro_rules! p {
    ($line:expr, $column:expr, $ptr:expr) => {
      clojure_reader::parse::Position { line: $line, column: $column, ptr: $ptr }
    };
    // helpful if single-line
    ($column:expr, $ptr:expr) => {
      clojure_reader::parse::Position { line: 1, column: $column, ptr: $ptr }
    };
    // helpful if single-line and no wide characters
    ($column:expr) => {
      clojure_reader::parse::Position { line: 1, column: $column, ptr: const { $column - 1 } }
    };
  }

  #[test]
  fn parse_empty() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("")).unwrap(),
      Node::no_discards(NodeKind::Nil, Span(p!(1), p!(1)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("#_42")).unwrap(),
      Node::no_discards(NodeKind::Nil, Span(p!(1), p!(5)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("[]")).unwrap(),
      Node::no_discards(NodeKind::Vector(vec![], vec![]), Span(p!(1), p!(3)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("()")).unwrap(),
      Node::no_discards(NodeKind::List(vec![], vec![]), Span(p!(1), p!(3)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("{}")).unwrap(),
      Node::no_discards(NodeKind::Map(vec![], vec![]), Span(p!(1), p!(3)))
    );
  }

  #[test]
  fn strings() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("\"猫 are 猫\"")).unwrap(),
      Node::no_discards(
        NodeKind::Str("猫 are 猫"),
        Span(p!(1, 1, 0), p!(1, 10, /* 猫 is 3 bytes wide */ 13))
      )
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new(r#""foo\rbar""#)).unwrap(),
      Node::no_discards(NodeKind::Str("foo\\rbar"), Span(p!(1), p!(11)))
    );
  }

  #[test]
  fn maps() {
    let e = "{
        :cat \"猫\" ; this is utf-8
        :num -0x9042
        :r 42/4242
        #_#_:num 9042
        {:foo \"bar\"} \"foobar\"
        ; dae paren
        :lisp (())
        #_\"a map\"
    }";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      Node::no_discards(
        NodeKind::Map(
          Vec::from([
            (
              Node::no_discards(NodeKind::Key("cat"), Span(p!(2, 9, 10), p!(2, 13, 14))),
              Node::no_discards(NodeKind::Str("猫"), Span(p!(2, 14, 15), p!(2, 17, 20))),
            ),
            (
              Node::no_discards(NodeKind::Key("num"), Span(p!(3, 9, 45), p!(3, 13, 49))),
              Node::no_discards(NodeKind::Int(-36930), Span(p!(3, 14, 50), p!(3, 21, 57))),
            ),
            (
              Node::no_discards(NodeKind::Key("r"), Span(p!(4, 9, 66), p!(4, 11, 68))),
              Node::no_discards(NodeKind::Rational((42, 4242)), Span(p!(4, 12, 69), p!(4, 19, 76))),
            ),
            (
              Node {
                kind: NodeKind::Map(
                  Vec::from([(
                    Node::no_discards(NodeKind::Key("foo"), Span(p!(6, 10, 108), p!(6, 14, 112))),
                    Node::no_discards(NodeKind::Str("bar"), Span(p!(6, 15, 113), p!(6, 20, 118))),
                  )]),
                  vec![],
                ),
                span: Span(p!(6, 9, 107), p!(6, 21, 119)),
                leading_discards: vec![Discard(
                  Node {
                    kind: NodeKind::Int(9042),
                    span: Span(p!(5, 18, 94), p!(5, 22, 98)),
                    leading_discards: vec![Discard(
                      Node::no_discards(NodeKind::Key("num"), Span(p!(5, 13, 89), p!(5, 17, 93))),
                      Span(p!(5, 11, 87), p!(5, 17, 93))
                    )]
                  },
                  Span(p!(5, 9, 85), p!(5, 22, 98))
                )]
              },
              Node::no_discards(NodeKind::Str("foobar"), Span(p!(6, 22, 120), p!(6, 30, 128))),
            ),
            (
              Node::no_discards(NodeKind::Key("lisp"), Span(p!(8, 9, 157), p!(8, 14, 162))),
              Node::no_discards(
                NodeKind::List(
                  vec![Node {
                    kind: NodeKind::List(vec![], vec![]),
                    span: Span(p!(8, 16, 164), p!(8, 18, 166)),
                    leading_discards: vec![]
                  }],
                  vec![]
                ),
                Span(p!(8, 15, 163), p!(8, 19, 167)),
              ),
            ),
          ]),
          vec![Discard(
            Node::no_discards(NodeKind::Str("a map"), Span(p!(9, 11, 178), p!(9, 18, 185))),
            Span(p!(9, 9, 176), p!(9, 18, 185))
          )]
        ),
        Span(p!(1, 1, 0), p!(10, 6, 191)),
      ),
    );
  }

  #[test]
  fn whitespace() {
    struct SpanMap {
      outer_map: Span,
      inner_vec: Span,
      inner_map: Span,
      key_somevec: Span,
      key_value: Span,
      int_42: Span,
    }
    fn expected_result(span_map: SpanMap) -> Node<'static> {
      Node::no_discards(
        NodeKind::Map(
          Vec::from([(
            Node::no_discards(NodeKind::Key("somevec"), span_map.key_somevec),
            Node::no_discards(
              NodeKind::Vector(
                vec![Node::no_discards(
                  NodeKind::Map(
                    Vec::from([(
                      Node::no_discards(NodeKind::Key("value"), span_map.key_value),
                      Node::no_discards(NodeKind::Int(42), span_map.int_42),
                    )]),
                    vec![],
                  ),
                  span_map.inner_map,
                )],
                vec![],
              ),
              span_map.inner_vec,
            ),
          )]),
          vec![],
        ),
        span_map.outer_map,
      )
    }

    let e = "{:somevec
 [{:value 42},]
    }";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      expected_result(SpanMap {
        outer_map: Span(p!(1, 1, 0), p!(3, 6, 31)),
        inner_vec: Span(p!(2, 2, 11), p!(2, 16, 25)),
        inner_map: Span(p!(2, 3, 12), p!(2, 14, 23)),
        key_somevec: Span(p!(1, 2, 1), p!(1, 10, 9)),
        key_value: Span(p!(2, 4, 13), p!(2, 10, 19)),
        int_42: Span(p!(2, 11, 20), p!(2, 13, 22))
      })
    );

    let e = "{:somevec
 [{:value 42}
]
    }";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      expected_result(SpanMap {
        outer_map: Span(p!(1, 1, 0), p!(4, 6, 31)),
        inner_vec: Span(p!(2, 2, 11), p!(3, 2, 25)),
        inner_map: Span(p!(2, 3, 12), p!(2, 14, 23)),
        key_somevec: Span(p!(1, 2, 1), p!(1, 10, 9)),
        key_value: Span(p!(2, 4, 13), p!(2, 10, 19)),
        int_42: Span(p!(2, 11, 20), p!(2, 13, 22))
      })
    );

    let e = "{:somevec
 [ {:value 42} ; lol
]
    }";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      expected_result(SpanMap {
        outer_map: Span(p!(1, 1, 0), p!(4, 6, 38)),
        inner_vec: Span(p!(2, 2, 11), p!(3, 2, 32)),
        inner_map: Span(p!(2, 4, 13), p!(2, 15, 24)),
        key_somevec: Span(p!(1, 2, 1), p!(1, 10, 9)),
        key_value: Span(p!(2, 5, 14), p!(2, 11, 20)),
        int_42: Span(p!(2, 12, 21), p!(2, 14, 23))
      })
    );

    let e = "{:somevec,[{:value,42}]}";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      expected_result(SpanMap {
        outer_map: Span(p!(1, 1, 0), p!(1, 25, 24)),
        inner_vec: Span(p!(1, 11, 10), p!(1, 24, 23)),
        inner_map: Span(p!(1, 12, 11), p!(1, 23, 22)),
        key_somevec: Span(p!(1, 2, 1), p!(1, 10, 9)),
        key_value: Span(p!(1, 13, 12), p!(1, 19, 18)),
        int_42: Span(p!(1, 20, 19), p!(1, 22, 21))
      })
    );
  }

  #[test]
  fn sets() {
    let e = "#{:cat 1 true #{:cat true} 2 [42]}";
    assert_eq!(
      parse::parse(&mut SourceReader::new(e)).unwrap(),
      Node::no_discards(
        NodeKind::Set(
          Vec::from([
            Node::no_discards(NodeKind::Key("cat"), Span(p!(3), p!(7))),
            Node::no_discards(NodeKind::Int(1), Span(p!(8), p!(9))),
            Node::no_discards(NodeKind::Bool(true), Span(p!(10), p!(14))),
            Node::no_discards(
              NodeKind::Set(
                Vec::from([
                  Node::no_discards(NodeKind::Key("cat"), Span(p!(17), p!(21))),
                  Node::no_discards(NodeKind::Bool(true), Span(p!(22), p!(26)))
                ]),
                vec![]
              ),
              Span(p!(15), p!(27))
            ),
            Node::no_discards(NodeKind::Int(2), Span(p!(28), p!(29))),
            Node::no_discards(
              NodeKind::Vector(
                vec![Node::no_discards(NodeKind::Int(42), Span(p!(31), p!(33)))],
                vec![]
              ),
              Span(p!(30), p!(34))
            ),
          ]),
          vec![]
        ),
        Span(p!(1), p!(35))
      )
    );
  }

  #[test]
  fn numbers() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("43/5143")).unwrap(),
      Node::no_discards(NodeKind::Rational((43, 5143)), Span(p!(1), p!(8)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-1190128294822145183/3023870813131455535")).unwrap(),
      Node::no_discards(
        NodeKind::Rational((-1190128294822145183, 3023870813131455535)),
        Span(p!(1), p!(41))
      )
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-2477641376863858799/-8976013293400652448")).unwrap(),
      Node::no_discards(
        NodeKind::Rational((-2477641376863858799, -8976013293400652448)),
        Span(p!(1), p!(42))
      )
    );
  }

  #[test]
  fn parse_0x_ints() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("0x2a")).unwrap(),
      Node::no_discards(NodeKind::Int(42), Span(p!(1), p!(5)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-0X2A")).unwrap(),
      Node::no_discards(NodeKind::Int(-42), Span(p!(1), p!(6)))
    );
    // leading plus character
    assert_eq!(
      parse::parse(&mut SourceReader::new("+42")).unwrap(),
      Node::no_discards(NodeKind::Int(42), Span(p!(1), p!(4)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("+0x2a")).unwrap(),
      Node::no_discards(NodeKind::Int(42), Span(p!(1), p!(6)))
    );
  }

  #[test]
  fn parse_radix_ints() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("16r2a")).unwrap(),
      Node::no_discards(NodeKind::Int(42), Span(p!(1), p!(6)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("8r63")).unwrap(),
      Node::no_discards(NodeKind::Int(51), Span(p!(1), p!(5)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("36rabcxyz")).unwrap(),
      Node::no_discards(NodeKind::Int(623_741_435), Span(p!(1), p!(10)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-16r2a")).unwrap(),
      Node::no_discards(NodeKind::Int(-42), Span(p!(1), p!(7)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-32rFOObar")).unwrap(),
      Node::no_discards(NodeKind::Int(-529_280_347), Span(p!(1), p!(11)))
    );
  }

  #[test]
  fn lisp_quoted() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("('(symbol))")).unwrap(),
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("'"), Span(p!(2), p!(3))),
            Node::no_discards(
              NodeKind::List(
                vec![Node::no_discards(NodeKind::Symbol("symbol"), Span(p!(4), p!(10)))],
                vec![]
              ),
              Span(p!(3), p!(11))
            )
          ],
          vec![]
        ),
        Span(p!(1), p!(12))
      ),
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new("(apply + '(1 2 3))")).unwrap(),
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("apply"), Span(p!(2), p!(7))),
            Node::no_discards(NodeKind::Symbol("+"), Span(p!(8), p!(9))),
            Node::no_discards(NodeKind::Symbol("'"), Span(p!(10), p!(11))),
            Node::no_discards(
              NodeKind::List(
                vec![
                  Node::no_discards(NodeKind::Int(1), Span(p!(12), p!(13))),
                  Node::no_discards(NodeKind::Int(2), Span(p!(14), p!(15))),
                  Node::no_discards(NodeKind::Int(3), Span(p!(16), p!(17)))
                ],
                vec![]
              ),
              Span(p!(11), p!(18))
            ),
          ],
          vec![]
        ),
        Span(p!(1), p!(19))
      )
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new("('(''symbol'foo''bar''))")).unwrap(),
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("'"), Span(p!(2), p!(3))),
            Node::no_discards(
              NodeKind::List(
                vec![Node::no_discards(
                  NodeKind::Symbol("''symbol'foo''bar''"),
                  Span(p!(4), p!(23))
                ),],
                vec![]
              ),
              Span(p!(3), p!(24))
            )
          ],
          vec![]
        ),
        Span(p!(1), p!(25))
      )
    );
  }

  #[test]
  fn numeric_like_symbols_keywords() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("-foobar")).unwrap(),
      Node::no_discards(NodeKind::Symbol("-foobar"), Span(p!(1), p!(8)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new("-:thi#n=g")).unwrap(),
      Node::no_discards(NodeKind::Symbol("-:thi#n=g"), Span(p!(1), p!(10)))
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new(":thi#n=g")).unwrap(),
      Node::no_discards(NodeKind::Key("thi#n=g"), Span(p!(1), p!(9)))
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new("(+foobar +foo+bar+ +'- '-+)")).unwrap(),
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("+foobar"), Span(p!(2), p!(9))),
            Node::no_discards(NodeKind::Symbol("+foo+bar+"), Span(p!(10), p!(19))),
            Node::no_discards(NodeKind::Symbol("+'-"), Span(p!(20), p!(23))),
            Node::no_discards(NodeKind::Symbol("'-+"), Span(p!(24), p!(27))),
          ],
          vec![]
        ),
        Span(p!(1), p!(28))
      )
    );

    assert!(parse::parse(&mut SourceReader::new("(-foo( ba")).is_err());
  }

  #[test]
  fn special_chars() {
    let mut reader = SourceReader::new("\\c[lolthisisvalidedn");
    assert_eq!(
      parse::parse(&mut reader).unwrap(),
      Node::no_discards(NodeKind::Char('c'), Span(p!(1), p!(3)))
    );
    assert!(parse::parse(&mut reader).is_err());

    let edn = "[\\space \\@ \\` \\tab \\return \\newline \\# \\% \\' \\g \\( \\* \\j \\+ \\, \\l \\- \\. \\/ \\0 \\2 \\r \\: \\; \\< \\\\ \\] \\} \\~ \\? \\_]";

    assert_eq!(
      parse::parse(&mut SourceReader::new(edn)).unwrap(),
      Node::no_discards(
        NodeKind::Vector(
          vec![
            Node::no_discards(NodeKind::Char(' '), Span(p!(2), p!(8))),
            Node::no_discards(NodeKind::Char('@'), Span(p!(9), p!(11))),
            Node::no_discards(NodeKind::Char('`'), Span(p!(12), p!(14))),
            Node::no_discards(NodeKind::Char('\t'), Span(p!(15), p!(19))),
            Node::no_discards(NodeKind::Char('\r'), Span(p!(20), p!(27))),
            Node::no_discards(NodeKind::Char('\n'), Span(p!(28), p!(36))),
            Node::no_discards(NodeKind::Char('#'), Span(p!(37), p!(39))),
            Node::no_discards(NodeKind::Char('%'), Span(p!(40), p!(42))),
            Node::no_discards(NodeKind::Char('\''), Span(p!(43), p!(45))),
            Node::no_discards(NodeKind::Char('g'), Span(p!(46), p!(48))),
            Node::no_discards(NodeKind::Char('('), Span(p!(49), p!(51))),
            Node::no_discards(NodeKind::Char('*'), Span(p!(52), p!(54))),
            Node::no_discards(NodeKind::Char('j'), Span(p!(55), p!(57))),
            Node::no_discards(NodeKind::Char('+'), Span(p!(58), p!(60))),
            Node::no_discards(NodeKind::Char(','), Span(p!(61), p!(63))),
            Node::no_discards(NodeKind::Char('l'), Span(p!(64), p!(66))),
            Node::no_discards(NodeKind::Char('-'), Span(p!(67), p!(69))),
            Node::no_discards(NodeKind::Char('.'), Span(p!(70), p!(72))),
            Node::no_discards(NodeKind::Char('/'), Span(p!(73), p!(75))),
            Node::no_discards(NodeKind::Char('0'), Span(p!(76), p!(78))),
            Node::no_discards(NodeKind::Char('2'), Span(p!(79), p!(81))),
            Node::no_discards(NodeKind::Char('r'), Span(p!(82), p!(84))),
            Node::no_discards(NodeKind::Char(':'), Span(p!(85), p!(87))),
            Node::no_discards(NodeKind::Char(';'), Span(p!(88), p!(90))),
            Node::no_discards(NodeKind::Char('<'), Span(p!(91), p!(93))),
            Node::no_discards(NodeKind::Char('\\'), Span(p!(94), p!(96))),
            Node::no_discards(NodeKind::Char(']'), Span(p!(97), p!(99))),
            Node::no_discards(NodeKind::Char('}'), Span(p!(100), p!(102))),
            Node::no_discards(NodeKind::Char('~'), Span(p!(103), p!(105))),
            Node::no_discards(NodeKind::Char('?'), Span(p!(106), p!(108))),
            Node::no_discards(NodeKind::Char('_'), Span(p!(109), p!(111))),
          ],
          vec![]
        ),
        Span(p!(1), p!(112))
      )
    );
  }

  #[test]
  fn read_forms() {
    let s = "(def foo 42)(sum '(1 2 3)) #_(foo the bar (cat)) 42 nil 2";
    let mut reader = parse::SourceReader::new(s);
    let n = parse::parse(&mut reader).unwrap();
    assert_eq!(
      n,
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("def"), Span(p!(2), p!(5))),
            Node::no_discards(NodeKind::Symbol("foo"), Span(p!(6), p!(9))),
            Node::no_discards(NodeKind::Int(42), Span(p!(10), p!(12)))
          ],
          vec![]
        ),
        Span(p!(1), p!(13))
      )
    );

    let n = parse::parse(&mut reader).unwrap();
    assert_eq!(
      n,
      Node::no_discards(
        NodeKind::List(
          vec![
            Node::no_discards(NodeKind::Symbol("sum"), Span(p!(14), p!(17))),
            Node::no_discards(NodeKind::Symbol("'"), Span(p!(18), p!(19))),
            Node::no_discards(
              NodeKind::List(
                vec![
                  Node::no_discards(NodeKind::Int(1), Span(p!(20), p!(21))),
                  Node::no_discards(NodeKind::Int(2), Span(p!(22), p!(23))),
                  Node::no_discards(NodeKind::Int(3), Span(p!(24), p!(25)))
                ],
                vec![]
              ),
              Span(p!(19), p!(26))
            )
          ],
          vec![]
        ),
        Span(p!(13), p!(27))
      )
    );

    let n = parse::parse(&mut reader).unwrap();
    assert_eq!(
      n,
      Node {
        kind: NodeKind::Int(42),
        span: Span(p!(50), p!(52)),
        leading_discards: vec![Discard(
          Node::no_discards(
            NodeKind::List(
              vec![
                Node::no_discards(NodeKind::Symbol("foo"), Span(p!(31), p!(34))),
                Node::no_discards(NodeKind::Symbol("the"), Span(p!(35), p!(38))),
                Node::no_discards(NodeKind::Symbol("bar"), Span(p!(39), p!(42))),
                Node::no_discards(
                  NodeKind::List(
                    vec![Node::no_discards(NodeKind::Symbol("cat"), Span(p!(44), p!(47)))],
                    vec![]
                  ),
                  Span(p!(43), p!(48))
                ),
              ],
              vec![]
            ),
            Span(p!(30), p!(49))
          ),
          Span(p!(28), p!(49))
        )]
      }
    );

    let n = parse::parse(&mut reader).unwrap();
    assert_eq!(n, Node::no_discards(NodeKind::Nil, Span(p!(53), p!(56))));

    let n = parse::parse(&mut reader).unwrap();
    assert_eq!(n, Node::no_discards(NodeKind::Int(2), Span(p!(57), p!(58))));

    // EOF
    assert!(
      parse::parse(&mut reader).is_ok_and(|n| matches!(n.kind, NodeKind::Nil) && n.span.is_empty())
    );
  }

  #[test]
  fn tagged() {
    assert_eq!(
      parse::parse(&mut SourceReader::new("#inst \"1985-04-12T23:20:50.52Z\"")).unwrap(),
      Node::no_discards(
        NodeKind::Tagged(
          "inst",
          Span(p!(2), p!(6)),
          Box::new(Node::no_discards(
            NodeKind::Str("1985-04-12T23:20:50.52Z"),
            Span(p!(7), p!(32))
          ))
        ),
        Span(p!(1), p!(32))
      ),
    );
    assert_eq!(
      parse::parse(&mut SourceReader::new(r"#Unit nil")).unwrap(),
      Node::no_discards(
        NodeKind::Tagged(
          "Unit",
          Span(p!(2), p!(6)),
          Box::new(Node::no_discards(NodeKind::Nil, Span(p!(7), p!(10))))
        ),
        Span(p!(1), p!(10))
      )
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new("#pow2 #pow3 2")).unwrap(),
      Node::no_discards(
        NodeKind::Tagged(
          "pow2",
          Span(p!(2), p!(6)),
          Box::new(Node::no_discards(
            NodeKind::Tagged(
              "pow3",
              Span(p!(8), p!(12)),
              Box::new(Node::no_discards(NodeKind::Int(2), Span(p!(13), p!(14))))
            ),
            Span(p!(7), p!(14))
          ))
        ),
        Span(p!(1), p!(14))
      )
    );

    assert_eq!(
      parse::parse(&mut SourceReader::new("#foo #bar #ニャンキャット {:baz #42 \"wut\"}")).unwrap(),
      Node::no_discards(
        NodeKind::Tagged(
          "foo",
          Span(p!(2), p!(5)),
          Box::new(Node::no_discards(
            NodeKind::Tagged(
              "bar",
              Span(p!(7), p!(10)),
              Box::new(Node::no_discards(
                NodeKind::Tagged(
                  "ニャンキャット",
                  Span(p!(12), p!(19, 32)),
                  Box::new(Node::no_discards(
                    NodeKind::Map(
                      Vec::from([(
                        Node::no_discards(NodeKind::Key("baz"), Span(p!(21, 34), p!(25, 38))),
                        Node::no_discards(
                          NodeKind::Tagged(
                            "42",
                            Span(p!(27, 40), p!(29, 42)),
                            Box::new(Node::no_discards(
                              NodeKind::Str("wut"),
                              Span(p!(30, 43), p!(35, 48))
                            ))
                          ),
                          Span(p!(26, 39), p!(35, 48))
                        )
                      )]),
                      vec![]
                    ),
                    Span(p!(20, 33), p!(36, 49))
                  ))
                ),
                Span(p!(11, 10), p!(36, 49))
              ))
            ),
            Span(p!(6, 5), p!(36, 49))
          ))
        ),
        Span(p!(1, 0), p!(36, 49))
      )
    );
  }
}
