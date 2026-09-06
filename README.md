# expectable

Text tables for Rust expect tests, inspired by [Jane Street's OCaml
expectable](https://github.com/janestreet/expectable). Rows only need
`serde::Serialize`; no `Debug` implementation or output parsing is involved.
Requires Rust 1.97.1 (edition 2024). Licensed under MIT.

```rust
use serde_derive::Serialize;

#[derive(Serialize)]
struct Animal<'a> {
    name: &'a str,
    age: Option<u32>,
}

let rows = [
    Animal { name: "otter", age: Some(3) },
    Animal { name: "owl", age: None },
];
let table = expectable::print(&rows);
assert_eq!(table, "\
┌───────┬─────┐
│ name  │ age │
├───────┼─────┤
│ otter │   3 │
│ owl   │     │
└───────┴─────┘
");
```

Add `expectable` and `serde` to your dependencies, and `serde_derive` to your
dev-dependencies for test-local derives. `expect-test` is optional:

```rust
use expect_test::expect;
use serde_derive::Serialize;

#[derive(Serialize)]
struct Row { item: &'static str, count: u32 }

let rows = [Row { item: "pear", count: 12 }];
expect![[r#"
    ┌──────┬───────┐
    │ item │ count │
    ├──────┼───────┤
    │ pear │    12 │
    └──────┴───────┘
"#]].assert_eq(&expectable::print(&rows));
```

## Options and writers

```rust
use expectable::{Align, NestedColumns, Options};
use serde_derive::Serialize;

#[derive(Serialize)]
struct Size { rows: u32, columns: u32 }
#[derive(Serialize)]
struct Row { label: Size }

let options = Options {
    nested_columns: NestedColumns::Dotted,
    max_depth: Some(2),
    align: Align::Left,
    separate_rows: true,
    limit_width_to: Some(80),
};
let rows = [Row { label: Size { rows: 10, columns: 20 } }];
let mut output = String::new();
options.print_to(&mut output, &rows).unwrap();
assert_eq!(output, options.print(&rows));
assert!(output.contains("label.rows"));
```

`print(&rows)` returns a `String`. `print_to(&mut writer, &rows)` accepts any
`std::fmt::Write` and returns its `fmt::Result`. Both have equivalent methods on
`Options`. Tables end in a newline. Empty input returns `""` and writes no text.

- `nested_columns`: `Auto` (default), `Dotted`, `Stacked`, or `Last`. Auto uses
  the upstream rule per column: dotted when the sum of field-name byte lengths
  divided by six is less than the path length; otherwise stacked. Stacked
  headers align at the bottom. Last may produce duplicate headers without
  merging the underlying columns.
- `max_depth`: `None` (default) expands all nested structs. `Some(0)` keeps each
  entire row inline in a `value` column; `Some(1)` expands only the root struct.
  This bounds column expansion, not serialization of the input.
- `align`: `Numbers` (default) right-aligns columns whose nonempty cell texts
  all parse as finite Rust `f64` values, including numeric strings. Other
  columns align left. `Left` and `Right` apply to every data column. Headers
  always align left. Numbers align at the right edge, not at decimal points.
- `separate_rows`: `false` (default); `true` draws borders between logical rows,
  including rows whose cells occupy several lines.
- `limit_width_to`: `None` (default) leaves widths unrestricted. A limit wraps
  headers and cells at character boundaries, shrinking the widest column first
  (leftmost wins ties). Widths use Unicode display columns, including borders
  and padding. A limit too narrow to fit all columns and their widest individual
  characters returns `"…\n"`; zero returns `""`. No content is truncated at
  feasible widths. Wrapping may split a multi-codepoint grapheme.

## Serde representation

A custom serializer preserves struct fields in serialization order (declaration
order with derived `Serialize`). Nested structs become columns, including ones
inside `Some` or newtype wrappers. Fields first encountered in later rows are
appended within their parent. Conditional omission therefore uses first-seen
order. Missing fields and `None` are empty cells. `Some` and newtype structs are
transparent at every depth; unit values are also empty. An optional nested
struct missing in one row gets empty cells in all of its inferred columns.
Serde cannot reveal the fields of a nested struct that is `None` in every row;
that field stays a single empty column.

Strings appear without quotes in ordinary cells. Newlines create multiple cell
lines; other control characters are escaped. Sequences, tuples, tuple structs,
and byte arrays are inline `[a, b]`. Maps are inline `{key: value}`, sorted by
rendered key and then rendered value, independent of iteration order. Structs
at the depth bound use the same braces in field order. Inline atoms containing
whitespace, control characters, quotes, backslashes, or collection delimiters
are quoted and escaped; empty strings are `""`. Inline absent or unit values
are `null`. This is a readable representation, not a lossless encoding: strings
and numbers can share a spelling.

Unit enum variants display their tag. Other externally tagged variants display
`Tag(payload)`, with tuple/struct payloads using the inline rules above. Other
Serde enum representations follow the shape their serializer emits. In
particular, Serde's `flatten` attribute emits a map, which stays inline.
Non-struct rows, including maps, use a `value` column. Empty structs show `{}`.
When a custom serializer changes a field between scalar and struct shapes,
its scalar column and nested columns are both retained.

Serialization failures produce an explicit `<serialization error: message>`
cell for the affected row, in the `value` column. Writer errors propagate from
`print_to`. As with Serde itself, custom `Serialize` implementations must finish
without panicking. For deterministic output they must serialize deterministic
values and struct field order; map iteration order does not matter.

The scoped OCaml adaptations are struct-based inference, inline collections,
transparent options at the depth bound, and right-edge numeric alignment.
OCaml's collection expansion, decimal-point padding, center alignment, custom
header callbacks, and case/alist/transposed helpers are not provided.
