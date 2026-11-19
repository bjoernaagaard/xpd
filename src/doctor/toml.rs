//! Reading the TOML files that hold Ruff and ty settings, and saying where in such a file a new
//! setting goes, so that an edit never repeats a table that the file already has.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Document, Item, Table, Value};

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

/// A setting's value as doctor compares it.
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
        text.trim_matches('`')
            .parse::<Value>()
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

/// A file that a suggestion is written to, which may not exist yet, and the table inside it
/// that holds the tool's settings: `tool.ruff` in a `pyproject.toml`, the root in a `ruff.toml`.
#[derive(Debug)]
pub(crate) struct Destination {
    pub(crate) file: PathBuf,
    pub(crate) prefix: &'static [&'static str],
    /// `None` when the file does not exist yet.
    pub(crate) doc: Option<Toml>,
}

/// Where an edit goes: under which table header, as which key, and what is already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Placement {
    /// The dotted name of the table whose header the key goes under, empty for the file's top.
    pub(crate) table: String,
    /// The line of that header, `None` when the table is new.
    pub(crate) table_line: Option<usize>,
    /// The key, dotted relative to the table.
    pub(crate) key: String,
    /// The line of the key when the file already sets it.
    pub(crate) key_line: Option<usize>,
    /// The value already there.
    pub(crate) current: Option<Observed>,
    /// Whether the value already there is an array that an item can be added to.
    pub(crate) array: bool,
    /// The file has tables, so a key for the top must go before the first of them.
    pub(crate) top_before_tables: bool,
    /// Whether the file exists yet.
    pub(crate) file_exists: bool,
}

impl Destination {
    /// Where the setting at `key`, dotted relative to the tool's table, goes.
    pub(crate) fn place(&self, key: &str) -> Placement {
        let path: Vec<&str> = self.prefix.iter().copied().chain(key.split('.')).collect();
        let Some(doc) = &self.doc else {
            return new_table(&path);
        };
        let mut table: &Table = doc.as_table();
        let mut header: usize = 0;
        let mut header_line = None;
        for (depth, part) in path[..path.len() - 1].iter().enumerate() {
            match table.get(part) {
                Some(Item::Table(child)) => {
                    if !child.is_implicit() && !child.is_dotted() {
                        header = depth + 1;
                        header_line = child.span().map(|span| line_of(doc, span.start));
                    }
                    table = child;
                }
                _ => break,
            }
        }
        if header == 0 && !self.prefix.is_empty() {
            return Placement {
                file_exists: true,
                ..new_table(&path)
            };
        }
        let item = get(doc, &path);
        let top_before_tables = header == 0
            && doc
                .as_table()
                .iter()
                .any(|(_, item)| matches!(item, Item::Table(t) if !t.is_implicit()));
        Placement {
            table: path[..header].join("."),
            table_line: if header == 0 { None } else { header_line },
            key: path[header..].join("."),
            key_line: item.and_then(|_| line(doc, &path)),
            current: item.and_then(Observed::of_item),
            array: item.is_some_and(|item| item.as_array().is_some()),
            top_before_tables,
            file_exists: true,
        }
    }
}

/// A placement in a new table named by every part of `path` but the last, in a new file.
fn new_table(path: &[&str]) -> Placement {
    let (key, table) = path.split_last().map_or(("", &[][..]), |(k, t)| (*k, t));
    Placement {
        table: table.join("."),
        table_line: None,
        key: key.to_owned(),
        key_line: None,
        current: None,
        array: false,
        top_before_tables: false,
        file_exists: false,
    }
}
