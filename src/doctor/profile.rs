//! The target setup that `xpd doctor` holds a project to, from `profile.toml`, and the record a
//! project keeps of it in the `[tool.xpd.profile]` table of its `pyproject.toml`.
//!
//! ```toml
//! [tool.xpd.profile]
//! revision = 1
//! kind = "application"
//!
//! [[tool.xpd.profile.exceptions]]
//! tool = "ruff"
//! id = "PTH118"
//! files = ["scripts/**"]
//! reason = "The scripts pass str paths to a library that rejects Path objects."
//! ```
//!
//! The profile names each value, and the record of the same id in `decisions.jsonl` explains
//! it. A profile revision changes whenever a value changes, and the schema changes only when
//! the shape of `profile.toml` or of the project's record changes.
//!
//! The base values apply to every kind of project, and a kind can add its own values in the
//! same shape. A kind also states which of `[project]`, `[build-system]` and scripts with inline
//! metadata it needs, and which files the profile owns for it. An owned file is a native template
//! in `src/doctor/profiles/`, listed in [`TEMPLATES`], that the project keeps as an exact copy.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use toml_edit::{Item, TableLike};

use super::catalog::{self, Catalog, Decision, Level, Tool};
use crate::toml::{self, Toml};

/// The packaged profile.
const PROFILE: &str = include_str!("profile.toml");

/// The shape of `profile.toml` and of `[tool.xpd.profile]` that this xpd reads.
pub(crate) const SCHEMA: i64 = 1;

/// Where a project's record of the profile lives in its `pyproject.toml`.
pub(crate) const TABLE: [&str; 3] = ["tool", "xpd", "profile"];

/// How a kind of project locks its dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockStyle {
    /// `uv.lock` beside `pyproject.toml`, refreshed by `uv lock`.
    Project,
    /// `<script>.lock` beside each script with inline metadata, by `uv lock --script`.
    Script,
}

/// Whether a kind of project needs a part of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presence {
    Required,
    Forbidden,
    Allowed,
}

impl Presence {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "required" => Some(Self::Required),
            "forbidden" => Some(Self::Forbidden),
            "allowed" => Some(Self::Allowed),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct KindSpec {
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) lock: LockStyle,
    /// The `[project]` table of `pyproject.toml`.
    pub(crate) project: Presence,
    /// The `[build-system]` table of `pyproject.toml`.
    pub(crate) build_system: Presence,
    /// Python files with inline script metadata.
    pub(crate) scripts: Presence,
    /// Whether `pyproject.toml` holds the `[tool]` table and nothing else, so the profile adds
    /// no dependency to it.
    pub(crate) tool_only: bool,
    /// The names of the owned files, from `[files]`.
    pub(crate) files: Vec<String>,
    /// The values the kind adds to the base values.
    pub(crate) extra: Values,
}

/// A file that the profile owns for some kinds, which the project keeps as an exact copy of a
/// template.
#[derive(Debug)]
pub(crate) struct Owned {
    pub(crate) name: String,
    /// The path relative to the project root.
    pub(crate) path: String,
    /// The template's file name in `src/doctor/profiles/`.
    pub(crate) template: String,
    pub(crate) text: &'static str,
}

/// The templates of owned files, each by its file name in `src/doctor/profiles/`. Revision 1
/// owns no file.
const TEMPLATES: &[(&str, &str)] = &[];

/// What the profile wants of Ruff, ty and pytest, for every kind or added by one kind.
#[derive(Debug, Default, Clone)]
pub(crate) struct Values {
    /// Ruff rule codes that run on every file.
    pub(crate) ruff_rules: Vec<String>,
    /// ty rules, each with the lowest level it may have.
    pub(crate) ty_rules: Vec<(String, Level)>,
    /// For pytest 9 and later.
    pub(crate) pytest: PytestWanted,
    /// For pytest before 9.
    pub(crate) pytest_before_nine: PytestWanted,
    /// Plugins in the dependency list that holds pytest.
    pub(crate) plugins: Vec<String>,
}

impl Values {
    /// These values with `extra` added. A ty level in `extra` replaces the base level.
    fn with(&self, extra: &Self) -> Self {
        let mut values = self.clone();
        let add = |into: &mut Vec<String>, from: &[String]| {
            for item in from {
                if !into.contains(item) {
                    into.push(item.clone());
                }
            }
        };
        add(&mut values.ruff_rules, &extra.ruff_rules);
        for (rule, level) in &extra.ty_rules {
            values.ty_rules.retain(|(name, _)| name != rule);
            values.ty_rules.push((rule.clone(), *level));
        }
        for (into, from) in [
            (&mut values.pytest, &extra.pytest),
            (&mut values.pytest_before_nine, &extra.pytest_before_nine),
        ] {
            for (key, value) in &from.settings {
                into.settings.retain(|(name, _)| name != key);
                into.settings.push((key.clone(), *value));
            }
            add(&mut into.addopts, &from.addopts);
        }
        add(&mut values.plugins, &extra.plugins);
        values
    }

