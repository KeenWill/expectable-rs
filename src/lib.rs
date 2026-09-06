#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod value;

use serde::Serialize;
use std::fmt;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use value::{Serializer, Value};

/// Alignment of data cells. Headers are always left-aligned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Right-align columns whose nonempty cells all parse as finite numbers.
    #[default]
    Numbers,
    /// Left-align all data cells.
    Left,
    /// Right-align all data cells.
    Right,
}

/// How a nested field's path is displayed in its column header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NestedColumns {
    /// Choose dotted or stacked using Jane Street's field-length heuristic.
    #[default]
    Auto,
    /// Join field names with dots.
    Dotted,
    /// Stack field names on separate lines, bottom-aligned with other headers.
    Stacked,
    /// Show only the final field name; duplicate headers remain separate columns.
    Last,
}

/// Formatting options. See the crate documentation for representation rules.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Header layout; defaults to [`NestedColumns::Auto`].
    pub nested_columns: NestedColumns,
    /// Maximum struct depth to expand. `None` expands all structs; `Some(0)` expands none.
    pub max_depth: Option<usize>,
    /// Data alignment; defaults to [`Align::Numbers`].
    pub align: Align,
    /// Draw a horizontal separator between each pair of data rows.
    pub separate_rows: bool,
    /// Maximum display width, including borders. Cells wrap to fit.
    /// An impossible width produces `…\n` (or an empty string for zero).
    pub limit_width_to: Option<usize>,
}

/// Render rows with default options. Empty input produces an empty string.
pub fn print<T: Serialize>(rows: &[T]) -> String {
    Options::default().print(rows)
}

/// Write a table with default options, propagating errors from the writer.
pub fn print_to<W: fmt::Write + ?Sized, T: Serialize>(writer: &mut W, rows: &[T]) -> fmt::Result {
    Options::default().print_to(writer, rows)
}

