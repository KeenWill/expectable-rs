use expect_test::expect;
use expectable::{Align, NestedColumns, Options, print, print_to};
use serde_derive::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use unicode_width::UnicodeWidthStr;

#[derive(Serialize)]
struct Row {
    name: &'static str,
    count: Option<i32>,
}

#[test]
fn declaration_order_numbers_and_none() {
    expect![[r#"
        ┌───────┬───────┐
        │ name  │ count │
        ├───────┼───────┤
        │ otter │     3 │
        │ owl   │       │
        │ fox   │   -20 │
        └───────┴───────┘
    "#]]
    .assert_eq(&print(&[
        Row {
            name: "otter",
            count: Some(3),
        },
        Row {
            name: "owl",
            count: None,
        },
        Row {
            name: "fox",
            count: Some(-20),
        },
    ]));
}

#[derive(Serialize)]
struct Size {
    rows: i32,
    columns: i32,
}
#[derive(Serialize)]
struct Nested {
    label: Size,
}
fn nested() -> [Nested; 1] {
    [Nested {
        label: Size {
            rows: 10,
            columns: 20,
        },
    }]
}

#[test]
fn auto_matches_upstream_example_layout() {
    expect![[r#"
        ┌────────────┬─────────┐
        │            │ label   │
        │ label.rows │ columns │
        ├────────────┼─────────┤
        │         10 │      20 │
        └────────────┴─────────┘
    "#]]
    .assert_eq(&print(&nested()));
}

#[test]
fn dotted_headers() {
    expect![[r#"
        ┌────────────┬───────────────┐
        │ label.rows │ label.columns │
        ├────────────┼───────────────┤
        │         10 │            20 │
        └────────────┴───────────────┘
    "#]]
    .assert_eq(
        &Options {
            nested_columns: NestedColumns::Dotted,
            ..Options::default()
        }
        .print(&nested()),
    );
}

#[test]
fn stacked_headers() {
    expect![[r#"
        ┌───────┬─────────┐
        │ label │ label   │
        │ rows  │ columns │
        ├───────┼─────────┤
        │    10 │      20 │
        └───────┴─────────┘
    "#]]
    .assert_eq(
        &Options {
            nested_columns: NestedColumns::Stacked,
            ..Options::default()
        }
        .print(&nested()),
    );
}

#[test]
fn last_headers_keep_distinct_columns() {
    #[derive(Serialize)]
    struct Pair {
        foo: Size,
        bar: Size,
    }
    expect![[r#"
        ┌──────┬─────────┬──────┬─────────┐
        │ rows │ columns │ rows │ columns │
        ├──────┼─────────┼──────┼─────────┤
        │   10 │      20 │    1 │       2 │
        └──────┴─────────┴──────┴─────────┘
    "#]]
    .assert_eq(
        &Options {
            nested_columns: NestedColumns::Last,
            ..Options::default()
        }
        .print(&[Pair {
            foo: Size {
                rows: 10,
                columns: 20,
            },
            bar: Size {
                rows: 1,
                columns: 2,
            },
        }]),
    );
}

#[test]
fn max_depth_one_keeps_nested_struct_inline() {
    expect![[r#"
        ┌─────────────────────────┐
        │ label                   │
        ├─────────────────────────┤
        │ {rows: 10, columns: 20} │
        └─────────────────────────┘
    "#]]
    .assert_eq(
        &Options {
            max_depth: Some(1),
            ..Options::default()
        }
        .print(&nested()),
    );
}

#[test]
fn max_depth_zero_keeps_whole_row_inline() {
    expect![[r#"
        ┌──────────────────────────────────┐
        │ value                            │
        ├──────────────────────────────────┤
        │ {label: {rows: 10, columns: 20}} │
        └──────────────────────────────────┘
    "#]]
    .assert_eq(
        &Options {
            max_depth: Some(0),
            ..Options::default()
        }
        .print(&nested()),
    );
}

#[test]
fn max_depth_two_expands_two_levels() {
    assert_eq!(
        Options {
            max_depth: Some(2),
            ..Options::default()
        }
        .print(&nested()),
        print(&nested())
    );
}

#[test]
fn missing_nested_struct_is_inferred_from_later_row() {
    #[derive(Serialize)]
    struct Optional {
        first: u32,
        nested: Option<Size>,
        last: bool,
    }
    expect![[r#"
        ┌───────┬─────────────┬─────────┬───────┐
        │       │             │ nested  │       │
        │ first │ nested.rows │ columns │ last  │
        ├───────┼─────────────┼─────────┼───────┤
        │     1 │             │         │ false │
        │     2 │          10 │      20 │ true  │
        └───────┴─────────────┴─────────┴───────┘
    "#]]
    .assert_eq(&print(&[
        Optional {
            first: 1,
            nested: None,
            last: false,
        },
        Optional {
            first: 2,
            nested: Some(Size {
                rows: 10,
                columns: 20,
            }),
            last: true,
        },
    ]));
}

#[test]
fn all_missing_nested_struct_stays_empty_column() {
    #[derive(Serialize)]
    struct Optional {
        nested: Option<Size>,
    }
    expect![[r#"
        ┌────────┐
        │ nested │
        ├────────┤
        │        │
        └────────┘
    "#]]
    .assert_eq(&print(&[Optional { nested: None }]));
}

#[test]
fn left_alignment() {
    expect![[r#"
        ┌───────┬───────┐
        │ name  │ count │
        ├───────┼───────┤
        │ otter │ 3     │
        │ owl   │ 120   │
        └───────┴───────┘
    "#]]
    .assert_eq(
        &Options {
            align: Align::Left,
            ..Options::default()
        }
        .print(&[
            Row {
                name: "otter",
                count: Some(3),
            },
            Row {
                name: "owl",
                count: Some(120),
            },
        ]),
    );
}

#[test]
fn right_alignment() {
    expect![[r#"
        ┌───────┬───────┐
        │ name  │ count │
        ├───────┼───────┤
        │ otter │     3 │
        │   owl │   120 │
        └───────┴───────┘
    "#]]
    .assert_eq(
        &Options {
            align: Align::Right,
            ..Options::default()
        }
        .print(&[
            Row {
                name: "otter",
                count: Some(3),
            },
            Row {
                name: "owl",
                count: Some(120),
            },
        ]),
    );
}

#[test]
fn numeric_strings_and_mixed_columns() {
    #[derive(Serialize)]
    struct Numbers {
        numeric: &'static str,
        mixed: &'static str,
    }
    expect![[r#"
        ┌─────────┬───────┐
        │ numeric │ mixed │
        ├─────────┼───────┤
        │     1.2 │ 1.2   │
        │    -300 │ word  │
        │         │       │
        └─────────┴───────┘
    "#]]
    .assert_eq(&print(&[
        Numbers {
            numeric: "1.2",
            mixed: "1.2",
        },
        Numbers {
            numeric: "-300",
            mixed: "word",
        },
        Numbers {
            numeric: "",
            mixed: "",
        },
    ]));
}

#[test]
fn multiline_cells_and_separate_rows() {
    expect![[r#"
        ┌───────┬───────┐
        │ name  │ count │
        ├───────┼───────┤
        │ otter │     3 │
        │ pup   │       │
        ├───────┼───────┤
        │ owl   │    12 │
        └───────┴───────┘
    "#]]
    .assert_eq(
        &Options {
            separate_rows: true,
            ..Options::default()
        }
        .print(&[
            Row {
                name: "otter\npup",
                count: Some(3),
            },
            Row {
                name: "owl",
                count: Some(12),
            },
        ]),
    );
}

#[test]
fn width_wraps_headers_and_values_without_dropping_text() {
    let output = Options {
        limit_width_to: Some(14),
        ..Options::default()
    }
    .print(&[Row {
        name: "abcdefgh",
        count: Some(12345),
    }]);
    expect![[r#"
        ┌─────┬──────┐
        │ nam │ coun │
        │ e   │ t    │
        ├─────┼──────┤
        │ abc │ 1234 │
        │ def │    5 │
        │ gh  │      │
        └─────┴──────┘
    "#]]
    .assert_eq(&output);
    assert!(output.lines().all(|line| line.width() <= 14));
}

#[test]
fn unicode_width_and_controls() {
    let output = print(&[Row {
        name: "界e\u{301}\t",
        count: Some(1),
    }]);
    expect![[r#"
        ┌───────┬───────┐
        │ name  │ count │
        ├───────┼───────┤
        │ 界é\t │     1 │
        └───────┴───────┘
    "#]]
    .assert_eq(&output);
    assert!(
        output
            .lines()
            .all(|line| line.width() == output.lines().next().unwrap().width())
    );
}

#[test]
fn width_extremes_and_wide_character_wrapping() {
    let rows = ["界界界", "a\u{301}b", "👩‍💻"];
    for limit in 0..=30 {
        let output = Options {
            limit_width_to: Some(limit),
            ..Options::default()
        }
        .print(&rows);
        assert!(
            output.lines().all(|line| line.width() <= limit),
            "limit {limit}: {output}"
        );
    }
    assert_eq!(
        Options {
            limit_width_to: Some(0),
            ..Options::default()
        }
        .print(&rows),
        ""
    );
    assert_eq!(
        Options {
            limit_width_to: Some(1),
            ..Options::default()
        }
        .print(&rows),
        "…\n"
    );
    assert_eq!(
        Options {
            limit_width_to: Some(usize::MAX),
            ..Options::default()
        }
        .print(&rows),
        print(&rows)
    );
    let wrapped = Options {
        limit_width_to: Some(6),
        ..Options::default()
    }
    .print(&["界界界"]);
    assert_eq!(wrapped.matches('界').count(), 3);
}

#[test]
fn sequences_maps_and_inline_escaping() {
    #[derive(Serialize)]
    struct Collections {
        sequence: Vec<Option<&'static str>>,
        map: BTreeMap<&'static str, i32>,
        tuple: (u8, bool),
    }
    expect![[r#"
        ┌──────────────────────────────────┬──────────────┬───────────┐
        │ sequence                         │ map          │ tuple     │
        ├──────────────────────────────────┼──────────────┼───────────┤
        │ [plain, "a b", "", "a\nb", null] │ {a: 1, z: 2} │ [3, true] │
        └──────────────────────────────────┴──────────────┴───────────┘
    "#]]
    .assert_eq(&print(&[Collections {
        sequence: vec![Some("plain"), Some("a b"), Some(""), Some("a\nb"), None],
        map: BTreeMap::from([("z", 2), ("a", 1)]),
        tuple: (3, true),
    }]));
}

#[test]
fn maps_sort_independently_of_iteration_and_accept_nonstring_keys() {
    let first = HashMap::from([((2, true), "z"), ((1, false), "a")]);
    let second = HashMap::from([((1, false), "a"), ((2, true), "z")]);
    assert_eq!(print(&[first]), print(&[second]));
    expect![[r#"
        ┌───────────────────┐
        │ value             │
        ├───────────────────┤
        │ {10: ten, 2: two} │
        └───────────────────┘
    "#]]
    .assert_eq(&print(&[BTreeMap::from([(2, "two"), (10, "ten")])]));
}

#[test]
fn enums_and_newtypes() {
    #[derive(Serialize)]
    enum Event {
        Unit,
        Newtype(u32),
        Tuple(u32, bool),
        Record { code: u32 },
    }
    #[derive(Serialize)]
    struct Wrapper(Event);
    expect![[r#"
        ┌───────────────────┐
        │ value             │
        ├───────────────────┤
        │ Unit              │
        │ Newtype(7)        │
        │ Tuple([8, true])  │
        │ Record({code: 9}) │
        └───────────────────┘
    "#]]
    .assert_eq(&print(&[
        Wrapper(Event::Unit),
        Wrapper(Event::Newtype(7)),
        Wrapper(Event::Tuple(8, true)),
        Wrapper(Event::Record { code: 9 }),
    ]));
}

#[test]
fn empty_inputs_and_empty_shapes() {
    #[derive(Serialize)]
    struct Empty {}
    assert_eq!(print::<Row>(&[]), "");
    expect![[r#"
        ┌───────┐
        │ value │
        ├───────┤
        │ {}    │
        └───────┘
    "#]]
    .assert_eq(&print(&[Empty {}]));
    expect![[r#"
        ┌───────┐
        │ value │
        ├───────┤
        │       │
        └───────┘
    "#]]
    .assert_eq(&print(&[()]));
    assert!(print(&[Vec::<u8>::new()]).contains("[]"));
}

#[test]
fn serde_rename_skip_and_flatten_follow_serialized_shape() {
    #[derive(Serialize)]
    struct Fields {
        #[serde(rename = "renamed")]
        z: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        optional: Option<u32>,
        a: u32,
    }
    expect![[r#"
        ┌─────────┬───┬──────────┐
        │ renamed │ a │ optional │
        ├─────────┼───┼──────────┤
        │       1 │ 2 │          │
        │       3 │ 5 │        4 │
        └─────────┴───┴──────────┘
    "#]]
    .assert_eq(&print(&[
        Fields {
            z: 1,
            optional: None,
            a: 2,
        },
        Fields {
            z: 3,
            optional: Some(4),
            a: 5,
        },
    ]));
    #[derive(Serialize)]
    struct Flat {
        #[serde(flatten)]
        fields: Size,
    }
    assert!(
        print(&[Flat {
            fields: Size {
                rows: 1,
                columns: 2
            }
        }])
        .contains("{columns: 2, rows: 1}")
    );
}

#[test]
fn options_and_newtypes_stay_transparent_at_depth_bound() {
    #[derive(Serialize)]
    struct Wrapper(Size);
    let rows = [Some(Wrapper(Size {
        rows: 1,
        columns: 2,
    }))];
    let expected = [Size {
        rows: 1,
        columns: 2,
    }];
    for depth in [None, Some(0), Some(1)] {
        let options = Options {
            max_depth: depth,
            ..Options::default()
        };
        assert_eq!(options.print(&rows), options.print(&expected));
    }
}

#[test]
fn writer_output_and_errors() {
    let rows = nested();
    let mut output = String::new();
    print_to(&mut output, &rows).unwrap();
    assert_eq!(output, print(&rows));
    let mut writer: &mut dyn fmt::Write = &mut output;
    let options = Options {
        separate_rows: true,
        ..Options::default()
    };
    options.print_to(&mut writer, &rows).unwrap();
    assert_eq!(output, format!("{}{}", print(&rows), options.print(&rows)));
    struct Failing;
    impl fmt::Write for Failing {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }
    assert!(print_to(&mut Failing, &rows).is_err());
    assert!(print_to::<_, Row>(&mut Failing, &[]).is_ok());
}

#[test]
fn serialization_errors_are_visible() {
    struct Failing;
    impl serde::Serialize for Failing {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("test failure"))
        }
    }
    expect![[r#"
        ┌─────────────────────────────────────┐
        │ value                               │
        ├─────────────────────────────────────┤
        │ <serialization error: test failure> │
        └─────────────────────────────────────┘
    "#]]
    .assert_eq(&print(&[Failing]));
}

#[test]
fn scalar_and_record_shapes_keep_all_values() {
    #[derive(Serialize)]
    #[serde(untagged)]
    enum Shape {
        Number(u32),
        Record { child: u32 },
    }
    #[derive(Serialize)]
    struct Variable {
        field: Shape,
    }
    expect![[r#"
        ┌───────┬─────────────┐
        │ field │ field.child │
        ├───────┼─────────────┤
        │     7 │             │
        │       │           9 │
        └───────┴─────────────┘
    "#]]
    .assert_eq(&print(&[
        Variable {
            field: Shape::Number(7),
        },
        Variable {
            field: Shape::Record { child: 9 },
        },
    ]));
}

#[test]
fn primitive_extremes_and_tuple_structs() {
    #[derive(Serialize)]
    struct Pair(i128, u128);
    let output = print(&[Pair(i128::MIN, u128::MAX)]);
    assert!(output.contains(&format!("[{}, {}]", i128::MIN, u128::MAX)));
    let output = print(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0]);
    assert!(output.contains("│ NaN   │"));
    assert!(output.contains("│ inf   │"));
    assert!(output.contains("│ -inf  │"));
    assert!(output.contains("│ -0    │"));
}

#[test]
fn byte_serialization_and_nested_inline_records() {
    struct Bytes;
    impl serde::Serialize for Bytes {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(&[0, 128, 255])
        }
    }
    assert!(print(&[Bytes]).contains("[0, 128, 255]"));
    assert!(
        print(&[vec![Size {
            rows: 1,
            columns: 2
        }]])
        .contains("[{rows: 1, columns: 2}]")
    );
}

#[test]
fn malformed_custom_maps_report_errors() {
    use serde::ser::SerializeMap;
    struct Malformed(u8);
    impl serde::Serialize for Malformed {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(None)?;
            match self.0 {
                0 => map.serialize_value(&1)?,
                1 => {
                    map.serialize_key(&1)?;
                    map.serialize_key(&2)?;
                }
                _ => map.serialize_key(&1)?,
            }
            map.end()
        }
    }
    let output = print(&[Malformed(0), Malformed(1), Malformed(2)]);
    assert_eq!(output.matches("serialization error").count(), 3);
    assert!(output.contains("map value without a key"));
    assert!(output.contains("map key without a value"));
}