    /// The values in `table`, a tool table of `profile.toml` or of a kind. A missing table holds
    /// no value.
    fn read(table: &dyn TableLike) -> Result<Self> {
        let ruff_rules = optional(table, "ruff")?
            .map(|ruff| strings(ruff, "rules"))
            .transpose()?
            .unwrap_or_default();
        let ty_rules = match optional(table, "ty")?
            .map(|ty| optional(ty, "rules"))
            .transpose()?
        {
            Some(Some(rules)) => rules
                .iter()
                .map(|(name, item)| {
                    let level = item
                        .as_str()
                        .and_then(Level::parse)
                        .with_context(|| format!("ty.rules.{name} is no level"))?;
                    Ok((name.to_owned(), level))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => Vec::new(),
        };
        let (pytest, pytest_before_nine, plugins) = match optional(table, "pytest")? {
            None => Default::default(),
            Some(pytest) => (
                wanted(pytest)?,
                optional(pytest, "before-9")?
                    .map(wanted)
                    .transpose()?
                    .unwrap_or_default(),
                optional_strings(pytest, "plugins")?,
            ),
        };
        Ok(Self {
            ruff_rules,
            ty_rules,
            pytest,
            pytest_before_nine,
            plugins,
        })
    }
}

/// What the profile wants of pytest for one range of pytest releases.
#[derive(Debug, Default, Clone)]
pub(crate) struct PytestWanted {
    /// Settings of the pytest table, each with the boolean it must hold.
    pub(crate) settings: Vec<(String, bool)>,
    /// Arguments that `addopts` must hold.
    pub(crate) addopts: Vec<String>,
}

/// What the profile removes from a project. No exception can keep it.
#[derive(Debug, Default)]
pub(crate) struct PytestForbid {
    /// Flags of `addopts`, with the value that follows each.
    pub(crate) addopts: Vec<String>,
    /// Settings of the pytest table.
    pub(crate) settings: Vec<String>,
    /// Settings of the pytest table that must not be `false`.
    pub(crate) false_settings: Vec<String>,
    /// Distributions in a dependency list.
    pub(crate) plugins: Vec<String>,
    /// The names under which each plugin of the profile loads, which `-p no:<name>` turns off.
    pub(crate) plugin_names: Vec<(String, Vec<String>)>,
}

impl PytestForbid {
    /// Whether the profile removes `id` of pytest without exception.
    pub(crate) fn forbids(&self, id: &str) -> bool {
        [
            &self.addopts,
            &self.settings,
            &self.false_settings,
            &self.plugins,
        ]
        .into_iter()
        .flatten()
        .any(|forbidden| forbidden == id)
    }
}

#[derive(Debug)]
pub(crate) struct Profile {
    pub(crate) revision: i64,
    pub(crate) kinds: Vec<KindSpec>,
    /// The values for every kind.
    pub(crate) base: Values,
    pub(crate) forbid: PytestForbid,
    pub(crate) files: Vec<Owned>,
}

impl Profile {
    /// The packaged profile, checked against the catalog that explains it.
    ///
    /// # Errors
    ///
    /// Returns an error if `profile.toml` does not parse, names a release other than the
    /// catalog's, or names a value that no `profile` record of the catalog explains.
    pub(crate) fn packaged(catalog: &Catalog) -> Result<Self> {
        let doc = toml_edit::Document::parse(PROFILE.to_owned()).context("profile.toml")?;
        let profile = Self::read(&doc).context("profile.toml")?;
        profile.check(catalog).context("profile.toml")?;
        Ok(profile)
    }

    fn read(doc: &Toml) -> Result<Self> {
        let top = doc.as_table();
        let schema = integer(top, "schema")?;
        ensure!(
            schema == SCHEMA,
            "schema {schema}, and this xpd reads {SCHEMA}"
        );
        let pins = table(top, "pins")?;
        for tool in [Tool::Ruff, Tool::Ty, Tool::Pytest] {
            let pinned = string(pins, tool.name())?;
            ensure!(
                pinned == tool.pinned(),
                "pins {} {pinned}, and the catalog is reviewed for {}",
                tool.name(),
                tool.pinned()
            );
        }
        let kinds = table(top, "kinds")?
            .iter()
            .map(|(name, item)| {
                let kind = item
                    .as_table_like()
                    .with_context(|| format!("kinds.{name} is no table"))?;
                let lock = match string(kind, "lock")?.as_str() {
                    "project" => LockStyle::Project,
                    "script" => LockStyle::Script,
                    other => bail!("kinds.{name}.lock is {other:?}"),
                };
                let presence = |key: &str| {
                    let text = string(kind, key)?;
                    Presence::parse(&text).with_context(|| {
                        format!(
                            "kinds.{name}.{key} is {text:?}, not required, forbidden or allowed"
                        )
                    })
                };
                Ok(KindSpec {
                    name: name.to_owned(),
                    summary: string(kind, "summary")?,
                    lock,
                    project: presence("project")?,
                    build_system: presence("build-system")?,
                    scripts: presence("scripts")?,
                    tool_only: kind
                        .get("tool-only")
                        .and_then(Item::as_bool)
                        .with_context(|| format!("kinds.{name}.tool-only is no boolean"))?,
                    files: strings(kind, "files")?,
                    extra: Values::read(kind).with_context(|| format!("kinds.{name}"))?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let files = table(top, "files")?
            .iter()
            .map(|(name, item)| {
                let file = item
                    .as_table_like()
                    .with_context(|| format!("files.{name} is no table"))?;
                let template = string(file, "template")?;
                let text = TEMPLATES
                    .iter()
                    .find(|(known, _)| *known == template)
                    .map(|(_, text)| *text)
                    .with_context(|| format!("files.{name} names the template {template}, which src/doctor/profiles/ lacks"))?;
                Ok(Owned {
                    name: name.to_owned(),
                    path: string(file, "path")?,
                    template,
                    text,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let pytest = table(top, "pytest")?;
        let forbid = table(pytest, "forbid")?;
        let plugin_names = table(pytest, "plugin-names")?
            .iter()
            .map(|(plugin, item)| {
                let names = item
                    .as_array()
                    .with_context(|| format!("pytest.plugin-names.{plugin} is no array"))?
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect();
                Ok((plugin.to_owned(), names))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            revision: integer(top, "revision")?,
            kinds,
            base: Values::read(top)?,
            forbid: PytestForbid {
                addopts: strings(forbid, "addopts")?,
                settings: strings(forbid, "settings")?,
                false_settings: strings(forbid, "false")?,
                plugins: strings(forbid, "plugins")?,
                plugin_names,
            },
            files,
        })
    }

    /// Every value has a `profile` record in the catalog, and every such record a value.
    fn check(&self, catalog: &Catalog) -> Result<()> {
        let named: BTreeSet<(Tool, String)> = self.ids().into_iter().collect();
        for (tool, id) in &named {
            ensure!(
                catalog.profiled(*tool, id).is_some(),
                "{} {id} has no profile record in decisions.jsonl",
                tool.name()
            );
        }
        for record in catalog.records() {
            if record.decision == Decision::Profile {
                ensure!(
                    named.contains(&(record.key.tool, record.key.id.clone())),
                    "{} is a profile record that profile.toml does not name",
                    record.key
                );
            }
        }
        let all = self
            .kinds
            .iter()
            .fold(self.base.clone(), |values, kind| values.with(&kind.extra));
        for code in &all.ruff_rules {
            let known = catalog
                .ruff_rules()
                .any(|rule| rule.key.id == *code && !rule.is_removed());
            ensure!(known, "ruff {} has no rule {code}", catalog::RUFF_RELEASE);
        }
        for (rule, _) in &all.ty_rules {
            ensure!(
                catalog
                    .ty_rule(rule)
                    .is_some_and(|record| record.key.id == *rule),
                "ty {} has no rule {rule}",
                catalog::TY_RELEASE
            );
        }
        ensure!(!self.kinds.is_empty(), "no kinds");
        for kind in &self.kinds {
            for name in &kind.files {
                ensure!(
                    self.files.iter().any(|file| file.name == *name),
                    "kinds.{} owns the file {name}, which [files] lacks",
                    kind.name
                );
            }
        }
        for (template, _) in TEMPLATES {
            ensure!(
                self.files.iter().any(|file| file.template == *template),
                "src/doctor/profiles/{template} is no file of the profile"
            );
        }
        for (plugin, _) in &self.forbid.plugin_names {
            ensure!(
                all.plugins.contains(plugin),
                "pytest.plugin-names names {plugin}, which no kind adds"
            );
        }
        Ok(())
    }

    /// Each value as the tool and id of the catalog record that explains it.
    fn ids(&self) -> Vec<(Tool, String)> {
        let values = self
            .kinds
            .iter()
            .fold(self.base.clone(), |values, kind| values.with(&kind.extra));
        let ruff = values
            .ruff_rules
            .iter()
            .map(|code| (Tool::Ruff, code.clone()));
        let ty = values
            .ty_rules
            .iter()
            .map(|(rule, _)| (Tool::Ty, rule.clone()));
        let wanted = [&values.pytest, &values.pytest_before_nine]
            .into_iter()
            .flat_map(|wanted| {
                let settings = wanted.settings.iter().map(|(key, _)| key.clone());
                settings.chain(wanted.addopts.iter().cloned())
            });
        let forbid = &self.forbid;
        let pytest = wanted
            .chain(values.plugins.iter().cloned())
            .chain(forbid.addopts.iter().cloned())
            .chain(forbid.settings.iter().cloned())
            .chain(
                forbid
                    .false_settings
                    .iter()
                    .map(|key| format!("{key} = false")),
            )
            .chain(forbid.plugins.iter().cloned())
            .map(|id| (Tool::Pytest, id));
        ruff.chain(ty).chain(pytest).collect()
    }

    pub(crate) fn kind(&self, name: &str) -> Option<&KindSpec> {
        self.kinds.iter().find(|kind| kind.name == name)
    }

    pub(crate) fn kind_names(&self) -> Vec<&str> {
        self.kinds.iter().map(|kind| kind.name.as_str()).collect()
    }

    /// The values for the kind `kind`, or the base values without a kind.
    pub(crate) fn values(&self, kind: Option<&str>) -> Values {
        match kind.and_then(|kind| self.kind(kind)) {
            Some(kind) => self.base.with(&kind.extra),
            None => self.base.clone(),
        }
    }

    /// The files the profile owns for the kind `kind`.
    pub(crate) fn owned(&self, kind: Option<&str>) -> Vec<&Owned> {
        let Some(kind) = kind.and_then(|kind| self.kind(kind)) else {
            return Vec::new();
        };
        self.files
            .iter()
            .filter(|file| kind.files.contains(&file.name))
            .collect()
    }
}

fn wanted(table: &dyn TableLike) -> Result<PytestWanted> {
    let Some(settings) = optional(table, "settings")? else {
        return Ok(PytestWanted {
            settings: Vec::new(),
            addopts: optional_strings(table, "addopts")?,
        });
    };
    let settings = settings
        .iter()
        .map(|(key, item)| {
            let value = item
                .as_bool()
                .with_context(|| format!("settings.{key} is no boolean"))?;
            Ok((key.to_owned(), value))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PytestWanted {
        settings,
        addopts: optional_strings(table, "addopts")?,
    })
}

/// The table at `key`, `None` when it is missing.
fn optional<'a>(parent: &'a dyn TableLike, key: &str) -> Result<Option<&'a dyn TableLike>> {
    parent
        .get(key)
        .map(|item| {
            item.as_table_like()
                .with_context(|| format!("{key} is no table"))
        })
        .transpose()
}

/// The strings at `key`, none when it is missing.
fn optional_strings(parent: &dyn TableLike, key: &str) -> Result<Vec<String>> {
    if parent.contains_key(key) {
        strings(parent, key)
    } else {
        Ok(Vec::new())
    }
}

fn table<'a>(parent: &'a dyn TableLike, key: &str) -> Result<&'a dyn TableLike> {
    parent
        .get(key)
        .and_then(Item::as_table_like)
        .with_context(|| format!("{key} is no table"))
}

fn integer(parent: &dyn TableLike, key: &str) -> Result<i64> {
    parent
        .get(key)
        .and_then(Item::as_integer)
        .with_context(|| format!("{key} is no integer"))
}

fn string(parent: &dyn TableLike, key: &str) -> Result<String> {
    parent
        .get(key)
        .and_then(Item::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{key} is no string"))
}

fn strings(parent: &dyn TableLike, key: &str) -> Result<Vec<String>> {
    parent
        .get(key)
        .and_then(Item::as_array)
        .with_context(|| format!("{key} is no array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .with_context(|| format!("{key} holds a value that is no string"))
        })
        .collect()
}

/// What a project's `[tool.xpd.profile]` table records.
#[derive(Debug, Default)]
pub(crate) struct Record {
    /// The profile revision that the project's settings follow. `None` before the first apply.
    pub(crate) revision: Option<i64>,
    pub(crate) kind: Option<String>,
    pub(crate) exceptions: Vec<Exception>,
}

/// One deliberate exception, as the project records it. `problems` holds what is wrong with
/// its fields, which doctor reports beside the exception.
#[derive(Debug, Clone)]
pub(crate) struct Exception {
    pub(crate) tool: String,
    pub(crate) id: String,
    /// Ruff only: the file patterns the rule is off for. Empty for the whole project.
    pub(crate) files: Vec<String>,
    /// ty only: the level the rule keeps.
    pub(crate) value: Option<String>,
    pub(crate) reason: String,
    pub(crate) line: usize,
    pub(crate) problems: Vec<String>,
}

/// The keys an exception takes.
const EXCEPTION_KEYS: [&str; 5] = ["tool", "id", "files", "value", "reason"];

/// The project's record of the profile in `doc`, its `pyproject.toml`. `None` when the table is
/// missing. A table whose shape doctor cannot read is an error. A problem inside one exception
/// is not: it stays with that exception.
///
/// # Errors
///
/// Returns an error if the table, its `revision`, its `kind` or its `exceptions` have the wrong
/// type, or the table holds another key.
pub(crate) fn record(doc: &Toml) -> Result<Option<Record>> {
    let Some(item) = toml::get(doc, &TABLE) else {
        return Ok(None);
    };
    let name = TABLE.join(".");
    let table = item
        .as_table_like()
        .with_context(|| format!("pyproject.toml: [{name}] is no table"))?;
    if let Some((key, _)) = table
        .iter()
        .find(|(key, _)| !matches!(*key, "revision" | "kind" | "exceptions"))
    {
        bail!(
            "pyproject.toml: [{name}] has no key {key}. Its keys are revision, kind and exceptions"
        );
    }
    let revision = match table.get("revision") {
        None => None,
        Some(item) => Some(
            item.as_integer()
                .filter(|revision| *revision > 0)
                .with_context(|| {
                    format!("pyproject.toml: {name}.revision is no positive integer")
                })?,
        ),
    };
    let kind = match table.get("kind") {
        None => None,
        Some(item) => Some(
            item.as_str()
                .with_context(|| format!("pyproject.toml: {name}.kind is no string"))?
                .to_owned(),
        ),
    };
    let exceptions = match table.get("exceptions") {
        None => Vec::new(),
        Some(item) => item
            .as_array_of_tables()
            .with_context(|| {
                format!(
                    "pyproject.toml: {name}.exceptions is no array of tables, [[{name}.exceptions]]"
                )
            })?
            .iter()
            .map(|table| exception(doc, table))
            .collect(),
    };
    Ok(Some(Record {
        revision,
        kind,
        exceptions,
    }))
}

fn exception(doc: &Toml, table: &toml_edit::Table) -> Exception {
    let mut problems = Vec::new();
    for (key, _) in table {
        if !EXCEPTION_KEYS.contains(&key) {
            problems.push(format!(
                "it has the key {key}, and an exception takes {}",
                EXCEPTION_KEYS.join(", ")
            ));
        }
    }
    let mut text = |key: &str| match table.get(key) {
        None => {
            problems.push(format!("it has no {key}"));
            String::new()
        }
        Some(item) => item.as_str().map_or_else(
            || {
                problems.push(format!("its {key} is no string"));
                String::new()
            },
            |text| text.trim().to_owned(),
        ),
    };
    let tool = text("tool");
    let id = text("id");
    let reason = text("reason");
    let value = table.get("value").and_then(|item| {
        let value = item.as_str().map(str::to_owned);
        if value.is_none() {
            problems.push("its value is no string".to_owned());
        }
        value
    });
    let files = match table.get("files") {
        None => Vec::new(),
        Some(item) => {
            let files: Option<Vec<String>> = item.as_array().and_then(|array| {
                array
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect()
            });
            files
                .filter(|files| !files.is_empty() && files.iter().all(|file| !file.is_empty()))
                .unwrap_or_else(|| {
                    problems.push("its files is no array of file patterns".to_owned());
                    Vec::new()
                })
        }
    };
    if reason.is_empty() && table.get("reason").is_some() {
        problems.push("its reason is empty".to_owned());
    }
    let line = table
        .span()
        .map_or(1, |span| toml::line_of(doc, span.start));
    Exception {
        tool,
        id,
        files,
        value,
        reason,
        line,
        problems,
    }
}
