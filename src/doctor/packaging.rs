//! The packaging part of the profile: the build backend, the Python floor and the bounds of the
//! requirements, the metadata each kind of project declares, uv's cooldown and defaults, and the
//! old packaging and hook files that a project moves to uv.
//!
//! The checks read `pyproject.toml` and the names of the files at the project root, and return
//! what differs. The plan turns each finding into a conflict, and writes the cooldown and the
//! Python floor itself.

use std::path::Path;

use anyhow::{Context, Result};
use toml_edit::{Item, TableLike, Value};

use super::pytest;
use crate::toml::{self, Toml};

/// The packaging values of `profile.toml`, from `[packaging]`.
#[derive(Debug, Clone)]
pub(crate) struct Rules {
    /// The value of `exclude-newer` in `[tool.uv]`, a rolling delay before uv takes an upload.
    pub(crate) cooldown: String,
    /// The Python release that `requires-python` starts at when a project names none.
    pub(crate) floor: String,
    /// Each build backend, with the file at the project root that calls for it. An empty file
    /// names the backend of a pure Python package.
    pub(crate) backends: Vec<(String, String)>,
    /// The settings of `[tool.uv]` that keep uv's own behavior, with the value each must have.
    pub(crate) uv: Vec<(String, Value)>,
    /// Old files at the project root, as names or patterns with `*`, with the way to move each.
    pub(crate) migrate_files: Vec<(String, String)>,
    /// Old tables of `pyproject.toml`, by their dotted path, with the way to move each.
    pub(crate) migrate_tables: Vec<(String, String)>,
}

impl Rules {
    /// Reads `[packaging]` of `profile.toml`.
    ///
    /// # Errors
    ///
    /// Returns an error when a key is missing or holds a value of the wrong type.
    pub(crate) fn read(table: &dyn TableLike) -> Result<Self> {
        let text = |key: &str| {
            table
                .get(key)
                .and_then(Item::as_str)
                .map(str::to_owned)
                .with_context(|| format!("packaging.{key} is no string"))
        };
        let pairs = |key: &str| -> Result<Vec<(String, String)>> {
            let mut found = table;
            for part in key.split('.') {
                found = found
                    .get(part)
                    .and_then(Item::as_table_like)
                    .with_context(|| format!("packaging.{key} is no table"))?;
            }
            found
                .iter()
                .map(|(name, item)| {
                    let value = item
                        .as_str()
                        .with_context(|| format!("packaging.{key}.{name} is no string"))?;
                    Ok((name.to_owned(), value.to_owned()))
                })
                .collect()
        };
        let uv = table
            .get("uv")
            .and_then(Item::as_table_like)
            .context("packaging.uv is no table")?
            .iter()
            .map(|(key, item)| {
                let mut value = item
                    .as_value()
                    .cloned()
                    .with_context(|| format!("packaging.uv.{key} is no value"))?;
                value.decor_mut().clear();
                Ok((key.to_owned(), value))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            cooldown: text("cooldown")?,
            floor: text("python-floor")?,
            backends: pairs("backends")?,
            uv,
            migrate_files: pairs("migrate.files")?,
            migrate_tables: pairs("migrate.tables")?,
        })
    }

    /// The backend that the project at `root` needs: the first whose file is at the root, or
    /// else the one for a pure Python package.
    pub(crate) fn backend(&self, root: &Path) -> Option<&str> {
        self.backends
            .iter()
            .find(|(_, file)| !file.is_empty() && root.join(file).is_file())
            .or_else(|| self.backends.iter().find(|(_, file)| file.is_empty()))
            .map(|(backend, _)| backend.as_str())
    }
}

/// One way the project differs from the packaging part of the profile, which no edit of
/// `pyproject.toml` resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// The file it is in, relative to the project root.
    pub(crate) file: String,
    pub(crate) message: String,
}

/// What the kind of a project asks of its packaging.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KindNeeds<'a> {
    /// The keys that `[project]` declares, or lists in `dynamic`.
    pub(crate) metadata: &'a [String],
    /// Whether the kind has a `[project]` table at all.
    pub(crate) project: bool,
}

