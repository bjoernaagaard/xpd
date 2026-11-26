//! ty's effective settings for the project.
//!
//! ty reads a `ty.toml`, or the `[tool.ty]` table of a `pyproject.toml`, from the project
//! folder or the nearest folder above it that has one. A `ty.toml` takes precedence over the
//! `pyproject.toml` beside it. A rule's level comes from the rules table, by its current name or
//! a historical alias, then from the table's `all` key, then from the release's default.
//! `ty explain rule` gives the installed release's rules and defaults.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::catalog::{Catalog, Key, Kind, Level, Surface, Tool};
use super::evidence::Location;
use super::placement::Destination;
use super::probe::Checker;
use crate::toml::{self, Observed, Toml};

/// A level the project sets, and where.
#[derive(Debug, Clone)]
pub(crate) struct Set {
    pub(crate) level: Level,
    pub(crate) at: Location,
}

/// An override table that sets rule levels for the files its patterns match.
#[derive(Debug)]
pub(crate) struct Override {
    pub(crate) include: Vec<String>,
    pub(crate) rules: BTreeMap<String, Set>,
}

#[derive(Debug)]
pub(crate) struct State {
    /// Levels by current rule name.
    pub(crate) rules: BTreeMap<String, Set>,
    pub(crate) all: Option<Set>,
    pub(crate) overrides: Vec<Override>,
    /// The value of each setting that a suggestion names, by its key under `tool.ty`.
    pub(crate) settings: BTreeMap<String, Observed>,
}

#[derive(Debug)]
pub(crate) struct Ty {
    pub(crate) checker: Checker,
    pub(crate) config: Option<PathBuf>,
    pub(crate) destination: Destination,
    pub(crate) state: Result<State, String>,
    pub(crate) notes: Vec<String>,
    /// The installed release's rules and their default levels, when ty runs here.
    pub(crate) installed: Option<BTreeMap<String, Level>>,
}

impl Ty {
    /// The level ty applies to `rule` outside any override, and the place the project sets it.
    pub(crate) fn level(&self, catalog: &Catalog, rule: &str) -> Option<(Level, Option<&Set>)> {
        let state = self.state.as_ref().ok()?;
        if let Some(set) = state.rules.get(rule).or(state.all.as_ref()) {
            return Some((set.level, Some(set)));
        }
        let default = match &self.installed {
            Some(installed) => *installed.get(rule)?,
            None => Level::parse(catalog.ty_rule(rule)?.default.as_deref()?)?,
        };
        Some((default, None))
    }
}

pub(crate) fn analyze(root: &Path, catalog: &Catalog, checker: Checker) -> Ty {
    let mut notes = Vec::new();
    let (config, first_pyproject) = discover(root, &mut notes);
    let file = config
        .clone()
        .or(first_pyproject)
        .unwrap_or_else(|| root.join("pyproject.toml"));
    let prefix: &'static [&'static str] = if file.file_name().is_some_and(|name| name == "ty.toml")
    {
        &[]
    } else {
        &["tool", "ty"]
    };
    let parsed = toml::read(&file);
    let state = match &parsed {
        Ok(doc) => Ok(state(catalog, &file, prefix, doc.as_ref())),
        Err(error) => Err(error.clone()),
    };
    if let Some(user) = user_config().filter(|path| path.is_file()) {
        notes.push(format!(
            "{} also applies on this machine; doctor judges the project's own settings",
            user.display()
        ));
    }
    let installed = installed(catalog, &checker, root, &mut notes);
    Ty {
        checker,
        config,
        destination: Destination {
            file,
            prefix,
            doc: parsed.ok().flatten(),
        },
        state,
        notes,
        installed,
    }
}

/// The file ty reads its project settings from, and the nearest `pyproject.toml`, which is where
/// a project without ty settings would add them.
fn discover(root: &Path, notes: &mut Vec<String>) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut first_pyproject = None;
    for dir in root.ancestors() {
        let ty = dir.join("ty.toml");
        let pyproject = dir.join("pyproject.toml");
        let has_table = |path: &Path| match toml::read(path) {
            Ok(Some(doc)) => toml::get(&doc, &["tool", "ty"]).is_some(),
            Ok(None) => false,
            Err(_) => true,
        };
        if ty.is_file() {
            if pyproject.is_file() && has_table(&pyproject) {
                notes.push(format!(
                    "{} takes precedence, so ty ignores the [tool.ty] table of {}",
                    ty.display(),
                    pyproject.display()
                ));
            }
            return (Some(ty), first_pyproject);
        }
        if pyproject.is_file() {
            if has_table(&pyproject) {
                return (Some(pyproject), first_pyproject);
            }
            first_pyproject.get_or_insert(pyproject);
        }
    }
    (None, first_pyproject)
}

fn user_config() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("ty").join("ty.toml"))
}

/// The rule levels and settings that `doc`, the file `file` with ty's table at `prefix`, sets.
pub(crate) fn state(catalog: &Catalog, file: &Path, prefix: &[&str], doc: Option<&Toml>) -> State {
    let mut state = State {
        rules: BTreeMap::new(),
        all: None,
        overrides: Vec::new(),
        settings: settings(catalog, prefix, doc),
    };
    let Some(doc) = doc else {
        return state;
    };
    let rules_path: Vec<&str> = prefix.iter().copied().chain(["rules"]).collect();
    if let Some(table) = toml::get(doc, &rules_path).and_then(|item| item.as_table_like()) {
        let (levels, all) = levels(catalog, table, |name, level| {
            let path: Vec<&str> = rules_path.iter().copied().chain([name]).collect();
            Set {
                level,
                at: Location {
                    file: file.to_path_buf(),
                    line: toml::line(doc, &path).unwrap_or(1),
                    text: format!("{} = \"{}\"", path.join("."), level.name()),
                },
            }
        });
        state.rules = levels;
        state.all = all;
    }
    state.overrides = overrides(catalog, file, prefix, doc);
    state
}

