use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use toml_edit::{Document, Item, Value};

/// A parsed TOML file.
pub(crate) type Toml = Document<String>;

/// Reads and parses `path`. `Ok(None)` when there is no such file.
pub(crate) fn read(path: &Path) -> Result<Option<Toml>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    Document::parse(text)
        .map(Some)
        .map_err(|error| format!("{}: {}", path.display(), error.to_string().trim()))
}

/// The item at the dotted `path`, through tables, inline tables and dotted keys alike.
pub(crate) fn get<'a>(doc: &'a Toml, path: &[&str]) -> Option<&'a Item> {
    let (last, parents) = path.split_last()?;
    let mut table: &dyn toml_edit::TableLike = doc.as_table();
    for part in parents {
        table = table.get(part)?.as_table_like()?;
    }
    table.get(last)
}

/// Each table of `item`, an array of tables in either spelling: `[[key]]` headers or an inline
/// array of inline tables. Each comes with the byte offset where it starts, when it has one.
pub(crate) fn tables(item: &Item) -> Vec<(&dyn toml_edit::TableLike, Option<usize>)> {
    match item {
        Item::ArrayOfTables(tables) => tables
            .iter()
            .map(|table| {
                let start = table.span().map(|span| span.start);
                let table: &dyn toml_edit::TableLike = table;
                (table, start)
            })
            .collect(),
        Item::Value(Value::Array(array)) => array
            .iter()
            .filter_map(|value| {
                let table = value.as_inline_table()?;
                let start = table.span().map(|span| span.start);
                let table: &dyn toml_edit::TableLike = table;
                Some((table, start))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The line, counted from 1, of the byte `offset` of `doc`.
pub(crate) fn line_of(doc: &Toml, offset: usize) -> usize {
    doc.raw()
        .get(..offset)
        .map_or(1, |before| before.matches('\n').count() + 1)
}

/// The line of the item at `path`, by its key.
pub(crate) fn line(doc: &Toml, path: &[&str]) -> Option<usize> {
    let (last, parents) = path.split_last()?;
    let mut table: &dyn toml_edit::TableLike = doc.as_table();
    for part in parents {
        table = table.get(part)?.as_table_like()?;
    }
    let (key, item) = table.get_key_value(last)?;
    key.span()
        .or_else(|| item.span())
        .map(|span| line_of(doc, span.start))
}

/// The shape and contents of a TOML setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Observed {
    Bool(bool),
    Text(String),
    /// The items of a list, or the keys of a table.
    Items(BTreeSet<String>),
}

impl Observed {
    pub(crate) fn of_item(item: &Item) -> Option<Self> {
        match item {
            Item::Value(value) => Some(Self::of_value(value)),
            Item::Table(table) => Some(Self::Items(
                table.iter().map(|(k, _)| k.to_owned()).collect(),
            )),
            Item::None | Item::ArrayOfTables(_) => None,
        }
    }

    pub(crate) fn of_value(value: &Value) -> Self {
        match value {
            Value::Boolean(b) => Self::Bool(*b.value()),
            Value::String(s) => Self::Text(s.value().clone()),
            Value::Array(items) => Self::Items(items.iter().map(scalar_text).collect()),
            Value::InlineTable(table) => {
                Self::Items(table.iter().map(|(key, _)| key.to_owned()).collect())
            }
            other => Self::Text(scalar_text(other)),
        }
    }

    /// The value of the TOML literal `text`, such as `false`, `"all"` or `{ "a" = "b" }`.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        text.parse::<Value>()
            .ok()
            .map(|value| Self::of_value(&value))
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Bool(b) => !b,
            Self::Text(text) => text.is_empty(),
            Self::Items(items) => items.is_empty(),
        }
    }

    pub(crate) fn show(&self) -> String {
        match self {
            Self::Bool(b) => b.to_string(),
            Self::Text(text) => format!("{text:?}"),
            Self::Items(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| format!("{item:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.value().clone(),
        Value::Integer(i) => i.value().to_string(),
        Value::Float(f) => f.value().to_string(),
        Value::Boolean(b) => b.value().to_string(),
        Value::Datetime(d) => d.value().to_string(),
        Value::Array(_) | Value::InlineTable(_) => String::new(),
    }
}