/// The findings for the project at `root` with the `pyproject.toml` `doc`. `pinned` names the
/// tools the profile pins itself, whose exact requirements the bounds rule leaves alone.
pub(crate) fn findings(
    root: &Path,
    doc: Option<&Toml>,
    rules: &Rules,
    needs: KindNeeds<'_>,
    pinned: &[String],
) -> Vec<Finding> {
    let mut found = Vec::new();
    migration(root, doc, rules, &mut found);
    uv_defaults(doc, rules, &mut found);
    let Some(doc) = doc else {
        return found;
    };
    backend(root, doc, rules, &mut found);
    if needs.project && toml::get(doc, &["project"]).is_some() {
        python_cap(doc, &mut found);
        bounds(doc, pinned, &mut found);
        metadata(doc, needs.metadata, &mut found);
    }
    found
}

fn finding(file: &str, message: String) -> Finding {
    Finding {
        file: file.to_owned(),
        message,
    }
}

const PYPROJECT: &str = "pyproject.toml";

/// An old packaging or hook file at the root, or an old table of `pyproject.toml`.
fn migration(root: &Path, doc: Option<&Toml>, rules: &Rules, found: &mut Vec<Finding>) {
    let names: Vec<String> = std::fs::read_dir(root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let mut sorted = names;
    sorted.sort();
    for (pattern, how) in &rules.migrate_files {
        for name in sorted.iter().filter(|name| matches(pattern, name)) {
            found.push(finding(
                name,
                format!("The profile packages the project with pyproject.toml and uv, so {name} goes. {how}"),
            ));
        }
    }
    for (table, how) in &rules.migrate_tables {
        let keys: Vec<&str> = table.split('.').collect();
        if doc.is_some_and(|doc| toml::get(doc, &keys).is_some()) {
            found.push(finding(
                PYPROJECT,
                format!("The profile packages the project with [project] and uv, so [{table}] goes. {how}"),
            ));
        }
    }
}

/// Whether `name` matches `pattern`, whose one `*` stands for any text.
fn matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        Some((start, end)) => {
            name.len() >= start.len() + end.len() && name.starts_with(start) && name.ends_with(end)
        }
        None => pattern == name,
    }
}

/// A setting of `[tool.uv]` that changes uv's index strategy, build isolation or default groups.
fn uv_defaults(doc: Option<&Toml>, rules: &Rules, found: &mut Vec<Finding>) {
    let Some(table) = doc
        .and_then(|doc| toml::get(doc, &["tool", "uv"]))
        .and_then(Item::as_table_like)
    else {
        return;
    };
    for (key, wanted) in &rules.uv {
        let Some(value) = table.get(key).and_then(Item::as_value) else {
            continue;
        };
        if !same(value, wanted) {
            found.push(finding(
                PYPROJECT,
                format!("[tool.uv] sets {key}, and the profile keeps uv's own {key}, {wanted}. Remove {key}."),
            ));
        }
    }
}

/// Whether two values are the same, without their layout.
fn same(left: &Value, right: &Value) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.decor_mut().clear();
    right.decor_mut().clear();
    if let (Some(left), Some(right)) = (left.as_array_mut(), right.as_array_mut()) {
        left.fmt();
        right.fmt();
    }
    left.to_string() == right.to_string()
}

/// A `[build-system]` whose backend is not the one the project needs.
fn backend(root: &Path, doc: &Toml, rules: &Rules, found: &mut Vec<Finding>) {
    let Some(system) = toml::get(doc, &["build-system"]) else {
        return;
    };
    let Some(wanted) = rules.backend(root) else {
        return;
    };
    let named = system
        .as_table_like()
        .and_then(|table| table.get("build-backend"))
        .and_then(Item::as_str);
    let why = rules
        .backends
        .iter()
        .find(|(backend, _)| backend == wanted)
        .map_or_else(String::new, |(_, file)| {
            if file.is_empty() {
                "a pure Python package".to_owned()
            } else {
                format!("a project with {file}")
            }
        });
    match named {
        Some(named) if named == wanted => {}
        Some(named) => found.push(finding(
            PYPROJECT,
            format!("[build-system] builds with {named}, and the profile builds {why} with {wanted}. Move the build to {wanted}, and compare the files in the old and new wheel before you release it."),
        )),
        None => found.push(finding(
            PYPROJECT,
            format!("[build-system] names no build-backend, and the profile builds {why} with {wanted}. Set build-backend = \"{wanted}\" with its requirement in requires."),
        )),
    }
}

