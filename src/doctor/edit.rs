//! Edits to a `pyproject.toml` that keep its comments and layout: a new item takes the indent of
//! the items beside it, and a new table gets its own header at the end of the file.

use toml_edit::{Array, DocumentMut, Item, Table, TableLike, Value};

/// The table at `path`, made with a header when it is missing and with no header for the tables
/// above it. `None` when a key on the way holds something other than a table.
pub(crate) fn table<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Option<&'a mut dyn TableLike> {
    let mut table: &mut dyn TableLike = doc.as_table_mut();
    for (depth, part) in path.iter().enumerate() {
        if !table.contains_key(part) {
            let mut new = Table::new();
            new.set_implicit(depth + 1 < path.len());
            table.insert(part, Item::Table(new));
        }
        let item = table.get_mut(part)?;
        if let Item::Table(child) = item
            && depth + 1 == path.len()
        {
            child.set_implicit(false);
        }
        table = item.as_table_like_mut()?;
    }
    Some(table)
}

/// The table at `path`, when it exists.
pub(crate) fn existing<'a>(
    doc: &'a mut DocumentMut,
    path: &[&str],
) -> Option<&'a mut dyn TableLike> {
    let mut table: &mut dyn TableLike = doc.as_table_mut();
    for part in path {
        table = table.get_mut(part)?.as_table_like_mut()?;
    }
    Some(table)
}

/// The array at `key` of the table at `path`, made empty when it is missing.
pub(crate) fn array<'a>(
    doc: &'a mut DocumentMut,
    path: &[&str],
    key: &str,
) -> Option<&'a mut Array> {
    let table = table(doc, path)?;
    if !table.contains_key(key) {
        table.insert(key, Item::Value(Value::Array(Array::new())));
    }
    table.get_mut(key)?.as_array_mut()
}

/// Sets `key` of the table at `path` to `value`, keeping the spacing and comment around an old
/// value. `false` when the table cannot hold it.
pub(crate) fn set(doc: &mut DocumentMut, path: &[&str], key: &str, value: Value) -> bool {
    let Some(table) = table(doc, path) else {
        return false;
    };
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = value;
            *old.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(value));
        }
    }
    true
}

/// Removes `key` from the table at `path`. `false` when it is not there.
pub(crate) fn remove(doc: &mut DocumentMut, path: &[&str], key: &str) -> bool {
    existing(doc, path).is_some_and(|table| table.remove(key).is_some())
}

/// Appends the string `text`: on its own line, indented like the item before it, in an array
/// with one item per line, and after a space in an array on one line.
pub(crate) fn push(array: &mut Array, text: &str) {
    let Some(last) = array.iter().last() else {
        array.push(text);
        return;
    };
    let prefix = last
        .decor()
        .prefix()
        .and_then(|prefix| prefix.as_str())
        .filter(|prefix| prefix.contains('\n'))
        .unwrap_or(" ")
        .to_owned();
    let mut value = Value::from(text);
    value.decor_mut().set_prefix(prefix);
    value.decor_mut().set_suffix("");
    array.push_formatted(value);
}

/// Removes the items at `indexes`, giving the new first item the spacing of the old one.
pub(crate) fn remove_items(array: &mut Array, indexes: &[usize]) {
    let first = array
        .get(0)
        .and_then(|value| value.decor().prefix())
        .and_then(|prefix| prefix.as_str())
        .map(str::to_owned);
    let mut sorted = indexes.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    for index in sorted.into_iter().rev() {
        if index < array.len() {
            array.remove(index);
        }
    }
    if let (Some(first), Some(value)) = (first, array.get_mut(0)) {
        value.decor_mut().set_prefix(first);
    }
    if array.is_empty() {
        array.set_trailing("");
        array.set_trailing_comma(false);
    }
}

/// The strings of `array`, each with its index. Items that are no string are left out.
pub(crate) fn strings(array: &Array) -> Vec<(usize, String)> {
    array
        .iter()
        .enumerate()
        .filter_map(|(index, value)| Some((index, value.as_str()?.to_owned())))
        .collect()
}