/// The value of each setting a suggestion names, from the file or else the pinned default.
fn settings(catalog: &Catalog, prefix: &[&str], doc: Option<&Toml>) -> BTreeMap<String, Observed> {
    let keys: BTreeSet<&str> = catalog.setting_keys(Tool::Ty).collect();
    let mut settings = BTreeMap::new();
    for key in keys {
        let path: Vec<&str> = prefix.iter().copied().chain(key.split('.')).collect();
        let observed = doc
            .and_then(|doc| toml::get(doc, &path))
            .and_then(Observed::of_item)
            .or_else(|| {
                let id = format!("tool.ty.{key}");
                let record =
                    catalog.get(&Key::pinned(Tool::Ty, Surface::Project, Kind::Option, &id))?;
                Observed::parse(record.default.as_deref()?.trim_matches('`'))
            });
        if let Some(observed) = observed {
            settings.insert(key.to_owned(), observed);
        }
    }
    settings
}

/// The override tables that set rule levels, each with its `include` patterns, in either
/// spelling of an array of tables.
fn overrides(catalog: &Catalog, file: &Path, prefix: &[&str], doc: &Toml) -> Vec<Override> {
    let path: Vec<&str> = prefix.iter().copied().chain(["overrides"]).collect();
    let Some(item) = toml::get(doc, &path) else {
        return Vec::new();
    };
    toml::tables(item)
        .into_iter()
        .filter_map(|(table, start)| {
            let include = table
                .get("include")
                .and_then(|item| item.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let line = start.map_or(1, |start| toml::line_of(doc, start));
            let rules = table.get("rules").and_then(|item| item.as_table_like())?;
            let (rules, _) = levels(catalog, rules, |name, level| Set {
                level,
                at: Location {
                    file: file.to_path_buf(),
                    line,
                    text: format!("overrides.rules.{name} = \"{}\"", level.name()),
                },
            });
            Some(Override { include, rules })
        })
        .collect()
}

/// The levels a rules table sets, by current rule name, and its `all` level. A current name
/// takes precedence over an alias of the same rule in one table.
fn levels(
    catalog: &Catalog,
    table: &dyn toml_edit::TableLike,
    set: impl Fn(&str, Level) -> Set,
) -> (BTreeMap<String, Set>, Option<Set>) {
    let mut by_alias = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    let mut all = None;
    for (name, item) in table.iter() {
        let Some(level) = item.as_str().and_then(Level::parse) else {
            continue;
        };
        if name == "all" {
            all = Some(set(name, level));
        } else if let Some(rule) = catalog.ty_rule(name) {
            let target = if rule.key.id == name {
                &mut by_name
            } else {
                &mut by_alias
            };
            target.insert(rule.key.id.clone(), set(name, level));
        }
    }
    by_alias.extend(by_name);
    (by_alias, all)
}

/// The installed release's rules with their default levels, and notes on how they differ from
/// the pinned release's.
fn installed(
    catalog: &Catalog,
    checker: &Checker,
    root: &Path,
    notes: &mut Vec<String>,
) -> Option<BTreeMap<String, Level>> {
    let json = match checker.run(&["explain", "rule", "--output-format", "json"], root)? {
        Ok(json) => json,
        Err(error) => {
            notes.push(format!(
                "ty explain rule failed, so the installed rules are unknown: {error}"
            ));
            return None;
        }
    };
    let decoded = match serde_json::from_str::<Vec<serde_json::Value>>(&json) {
        Ok(decoded) => decoded,
        Err(error) => {
            notes.push(format!(
                "ty explain rule returned invalid JSON, so the installed rules are unknown: {error}"
            ));
            return None;
        }
    };
    let rules: BTreeMap<String, Level> = decoded
        .iter()
        .filter_map(|rule| {
            Some((
                rule["name"].as_str()?.to_owned(),
                Level::parse(rule["default_level"].as_str()?)?,
            ))
        })
        .collect();
    if checker.is_pinned() {
        return Some(rules);
    }
    let version = &checker.installed.as_ref()?.version;
    let pinned: BTreeMap<&str, Option<Level>> = catalog
        .records()
        .iter()
        .filter(|record| record.key.tool == Tool::Ty && record.key.kind == Kind::Rule)
        .map(|record| {
            (
                record.key.id.as_str(),
                record.default.as_deref().and_then(Level::parse),
            )
        })
        .collect();
    let newer: Vec<&str> = rules
        .keys()
        .map(String::as_str)
        .filter(|name| !pinned.contains_key(name))
        .collect();
    if !newer.is_empty() {
        notes.push(format!(
            "ty {version} has {} rules without a reviewed decision: {}",
            newer.len(),
            newer.join(", ")
        ));
    }
    let missing: Vec<&str> = pinned
        .keys()
        .copied()
        .filter(|name| !rules.contains_key(*name))
        .collect();
    if !missing.is_empty() {
        notes.push(format!(
            "ty {version} lacks {} rules of {}: {}",
            missing.len(),
            Tool::Ty.pinned(),
            missing.join(", ")
        ));
    }
    let changed: Vec<String> = rules
        .iter()
        .filter_map(|(name, level)| {
            let before = (*pinned.get(name.as_str())?)?;
            (before != *level).then(|| format!("{name} {} -> {}", before.name(), level.name()))
        })
        .collect();
    if !changed.is_empty() {
        notes.push(format!(
            "default levels changed in ty {version}: {}",
            changed.join(", ")
        ));
    }
    Some(rules)
}
