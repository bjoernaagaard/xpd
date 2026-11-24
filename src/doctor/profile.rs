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

#[derive(Debug)]
pub(crate) struct KindSpec {
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) lock: LockStyle,
}

/// What the profile wants of pytest for one range of pytest releases.
#[derive(Debug, Default)]
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
}

#[derive(Debug)]
pub(crate) struct Profile {
    pub(crate) revision: i64,
    pub(crate) kinds: Vec<KindSpec>,
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
    pub(crate) forbid: PytestForbid,
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
                Ok(KindSpec {
                    name: name.to_owned(),
                    summary: string(kind, "summary")?,
                    lock,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let ruff_rules = strings(table(top, "ruff")?, "rules")?;
        let ty_rules = table(table(top, "ty")?, "rules")?
            .iter()
            .map(|(name, item)| {
                let level = item
                    .as_str()
                    .and_then(Level::parse)
                    .with_context(|| format!("ty.rules.{name} is no level"))?;
                Ok((name.to_owned(), level))
            })
            .collect::<Result<Vec<_>>>()?;
        let pytest = table(top, "pytest")?;
        let forbid = table(pytest, "forbid")?;
        Ok(Self {
            revision: integer(top, "revision")?,
            kinds,
            ruff_rules,
            ty_rules,
            pytest: wanted(pytest)?,
            pytest_before_nine: wanted(table(pytest, "before-9")?)?,
            plugins: strings(pytest, "plugins")?,
            forbid: PytestForbid {
                addopts: strings(forbid, "addopts")?,
                settings: strings(forbid, "settings")?,
                false_settings: strings(forbid, "false")?,
                plugins: strings(forbid, "plugins")?,
            },
        })
    }

    /// Every value has a `profile` record in the catalog, and every such record a value.
    fn check(&self, catalog: &Catalog) -> Result<()> {
        let named: BTreeSet<(Tool, String)> = self.ids().collect();
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
        for code in &self.ruff_rules {
            let known = catalog
                .ruff_rules()
                .any(|rule| rule.key.id == *code && !rule.is_removed());
            ensure!(known, "ruff {} has no rule {code}", catalog::RUFF_RELEASE);
        }
        for (rule, _) in &self.ty_rules {
            ensure!(
                catalog
                    .ty_rule(rule)
                    .is_some_and(|record| record.key.id == *rule),
                "ty {} has no rule {rule}",
                catalog::TY_RELEASE
            );
        }
        ensure!(!self.kinds.is_empty(), "no kinds");
        Ok(())
    }

    /// Each value as the tool and id of the catalog record that explains it.
    fn ids(&self) -> impl Iterator<Item = (Tool, String)> + '_ {
        let ruff = self
            .ruff_rules
            .iter()
            .map(|code| (Tool::Ruff, code.clone()));
        let ty = self
            .ty_rules
            .iter()
            .map(|(rule, _)| (Tool::Ty, rule.clone()));
        let wanted = [&self.pytest, &self.pytest_before_nine]
            .into_iter()
            .flat_map(|wanted| {
                let settings = wanted.settings.iter().map(|(key, _)| key.clone());
                settings.chain(wanted.addopts.iter().cloned())
            });
        let forbid = &self.forbid;
        let pytest = wanted
            .chain(self.plugins.iter().cloned())
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
        ruff.chain(ty).chain(pytest)
    }

    pub(crate) fn kind(&self, name: &str) -> Option<&KindSpec> {
        self.kinds.iter().find(|kind| kind.name == name)
    }

    pub(crate) fn kind_names(&self) -> Vec<&str> {
        self.kinds.iter().map(|kind| kind.name.as_str()).collect()
    }

    /// Whether the profile requires `id` of `tool`, so that an exception can apply to it.
    pub(crate) fn requires(&self, tool: Tool, id: &str) -> bool {
        match tool {
            Tool::Ruff => self.ruff_rules.iter().any(|code| code == id),
            Tool::Ty => self.ty_rules.iter().any(|(rule, _)| rule == id),
            Tool::Pytest => {
                [&self.pytest, &self.pytest_before_nine]
                    .into_iter()
                    .any(|wanted| {
                        wanted.settings.iter().any(|(key, _)| key == id)
                            || wanted.addopts.iter().any(|arg| arg == id)
                    })
                    || self.plugins.iter().any(|plugin| plugin == id)
            }
        }
    }

    /// Whether the profile removes `id` of pytest without exception.
    pub(crate) fn forbids(&self, id: &str) -> bool {
        let forbid = &self.forbid;
        [
            &forbid.addopts,
            &forbid.settings,
            &forbid.false_settings,
            &forbid.plugins,
        ]
        .into_iter()
        .flatten()
        .any(|forbidden| forbidden == id)
    }
}

fn wanted(table: &dyn TableLike) -> Result<PytestWanted> {
    let settings = self::table(table, "settings")?
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
        addopts: strings(table, "addopts")?,
    })
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