impl Options {
    /// Render rows as a table ending in a newline, or an empty string for no rows.
    /// Serialization failures appear as explicit error cells; see the crate documentation.
    pub fn print<T: Serialize>(&self, rows: &[T]) -> String {
        let values = rows
            .iter()
            .map(|row| {
                row.serialize(Serializer)
                    .unwrap_or_else(|error| Value::Atom(format!("<serialization error: {error}>")))
            })
            .collect::<Vec<_>>();
        if values.is_empty() {
            return String::new();
        }
        let mut schema = Schema::default();
        for value in &values {
            schema.observe(value, self.max_depth);
        }
        let mut columns = Vec::new();
        schema.columns(&mut Vec::new(), &mut columns);
        let headers = columns
            .iter()
            .map(|(path, _)| self.header(path))
            .collect::<Vec<_>>();
        let cells = values
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|(path, has_children)| match at_path(row, path) {
                        Some(Value::Struct(fields)) if *has_children && !fields.is_empty() => {
                            String::new()
                        }
                        Some(value) => value.cell(),
                        None => String::new(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        render(&headers, &cells, self)
    }

    /// Write a table using these options, propagating errors from the writer.
    pub fn print_to<W: fmt::Write + ?Sized, T: Serialize>(
        &self,
        writer: &mut W,
        rows: &[T],
    ) -> fmt::Result {
        let table = self.print(rows);
        if table.is_empty() {
            Ok(())
        } else {
            writer.write_str(&table)
        }
    }

    fn header(&self, path: &[String]) -> String {
        if path.is_empty() {
            return "value".into();
        }
        let mode = if self.nested_columns == NestedColumns::Auto {
            if path.iter().map(String::len).sum::<usize>() / 6 < path.len() {
                NestedColumns::Dotted
            } else {
                NestedColumns::Stacked
            }
        } else {
            self.nested_columns
        };
        let header = match mode {
            NestedColumns::Last => path.last().cloned().unwrap_or_default(),
            NestedColumns::Stacked => path.join("\n"),
            _ => path.join("."),
        };
        value::clean(&header)
    }
}

#[derive(Default)]
struct Schema {
    leaf: bool,
    children: Vec<(String, Schema)>,
}
impl Schema {
    fn observe(&mut self, value: &Value, remaining: Option<usize>) {
        match value {
            Value::Struct(fields) if remaining != Some(0) && !fields.is_empty() => {
                for (key, value) in fields {
                    let index = match self.children.iter().position(|(name, _)| name == key) {
                        Some(index) => index,
                        None => {
                            self.children.push((key.clone(), Self::default()));
                            self.children.len() - 1
                        }
                    };
                    self.children[index]
                        .1
                        .observe(value, remaining.map(|n| n - 1));
                }
            }
            Value::Empty => {}
            _ => self.leaf = true,
        }
    }
    fn columns(&self, path: &mut Vec<String>, out: &mut Vec<(Vec<String>, bool)>) {
        if self.leaf || self.children.is_empty() {
            out.push((path.clone(), !self.children.is_empty()));
        }
        for (name, child) in &self.children {
            path.push(name.clone());
            child.columns(path, out);
            path.pop();
        }
    }
}

fn at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut value = value;
    for name in path {
        let Value::Struct(fields) = value else {
            return None;
        };
        value = &fields.iter().find(|(key, _)| key == name)?.1;
    }
    Some(value)
}

fn render(headers: &[String], rows: &[Vec<String>], options: &Options) -> String {
    let count = headers.len();
    let mut widths = vec![1; count];
    let mut minimums = vec![1; count];
    for row in std::iter::once(headers).chain(rows.iter().map(Vec::as_slice)) {
        for (i, cell) in row.iter().enumerate() {
            for line in cell.split('\n') {
                widths[i] = widths[i].max(line.width());
            }
            for c in cell.chars() {
                minimums[i] = minimums[i].max(c.width().unwrap_or(0));
            }
        }
    }
    if let Some(limit) = options.limit_width_to {
        let overhead = count * 3 + 1;
        if limit < overhead + minimums.iter().sum::<usize>() {
            return if limit == 0 {
                String::new()
            } else {
                "…\n".into()
            };
        }
        let mut total = overhead + widths.iter().sum::<usize>();
        while total > limit {
            let index = (0..count)
                .filter(|&i| widths[i] > minimums[i])
                .max_by_key(|&i| (widths[i], std::cmp::Reverse(i)));
            let Some(index) = index else {
                break;
            };
            widths[index] -= 1;
            total -= 1;
        }
    }
    let right = (0..count)
        .map(|i| match options.align {
            Align::Left => false,
            Align::Right => true,
            Align::Numbers => {
                let nonempty = rows
                    .iter()
                    .map(|row| row[i].as_str())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                !nonempty.is_empty()
                    && nonempty
                        .iter()
                        .all(|s| s.parse::<f64>().is_ok_and(f64::is_finite))
            }
        })
        .collect::<Vec<_>>();
    let mut out = String::new();
    border(&mut out, &widths, '┌', '┬', '┐');
    write_row(&mut out, headers, &widths, &vec![false; count], true);
    border(&mut out, &widths, '├', '┼', '┤');
    for (i, row) in rows.iter().enumerate() {
        if i > 0 && options.separate_rows {
            border(&mut out, &widths, '├', '┼', '┤');
        }
        write_row(&mut out, row, &widths, &right, false);
    }
    border(&mut out, &widths, '└', '┴', '┘');
    out
}

fn border(out: &mut String, widths: &[usize], left: char, middle: char, right: char) {
    out.push(left);
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            out.push(middle);
        }
        out.push_str(&"─".repeat(width + 2));
    }
    out.push(right);
    out.push('\n');
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let mut part = String::new();
        for c in line.chars() {
            let previous_len = part.len();
            part.push(c);
            if part.width() > width && previous_len > 0 {
                part.truncate(previous_len);
                lines.push(std::mem::take(&mut part));
                part.push(c);
            }
        }
        lines.push(part);
    }
    lines
}

fn write_row(out: &mut String, row: &[String], widths: &[usize], right: &[bool], bottom: bool) {
    let lines = row
        .iter()
        .zip(widths)
        .map(|(cell, &width)| wrap(cell, width))
        .collect::<Vec<_>>();
    let height = lines.iter().map(Vec::len).max().unwrap_or(1);
    for line in 0..height {
        out.push('│');
        for (i, cell) in lines.iter().enumerate() {
            let offset = if bottom { height - cell.len() } else { 0 };
            let text = line
                .checked_sub(offset)
                .and_then(|line| cell.get(line))
                .map_or("", String::as_str);
            let padding = " ".repeat(widths[i].saturating_sub(text.width()));
            out.push(' ');
            if right[i] {
                out.push_str(&padding);
            }
            out.push_str(text);
            if !right[i] {
                out.push_str(&padding);
            }
            out.push_str(" │");
        }
        out.push('\n');
    }
}
