//! Whether the pinned Ruff and ty accept the settings of a `pyproject.toml`. Ruff and ty stop
//! with an error on a key they do not know, and Ruff also on a rule selector it does not know,
//! so a project with such a setting cannot match the profile.
//!
//! [`problems`] reads the file against the catalog of the pinned releases and runs no tool.
//! [`ruff_rejects`] runs the pinned Ruff on a copy of the settings when it is installed, which
//! also catches a value of the wrong type.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use toml_edit::{Item, TableLike};

use super::catalog::{self, Catalog, Kind, Surface, Tool};
use super::plan::dotted;
use super::probe::Checker;
use super::ruff;
use crate::toml::{self, Toml};

/// The arrays of Ruff rule selectors in which a removed rule stops Ruff, because they select it.
const SELECTING: [&str; 6] = [
    "select",
    "extend-select",
    "fixable",
    "extend-fixable",
    "extend-safe-fixes",
    "extend-unsafe-fixes",
];
/// The arrays of Ruff rule selectors in which a removed rule only gets a warning.
const IGNORING: [&str; 4] = ["ignore", "extend-ignore", "unfixable", "extend-unfixable"];
/// The tables of file patterns that each hold an array of Ruff rule selectors.
const PER_FILE: [&str; 2] = ["per-file-ignores", "extend-per-file-ignores"];

/// One setting that the pinned Ruff or ty rejects: its line and what is wrong.
#[derive(Debug)]
pub(crate) struct Problem {
    pub(crate) line: Option<usize>,
    pub(crate) message: String,
}

/// The settings of `doc` that the pinned Ruff or ty rejects, by the catalog of their keys and
/// rules.
pub(crate) fn problems(doc: &Toml, catalog: &Catalog) -> Vec<Problem> {
    let mut found = Vec::new();
    if let Some(table) = toml::get(doc, &["tool", "ruff"]).and_then(Item::as_table_like) {
        let keys = keys(catalog, Tool::Ruff, "");
        walk(
            doc,
            table,
            &["tool", "ruff"],
            2,
            &keys,
            &mut |path| {
                format!(
                    "Ruff {} has no setting {}, so Ruff stops with an error. Remove it, or name a setting of Ruff {}.",
                    catalog::RUFF_RELEASE,
                    dotted(path),
                    catalog::RUFF_RELEASE
                )
            },
            &mut found,
        );
        selectors(doc, catalog, &mut found);
    }
    if let Some(table) = toml::get(doc, &["tool", "ty"]).and_then(Item::as_table_like) {
        let keys = keys(catalog, Tool::Ty, "tool.ty.");
        walk(
            doc,
            table,
            &["tool", "ty"],
            2,
            &keys,
            &mut |path| {
                format!(
                    "ty {} has no setting {}, so ty stops with an error. Remove it, or name a setting of ty {}.",
                    catalog::TY_RELEASE,
                    dotted(path),
                    catalog::TY_RELEASE
                )
            },
            &mut found,
        );
    }
    found
}

/// The keys of the pinned release of `tool` in its project table, each split at its dots, with
/// `prefix` taken off. `<key>` stands for any key.
fn keys(catalog: &Catalog, tool: Tool, prefix: &str) -> Vec<Vec<String>> {
    catalog
        .records()
        .iter()
        .filter(|record| {
            record.key.tool == tool
                && record.key.surface == Surface::Project
                && record.key.release == tool.pinned()
                && matches!(
                    record.key.kind,
                    Kind::Option | Kind::Schema | Kind::Legacy | Kind::Alias
                )
        })
        .filter_map(|record| record.key.id.strip_prefix(prefix))
        .map(|id| id.split('.').map(str::to_owned).collect())
        .collect()
}

/// Whether `key`, a known key split at its dots, names `path`.
fn names(key: &[String], path: &[&str]) -> bool {
    key.len() == path.len() && starts(key, path)
}

/// Whether `key`, a known key split at its dots, names a key below `path`.
fn below(key: &[String], path: &[&str]) -> bool {
    key.len() > path.len() && starts(key, path)
}

fn starts(key: &[String], path: &[&str]) -> bool {
    key.iter()
        .zip(path)
        .all(|(known, part)| known == "<key>" || known == part)
}

/// Records a problem for each key of `table`, at `path` in `doc`, that `keys` lacks, and walks
/// into each table and array of tables whose key has keys below it. `skip` parts of `path` are
/// the tool's table, which the keys leave out.
fn walk(
    doc: &Toml,
    table: &dyn TableLike,
    path: &[&str],
    skip: usize,
    keys: &[Vec<String>],
    message: &mut dyn FnMut(&[&str]) -> String,
    found: &mut Vec<Problem>,
) {
    for (key, item) in table.iter() {
        let mut full = path.to_vec();
        full.push(key);
        let relative = &full[skip..];
        if !keys.iter().any(|known| names(known, relative)) {
            found.push(Problem {
                line: toml::line(doc, &full),
                message: message(&full),
            });
            continue;
        }
        if !keys.iter().any(|known| below(known, relative)) {
            continue;
        }
        if let Some(child) = item.as_table_like() {
            walk(doc, child, &full, skip, keys, message, found);
        } else {
            for (child, _) in toml::tables(item) {
                walk(doc, child, &full, skip, keys, message, found);
            }
        }
    }
}

