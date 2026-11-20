use std::path::PathBuf;

use toml_edit::{Item, Table};

use crate::toml::{Observed, Toml, get, line, line_of};

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
    /// The TOML spelling of a setting's value, without comments or surrounding whitespace.
    pub(crate) fn literal(&self, key: &str) -> Option<String> {
        let path: Vec<&str> = self.prefix.iter().copied().chain(key.split('.')).collect();
        let doc = self.doc.as_ref()?;
        let span = get(doc, &path)?.span()?;
        doc.raw().get(span).map(str::to_owned)
    }

    /// Where the setting at `key`, dotted relative to the tool's table, goes.
    pub(crate) fn place(&self, key: &str) -> Placement {
        self.place_path(&key.split('.').collect::<Vec<_>>())
    }

    /// Like `place`, but each segment can itself contain a dot.
    pub(crate) fn place_path(&self, key: &[&str]) -> Placement {
        let path: Vec<&str> = self
            .prefix
            .iter()
            .copied()
            .chain(key.iter().copied())
            .collect();
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