/// A `requires-python` that caps the Python release.
fn python_cap(doc: &Toml, found: &mut Vec<Finding>) {
    let Some(text) = toml::get(doc, &["project", "requires-python"]).and_then(Item::as_str) else {
        return;
    };
    let caps: Vec<&str> = specifiers(text)
        .filter(|clause| ["<", "~=", "=="].iter().any(|op| clause.starts_with(op)))
        .collect();
    if !caps.is_empty() {
        found.push(finding(
            PYPROJECT,
            format!("requires-python {text:?} caps Python with {}, and the profile names only the oldest release the project supports. Remove the cap.", caps.join(", ")),
        ));
    }
}

/// The clauses of a version specifier such as `>=3.13,<4`.
fn specifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(',')
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
}

/// A runtime requirement without a lowest release, or with one exact release.
fn bounds(doc: &Toml, pinned: &[String], found: &mut Vec<Finding>) {
    let mut lists: Vec<(String, &toml_edit::Array)> = Vec::new();
    if let Some(array) = toml::get(doc, &["project", "dependencies"]).and_then(Item::as_array) {
        lists.push(("project.dependencies".to_owned(), array));
    }
    if let Some(extras) =
        toml::get(doc, &["project", "optional-dependencies"]).and_then(Item::as_table_like)
    {
        for (extra, item) in extras.iter() {
            if let Some(array) = item.as_array() {
                lists.push((format!("project.optional-dependencies.{extra}"), array));
            }
        }
    }
    for (list, array) in lists {
        for value in array {
            let Some(text) = value.as_str() else { continue };
            if text.split(';').next().unwrap_or_default().contains('@') {
                continue;
            }
            let name = pytest::dependency(value).unwrap_or_default();
            if pinned.iter().any(|pin| pytest::normalize(pin) == name) {
                continue;
            }
            let clauses: Vec<&str> = pytest::clauses(text).collect();
            let lowest = clauses
                .iter()
                .any(|clause| [">", "==", "~="].iter().any(|op| clause.starts_with(op)));
            let exact = clauses.iter().any(|clause| {
                clause.starts_with("==") && !clause.ends_with(".*") || clause.starts_with("===")
            });
            if !lowest {
                found.push(finding(
                    PYPROJECT,
                    format!("{list} requires {text:?} without a lowest release. Name the oldest release that works, as in {name}>=<release>."),
                ));
            } else if exact {
                found.push(finding(
                    PYPROJECT,
                    format!("{list} requires {text:?}, one exact release, and the profile keeps exact choices in uv.lock. Use >= at the oldest release that works, and a cap only for a known incompatibility."),
                ));
            }
        }
    }
}

/// A key of `[project]` that the kind needs and the project neither declares nor lists in
/// `dynamic`, a license that is no SPDX expression, and `urls` without the repository.
fn metadata(doc: &Toml, needed: &[String], found: &mut Vec<Finding>) {
    let Some(project) = toml::get(doc, &["project"]).and_then(Item::as_table_like) else {
        return;
    };
    let dynamic: Vec<&str> = project
        .get("dynamic")
        .and_then(Item::as_array)
        .map(|array| array.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for key in needed {
        let declared = project.contains_key(key) || dynamic.contains(&key.as_str());
        if !declared {
            found.push(finding(
                PYPROJECT,
                format!(
                    "[project] declares no {key}, which the profile needs for this kind. Add it."
                ),
            ));
            continue;
        }
        match key.as_str() {
            "urls" if !dynamic.contains(&"urls") => {
                let repository = project
                    .get("urls")
                    .and_then(Item::as_table_like)
                    .is_some_and(|urls| {
                        urls.iter().any(|(label, _)| {
                            let label: String = label
                                .chars()
                                .filter(char::is_ascii_alphanumeric)
                                .collect::<String>()
                                .to_ascii_lowercase();
                            matches!(label.as_str(), "repository" | "source" | "sourcecode")
                        })
                    });
                if !repository {
                    found.push(finding(
                        PYPROJECT,
                        "[project.urls] names no Repository, which the profile needs for this kind. Add Repository = \"<url>\".".to_owned(),
                    ));
                }
            }
            "license"
                if !dynamic.contains(&"license")
                    && project.get("license").and_then(Item::as_str).is_none() =>
            {
                found.push(finding(
                    PYPROJECT,
                    "[project] license is a table, and the profile needs the SPDX expression of the license, such as license = \"MIT\", with the files in license-files.".to_owned(),
                ));
            }
            _ => {}
        }
    }
}
