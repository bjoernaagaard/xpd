//! Whether the pinned Ruff and ty accept the settings of a `pyproject.toml`. Ruff and ty stop
//! with an error on a key they do not know, and Ruff also on a rule selector it does not know,
//! so a project with such a setting cannot match the profile.
//!
//! [`problems`] reads the file against the catalog of the pinned releases and runs no tool.
//! [`ruff_rejects`] runs the pinned Ruff on a copy of the settings when it is installed, which
//! also catches a value of the wrong type.

use std::path::Path;
use std::process::{Command, Stdio};

use toml_edit::{Item, TableLike};

use super::catalog::{self, Catalog, Key, Kind, Surface, Tool};
use super::plan::dotted;
use super::probe::Checker;
use super::profile::Scope;
use super::ruff;
use crate::private::{self, Scratch};
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
/// rules, for each of the two that the project holds in `scope`.
pub(crate) fn problems(doc: &Toml, catalog: &Catalog, scope: &Scope) -> Vec<Problem> {
    let mut found = Vec::new();
    if let Some(table) = toml::get(doc, &["tool", "ruff"])
        .and_then(Item::as_table_like)
        .filter(|_| scope.holds(Tool::Ruff))
    {
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
    if let Some(table) = toml::get(doc, &["tool", "ty"])
        .and_then(Item::as_table_like)
        .filter(|_| scope.holds(Tool::Ty))
    {
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
        ty_rules(doc, catalog, &mut found);
    }
    found
}

/// Records a problem for each rule name in `[tool.ty.rules]` and in the `rules` of each
/// override that the pinned ty does not have, which ty reports as `unknown-rule` and which
/// fails its check.
fn ty_rules(doc: &Toml, catalog: &Catalog, found: &mut Vec<Problem>) {
    let release = catalog::TY_RELEASE;
    let mut tables: Vec<(String, &dyn TableLike, Option<usize>)> = Vec::new();
    if let Some(rules) = toml::get(doc, &["tool", "ty", "rules"]).and_then(Item::as_table_like) {
        tables.push(("tool.ty.rules".to_owned(), rules, None));
    }
    if let Some(overrides) = toml::get(doc, &["tool", "ty", "overrides"]) {
        for (index, (table, start)) in toml::tables(overrides).into_iter().enumerate() {
            if let Some(rules) = table.get("rules").and_then(Item::as_table_like) {
                let line = start.map(|start| toml::line_of(doc, start));
                tables.push((format!("tool.ty.overrides[{index}].rules"), rules, line));
            }
        }
    }
    for (shown, rules, line) in tables {
        for (name, item) in rules.iter() {
            if name == "all" || catalog.ty_rule(name).is_some() {
                continue;
            }
            let removed = Key::pinned(Tool::Ty, Surface::Rules, Kind::Removed, name);
            let what = if catalog.get(&removed).is_some() {
                format!("ty {release} removed the rule {name}")
            } else {
                format!("ty {release} has no rule {name}")
            };
            let line = rules
                .key(name)
                .and_then(toml_edit::Key::span)
                .or_else(|| item.span())
                .map(|span| toml::line_of(doc, span.start))
                .or(line);
            found.push(Problem {
                line,
                message: format!(
                    "{shown} sets {name}, and {what}, so ty reports unknown-rule and its check fails. Remove it, or name a rule of ty {release}."
                ),
            });
        }
    }
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
                } else if catalog.ruff_code(target) != target && !preview(doc) {
                    Some(format!(
                        "{shown} names the rule {selector} by its name, which Ruff {release} reads only in preview mode, so Ruff stops with an error. Name its code, {}, or turn preview on.",
                        catalog.ruff_code(target)
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

/// Whether `doc` turns Ruff's preview mode on for the linter, in `lint.preview` or else in the
/// top-level `preview`.
fn preview(doc: &Toml) -> bool {
    [
        &["tool", "ruff", "lint", "preview"][..],
        &["tool", "ruff", "preview"],
    ]
    .into_iter()
    .find_map(|path| toml::get(doc, path).and_then(Item::as_bool))
    .unwrap_or(false)
}

/// Whether uv may fetch the pinned Ruff from the network when its cache lacks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fetch {
    /// From uv's cache only, as plain `xpd doctor` does.
    Offline,
    /// From the cache, and else from the index, as `--check` and `--apply` do.
    Online,
}

/// What the pinned Ruff says about a `pyproject.toml`.
#[derive(Debug)]
pub(crate) enum Verdict {
    Accepts,
    /// It stops with this error.
    Rejects(String),
    /// No route to the pinned Ruff worked, for this reason.
    Unverified(String),
}

/// What the pinned Ruff says about the `pyproject.toml` text `text` of the project at `root`.
/// It runs the Ruff in the project's environment or on `PATH` when that is the pinned release,
/// and else `uv tool run --from ruff==<pin>`, from uv's cache first. Either way the answer does
/// not depend on whether the project's environment is synced.
///
/// Ruff gets the `[tool.ruff]` table alone, as a `ruff.toml` that `--config` names, so no other
/// setting of `pyproject.toml`, such as an index URL with a token, is written anywhere. The file
/// sits in a [`Scratch`] folder, mode 0700 with a name no other process can predict, which is
/// removed on every way out.
pub(crate) fn ruff_verdict(root: &Path, text: &str, fetch: Fetch) -> Verdict {
    let scratch = match Scratch::new("xpd-doctor") {
        Ok(scratch) => scratch,
        Err(error) => {
            return Verdict::Unverified(format!("cannot make a private scratch folder: {error}"));
        }
    };
    let folder = scratch.path();
    let config = folder.join("ruff.toml");
    let file = folder.join("check.py");
    let written =
        private::write_new(&config, &ruff_table(text)).and_then(|()| private::write_new(&file, ""));
    if let Err(error) = written {
        return Verdict::Unverified(format!("cannot write the Ruff settings: {error}"));
    }
    let ruff_args = |command: &mut Command| {
        command
            .args(["check", "--no-cache", "--show-settings", "--config"])
            .arg(&config)
            .arg(&file);
    };
    let checker = Checker::find(Tool::Ruff, root);
    if let Some(installed) = checker.installed.as_ref().filter(|_| checker.is_pinned()) {
        let mut command = Command::new(&installed.path);
        ruff_args(&mut command);
        return run(&mut command, root, folder);
    }
    let from = format!("ruff=={}", catalog::RUFF_RELEASE);
    let uv = |offline: bool| {
        let mut command = Command::new("uv");
        if offline {
            command.arg("--offline");
        }
        command.args(["--quiet", "tool", "run", "--from", &from, "ruff"]);
        ruff_args(&mut command);
        run(&mut command, root, folder)
    };
    match (uv(true), fetch) {
        (Verdict::Unverified(_), Fetch::Online) => uv(false),
        (verdict, _) => verdict,
    }
}

/// Whether the locked pytest of the project at `root` starts with the settings on disk. It runs
/// `uv run --locked pytest --markers`, which loads each plugin that `addopts` names and reads
/// each option, and imports no test module. `--help` would not do, because pytest prints its
/// help before it rejects an option it does not know. uv syncs the project's environment first
/// when it has to.
pub(crate) fn pytest_verdict(root: &Path) -> Verdict {
    let output = Command::new("uv")
        .args(["run", "--locked", "--quiet", "pytest", "--markers"])
        .args(["-p", "no:cacheprovider"])
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .current_dir(root)
        .stdin(Stdio::null())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => return Verdict::Unverified(format!("cannot run uv: {error}")),
    };
    if output.status.success() {
        return Verdict::Accepts;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<String> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(crate::output::hosts_only)
        .collect();
    let errors: Vec<&str> = lines
        .iter()
        .filter_map(|line| {
            line.strip_prefix("ERROR:")
                .or_else(|| line.strip_prefix("pytest: error:"))
        })
        .map(str::trim)
        .filter(|line| !line.starts_with("usage:"))
        .collect();
    if errors.is_empty() {
        Verdict::Unverified(lines.join(" "))
    } else {
        Verdict::Rejects(errors.join(" "))
    }
}

/// The `[tool.ruff]` table of the `pyproject.toml` text `text` as the text of a `ruff.toml`,
/// whose keys are those of `[tool.ruff]` at its top. Empty when the table is missing.
fn ruff_table(text: &str) -> String {
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return String::new();
    };
    let table = match doc.get("tool").and_then(|tool| tool.get("ruff")) {
        Some(Item::Table(table)) => table.clone(),
        Some(Item::Value(toml_edit::Value::InlineTable(table))) => table.clone().into_table(),
        _ => return String::new(),
    };
    let mut ruff = toml_edit::DocumentMut::new();
    for (key, item) in &table {
        ruff.insert(key, item.clone());
    }
    ruff.to_string()
}

/// Runs a command of the pinned Ruff in `root` on a copy of the settings in `folder`.
fn run(command: &mut Command, root: &Path, folder: &Path) -> Verdict {
    let output = match command.current_dir(root).stdin(Stdio::null()).output() {
        Ok(output) => output,
        Err(error) => return Verdict::Unverified(format!("cannot run uv: {error}")),
    };
    if output.status.success() {
        return Verdict::Accepts;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let shown = folder.display().to_string();
    let lines: Vec<String> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('|') && !line.starts_with("warning:"))
        .filter(|line| {
            line.split_once('|')
                .is_none_or(|(number, _)| number.trim().parse::<usize>().is_err())
        })
        .map(|line| line.strip_prefix("Cause:").map_or(line, str::trim))
        .map(|line| line.replace(&format!("{shown}/"), "").replace(&shown, "."))
        .map(|line| crate::output::hosts_only(&line))
        .collect();
    if stderr.contains("ruff failed") {
        let cause: Vec<String> = lines
            .into_iter()
            .filter(|line| line != "ruff failed")
            .collect();
        Verdict::Rejects(cause.join(" "))
    } else {
        Verdict::Unverified(lines.join(" "))
    }
}