/// Records a problem for each rule selector that the pinned Ruff does not know, and for each
/// removed rule that a selecting array names.
fn selectors(doc: &Toml, catalog: &Catalog, found: &mut Vec<Problem>) {
    for table in [&["tool", "ruff", "lint"][..], &["tool", "ruff"]] {
        let arrays = SELECTING
            .iter()
            .map(|key| (*key, Removed::Rejected))
            .chain(IGNORING.iter().map(|key| (*key, Removed::Warned)));
        for (key, selecting) in arrays {
            let mut path = table.to_vec();
            path.push(key);
            if let Some(array) = toml::get(doc, &path).and_then(Item::as_array) {
                check_array(doc, catalog, &path, array, selecting, found);
            }
        }
        for key in PER_FILE {
            let mut path = table.to_vec();
            path.push(key);
            let Some(patterns) = toml::get(doc, &path).and_then(Item::as_table_like) else {
                continue;
            };
            for (pattern, item) in patterns.iter() {
                if let Some(array) = item.as_array() {
                    let mut full = path.clone();
                    full.push(pattern);
                    check_array(doc, catalog, &full, array, Removed::Warned, found);
                }
            }
        }
    }
}

/// What Ruff does with a removed rule in an array of selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Removed {
    /// It stops with an error, because the array selects the rule.
    Rejected,
    /// It warns, because the array turns the rule off.
    Warned,
}

fn check_array(
    doc: &Toml,
    catalog: &Catalog,
    path: &[&str],
    array: &toml_edit::Array,
    removed: Removed,
    found: &mut Vec<Problem>,
) {
    let release = catalog::RUFF_RELEASE;
    let shown = dotted(path);
    for value in array {
        let Some(selector) = value.as_str() else {
            continue;
        };
        let line = value.span().map(|span| toml::line_of(doc, span.start));
        let message = match catalog.redirect(selector) {
            None => Some(format!(
                "{shown} names {selector}, which Ruff {release} no longer accepts, so Ruff stops with an error. Name the rule that replaced it."
            )),
            Some("ALL") => None,
            Some(target) => {
                let rules: Vec<_> = catalog
                    .ruff_rules()
                    .filter(|rule| ruff::selects(target, rule))
                    .collect();
                if rules.is_empty() {
                    Some(format!(
                        "{shown} names {selector}, which is no rule selector of Ruff {release}, so Ruff stops with an error. Remove it, or name a code or prefix of Ruff {release}."
                    ))
                } else if removed == Removed::Rejected
                    && rules
                        .iter()
                        .all(|rule| rule.key.id == target && rule.is_removed())
                {
                    Some(format!(
                        "{shown} selects {selector}, which Ruff {release} removed, so Ruff stops with an error. Remove it."
                    ))
                } else {
                    None
                }
            }
        };
        if let Some(message) = message {
            found.push(Problem { line, message });
        }
    }
}

/// What the pinned Ruff says against the `pyproject.toml` text `text`, when the project's
/// environment or `PATH` has the pinned Ruff and it stops with an error on that file. `None`
/// when it accepts the file or the pinned Ruff is not installed.
pub(crate) fn ruff_rejects(root: &Path, text: &str) -> Option<String> {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let checker = Checker::find(Tool::Ruff, root);
    if !checker.is_pinned() {
        return None;
    }
    let folder = std::env::temp_dir().join(format!(
        "xpd-doctor-{}-{}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        fs::create_dir_all(&folder).ok()?;
        fs::write(folder.join("pyproject.toml"), text).ok()?;
        fs::write(folder.join("check.py"), "").ok()?;
        let path = checker.installed.as_ref()?.path.clone();
        let output = Command::new(path)
            .args(["check", "--no-cache", "--show-settings", "check.py"])
            .current_dir(&folder)
            .stdin(Stdio::null())
            .output()
            .ok()?;
        if output.status.success() {
            return None;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let shown = folder.display().to_string();
        let cause: Vec<String> = stderr
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != "ruff failed")
            .filter(|line| !line.starts_with("warning:"))
            .map(|line| line.strip_prefix("Cause:").map_or(line, str::trim))
            .map(|line| line.replace(&format!("{shown}/"), "").replace(&shown, "."))
            .collect();
        Some(cause.join(" "))
    })();
    let _ = fs::remove_dir_all(&folder);
    result
}
