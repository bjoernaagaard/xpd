//! The target setup that `xpd doctor` holds a project to, from `profile.toml`, and the record a
//! project keeps of it in the `[tool.xpd.profile]` table of its `pyproject.toml`.
//!
//! ```toml
//! [tool.xpd.profile]
//! revision = 4
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
//! it. Ruff's `select` and `ignore` decide every rule of the pinned release, so each rule has a
//! `profile` record, on or off, and a setting is explained by the record of the setting that
//! holds it. A profile revision changes whenever a value changes, and the schema changes only
//! when the shape of `profile.toml` or of the project's record changes.
//!
//! The base values apply to every kind of project, and a kind can add its own values in the
//! same shape. A kind also states which of `[project]`, `[build-system]` and scripts with inline
//! metadata it needs, and which files the profile owns for it. An owned file is a native template
//! in `src/doctor/profiles/`, listed in [`TEMPLATES`], that the project keeps as an exact copy,
//! or, with `holds`, a file the project also writes itself, which must hold that line and is the
//! template when the project has none, or, with `insert`, a file of lines such as `.gitignore`,
//! which must hold every line of the template and gets the missing ones at its top. `rivals` and
//! `tables` name the other files and `pyproject.toml` tables where the same tool would read its
//! settings, which the project must not have.
//!
//! `[dev]` `pins` names the tools a project runs with `uv run`. Each goes into
//! `[dependency-groups] dev` as an exact pin at its release in `[pins]`, and `uv.lock` locks that
//! release. `shell` and `notebooks` name the tools that only a project with shell scripts, or
//! with notebooks, gets. `[pytest.plugin-names]` holds the names under which `-p` loads each
//! plugin, by its `pytest11` entry point or its module: `-p no:<name>` turns off a plugin of the
//! profile, and `-p <name>` loads a plugin that the profile removes.
//!
//! `[packaging]` holds uv's cooldown, the Python floor, the build backends, uv's defaults and the
//! old packaging files, which `packaging.rs` holds a project to.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use toml_edit::{Item, TableLike};

use super::catalog::{self, Catalog, Decision, Key, Kind, Level, Surface, Tool};
use crate::toml::{self, Toml};

/// The packaged profile.
const PROFILE: &str = include_str!("profile.toml");

/// The shape of `profile.toml` and of `[tool.xpd.profile]` that this xpd reads.
pub(crate) const SCHEMA: i64 = 4;

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
    /// The keys that `[project]` declares, or lists in `dynamic`.
    pub(crate) metadata: Vec<String>,
    /// The values the kind adds to the base values.
    pub(crate) extra: Values,
}

/// A file that the profile owns for some kinds, which the project keeps as an exact copy of a
/// template, or, for a file the project also writes itself, such as `conftest.py`, which holds
/// one line of the template and is the template when the project has none.
#[derive(Debug)]
pub(crate) struct Owned {
    pub(crate) name: String,
    /// The path relative to the project root.
    pub(crate) path: String,
    /// The template's file name in `src/doctor/profiles/`.
    pub(crate) template: String,
    pub(crate) text: &'static str,
    /// The line that a file the project writes itself must hold. `None` for an exact copy.
    pub(crate) holds: Option<String>,
    /// Whether the file must hold every line of the template, which apply puts at its top.
    pub(crate) insert: bool,
    /// Other files, by their path relative to the project root, from which the same tool would
    /// read its settings.
    pub(crate) rivals: Vec<String>,
    /// Tables of `pyproject.toml`, such as `tool.tombi`, from which the same tool would read its
    /// settings.
    pub(crate) tables: Vec<String>,
}

/// Which files a project tracks that only some tools check.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Tracked {
    /// Shell scripts that `ShellCheck` and `shfmt` read: a `.sh` or `.bash` file, or a file without
    /// a suffix whose first line runs `sh`, `bash`, `dash` or `ksh`.
    pub(crate) shell: bool,
    /// Jupyter notebooks.
    pub(crate) notebooks: bool,
}

/// The templates of owned files, each by its file name in `src/doctor/profiles/`.
const TEMPLATES: &[(&str, &str)] = &[
    (
        "conftest.py.template",
        include_str!("profiles/conftest.py.template"),
    ),
    (
        "editorconfig.template",
        include_str!("profiles/editorconfig.template"),
    ),
    (
        "gitattributes.template",
        include_str!("profiles/gitattributes.template"),
    ),
    (
        "gitignore.template",
        include_str!("profiles/gitignore.template"),
    ),
    (
        "tombi.toml.template",
        include_str!("profiles/tombi.toml.template"),
    ),
    (
        "rumdl.toml.template",
        include_str!("profiles/rumdl.toml.template"),
    ),
    (
        "yamllint.yml.template",
        include_str!("profiles/yamllint.yml.template"),
    ),
];

/// One setting of a tool table, by its path below the table, with the value the profile wants.
#[derive(Debug, Clone)]
pub(crate) struct Setting {
    pub(crate) path: Vec<String>,
    pub(crate) value: toml_edit::Value,
}

impl Setting {
    /// The path joined with dots, as Ruff and ty name the setting.
    pub(crate) fn dotted(&self) -> String {
        self.path.join(".")
    }
}

/// What the profile wants of Ruff.
#[derive(Debug, Default, Clone)]
pub(crate) struct RuffWanted {
    /// `lint.select`, which the profile owns.
    pub(crate) select: Vec<String>,
    /// Selectors that `lint.ignore` holds. Each turns off rules that a reason in the catalog
    /// explains.
    pub(crate) ignore: Vec<String>,
    /// Patterns of `lint.per-file-ignores`, each with the selectors it holds.
    pub(crate) per_file_ignores: Vec<(String, Vec<String>)>,
    /// Entries of `lint.flake8-tidy-imports.banned-api`, each with the message it gets when doctor
    /// adds it. An entry the project has keeps its own message.
    pub(crate) banned_api: Vec<(String, String)>,
    /// Settings below `[tool.ruff]` that hold exactly the profile's value.
    pub(crate) settings: Vec<Setting>,
    /// Settings below `[tool.ruff]` that must not be `true`.
    pub(crate) off: Vec<Vec<String>>,
    /// The rules that the profile's `lint.pydocstyle.convention` turns off in the pinned release,
    /// as `ruff check --show-settings` lists them with and without the convention.
    pub(crate) convention_off: Vec<String>,
}

impl RuffWanted {
    /// Whether the profile turns Ruff's preview mode on for the linter.
    pub(crate) fn preview(&self) -> bool {
        ["lint.preview", "preview"].into_iter().any(|key| {
            self.settings
                .iter()
                .find(|setting| setting.dotted() == key)
                .is_some_and(|setting| setting.value.as_bool() == Some(true))
        })
    }

    /// The rules of the pinned release that the profile turns on: those its selection selects
    /// and no selector of its ignore list turns off.
    pub(crate) fn required(&self, catalog: &Catalog) -> Vec<String> {
        self.decided(catalog)
            .into_iter()
            .filter(|(_, on)| *on)
            .map(|(code, _)| code)
            .collect()
    }

    /// Each rule of the pinned release that the profile's selection reaches, with whether it
    /// stays on. A preview rule counts only when the profile turns preview on.
    pub(crate) fn decided(&self, catalog: &Catalog) -> Vec<(String, bool)> {
        let preview = self.preview();
        catalog
            .ruff_rules()
            .filter(|rule| !rule.is_removed() && (preview || !rule.is_preview()))
            .filter(|rule| {
                self.select
                    .iter()
                    .any(|selector| super::ruff::selects(selector, rule))
            })
            .map(|rule| {
                let off = self.convention_off.contains(&rule.key.id)
                    || self
                        .ignore
                        .iter()
                        .any(|selector| super::ruff::selects(selector, rule));
                (rule.key.id.clone(), !off)
            })
            .collect()
    }

    fn with(&mut self, extra: &Self) {
        add(&mut self.select, &extra.select);
        add(&mut self.ignore, &extra.ignore);
        for (pattern, selectors) in &extra.per_file_ignores {
            match self
                .per_file_ignores
                .iter_mut()
                .find(|(known, _)| known == pattern)
            {
                Some((_, known)) => add(known, selectors),
                None => self
                    .per_file_ignores
                    .push((pattern.clone(), selectors.clone())),
            }
        }
        for (name, message) in &extra.banned_api {
            self.banned_api.retain(|(known, _)| known != name);
            self.banned_api.push((name.clone(), message.clone()));
        }
        replace(&mut self.settings, &extra.settings);
        for path in &extra.off {
            if !self.off.contains(path) {
                self.off.push(path.clone());
            }
        }
        add(&mut self.convention_off, &extra.convention_off);
    }

    fn read(table: &dyn TableLike) -> Result<Self> {
        let per_file_ignores = optional(table, "per-file-ignores")?
            .map(|patterns| {
                patterns
                    .iter()
                    .map(|(pattern, _)| Ok((pattern.to_owned(), strings(patterns, pattern)?)))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let banned_api = optional(table, "banned-api")?
            .map(|bans| {
                bans.iter()
                    .map(|(name, _)| Ok((name.to_owned(), string(bans, name)?)))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            select: optional_strings(table, "select")?,
            ignore: optional_strings(table, "ignore")?,
            per_file_ignores,
            banned_api,
            settings: optional(table, "settings")?
                .map(settings)
                .transpose()?
                .unwrap_or_default(),
            off: optional_strings(table, "off")?
                .iter()
                .map(|key| key.split('.').map(str::to_owned).collect())
                .collect(),
            convention_off: optional_strings(table, "convention-off")?,
        })
    }
}

/// Adds each item of `from` that `into` lacks.
fn add(into: &mut Vec<String>, from: &[String]) {
    for item in from {
        if !into.contains(item) {
            into.push(item.clone());
        }
    }
}

/// Replaces each setting of `into` that `from` sets, and adds the others.
fn replace(into: &mut Vec<Setting>, from: &[Setting]) {
    for setting in from {
        into.retain(|known| known.path != setting.path);
        into.push(setting.clone());
    }
}

/// Every value below `table`, with its path. A table is a step of the path, so each setting is a
/// value that is no table.
fn settings(table: &dyn TableLike) -> Result<Vec<Setting>> {
    let mut found = Vec::new();
    collect_settings(table, &mut Vec::new(), &mut found);
    ensure!(!found.is_empty(), "settings holds no value");
    Ok(found)
}

fn collect_settings(table: &dyn TableLike, path: &mut Vec<String>, found: &mut Vec<Setting>) {
    for (key, item) in table.iter() {
        path.push(key.to_owned());
        if let Some(child) = item.as_table_like() {
            collect_settings(child, path, found);
        } else if let Some(value) = item.as_value() {
            let mut value = value.clone();
            value.decor_mut().clear();
            found.push(Setting {
                path: path.clone(),
                value,
            });
        }
        path.pop();
    }
}

/// What the profile wants of Ruff, ty and pytest, for every kind or added by one kind.
#[derive(Debug, Default, Clone)]
pub(crate) struct Values {
    pub(crate) ruff: RuffWanted,
    /// ty rules, each with the lowest level it may have.
    pub(crate) ty_rules: Vec<(String, Level)>,
    /// Settings below `[tool.ty]` that hold exactly the profile's value.
    pub(crate) ty_settings: Vec<Setting>,
    /// For pytest 9 and later.
    pub(crate) pytest: PytestWanted,
    /// For pytest before 9.
    pub(crate) pytest_before_nine: PytestWanted,
    /// Plugins in the dependency list that holds pytest.
    pub(crate) plugins: Vec<String>,
}

impl Values {
    /// These values with `extra` added. A ty level or a setting in `extra` replaces the base one.
    fn with(&self, extra: &Self) -> Self {
        let mut values = self.clone();
        values.ruff.with(&extra.ruff);
        for (rule, level) in &extra.ty_rules {
            values.ty_rules.retain(|(name, _)| name != rule);
            values.ty_rules.push((rule.clone(), *level));
        }
        replace(&mut values.ty_settings, &extra.ty_settings);
        for (into, from) in [
            (&mut values.pytest, &extra.pytest),
            (&mut values.pytest_before_nine, &extra.pytest_before_nine),
        ] {
            for (key, value) in &from.settings {
                into.settings.retain(|(name, _)| name != key);
                into.settings.push((key.clone(), value.clone()));
            }
            add(&mut into.addopts, &from.addopts);
        }
        add(&mut values.plugins, &extra.plugins);
        values
    }

    /// The values in `table`, a tool table of `profile.toml` or of a kind. A missing table holds
    /// no value.
    fn read(table: &dyn TableLike) -> Result<Self> {
        let ruff = optional(table, "ruff")?
            .map(RuffWanted::read)
            .transpose()
            .context("ruff")?
            .unwrap_or_default();
        let ty = optional(table, "ty")?;
        let ty_rules = match ty.map(|ty| optional(ty, "rules")).transpose()? {
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
        let ty_settings = match ty.map(|ty| optional(ty, "settings")).transpose()? {
            Some(Some(table)) => settings(table).context("ty")?,
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
            ruff,
            ty_rules,
            ty_settings,
            pytest,
            pytest_before_nine,
            plugins,
        })
    }
}

/// What the profile wants of pytest for one range of pytest releases.
#[derive(Debug, Default, Clone)]
pub(crate) struct PytestWanted {
    /// Settings of the pytest table, each with the value it must hold. A list must hold each of
    /// the profile's items and may hold more.
    pub(crate) settings: Vec<(String, toml_edit::Value)>,
    /// Arguments that `addopts` must hold. A `-p` and its plugin are one item, such as
    /// `-p no:doctest`.
    pub(crate) addopts: Vec<String>,
}

/// What the profile removes from a project. No exception can keep it.
#[derive(Debug, Default)]
pub(crate) struct PytestForbid {
    /// Flags of `addopts` that take a value, removed with the value that follows each.
    pub(crate) addopts: Vec<String>,
    /// Flags of `addopts` that take no value.
    pub(crate) switches: Vec<String>,
    /// Plugins that `-p no:<name>` in `addopts` must not turn off.
    pub(crate) disabled: Vec<String>,
    /// Settings of the pytest table.
    pub(crate) settings: Vec<String>,
    /// Settings of the pytest table that must not be `false`.
    pub(crate) false_settings: Vec<String>,
    /// Distributions in a dependency list.
    pub(crate) plugins: Vec<String>,
    /// The names under which each plugin loads with `-p`: `-p no:<name>` turns off a plugin of
    /// the profile, and `-p <name>` loads a plugin that the profile removes.
    pub(crate) plugin_names: Vec<(String, Vec<String>)>,
}

impl PytestForbid {
    /// What `[pytest.forbid]` and `[pytest.plugin-names]` in `pytest`, the pytest table of
    /// `profile.toml`, remove.
    fn read(pytest: &dyn TableLike) -> Result<Self> {
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
            addopts: strings(forbid, "addopts")?,
            switches: optional_strings(forbid, "switches")?,
            disabled: optional_strings(forbid, "disabled")?,
            settings: strings(forbid, "settings")?,
            false_settings: strings(forbid, "false")?,
            plugins: strings(forbid, "plugins")?,
            plugin_names,
        })
    }

    /// The plugin that the profile removes which `-p <load>` loads, by one of its names or by a
    /// module inside one of them.
    pub(crate) fn loaded_by(&self, load: &str) -> Option<&str> {
        self.plugin_names
            .iter()
            .filter(|(plugin, _)| self.plugins.contains(plugin))
            .find(|(_, names)| {
                names.iter().any(|name| {
                    load == name
                        || load
                            .strip_prefix(name.as_str())
                            .is_some_and(|rest| rest.starts_with('.'))
                })
            })
            .map(|(plugin, _)| plugin.as_str())
    }

    /// The id of the `-p no:<plugin>` that the profile removes, as the catalog names it.
    pub(crate) fn disable_id(plugin: &str) -> String {
        format!("-p no:{plugin}")
    }

    /// Whether the profile removes `id` of pytest without exception.
    pub(crate) fn forbids(&self, id: &str) -> bool {
        [
            &self.addopts,
            &self.switches,
            &self.settings,
            &self.false_settings,
            &self.plugins,
        ]
        .into_iter()
        .flatten()
        .any(|forbidden| forbidden == id)
            || self
                .disabled
                .iter()
                .any(|plugin| Self::disable_id(plugin) == id)
    }
}

#[derive(Debug)]
pub(crate) struct Profile {
    pub(crate) revision: i64,
    /// The tools pinned exactly in `[dependency-groups] dev`, each with its release from
    /// `[pins]`, which `uv.lock` must lock.
    pub(crate) dev_pins: Vec<(String, String)>,
    /// The pins that only a project with shell scripts gets.
    pub(crate) shell_pins: Vec<(String, String)>,
    /// The pins that only a project with notebooks gets.
    pub(crate) notebook_pins: Vec<(String, String)>,
    pub(crate) kinds: Vec<KindSpec>,
    /// The values for every kind.
    pub(crate) base: Values,
    pub(crate) forbid: PytestForbid,
    pub(crate) files: Vec<Owned>,
    pub(crate) packaging: super::packaging::Rules,
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
        let dev_pins = dev_pins(top, pins, "pins")?;
        let shell_pins = dev_pins_of(top, pins, "shell")?;
        let notebook_pins = dev_pins_of(top, pins, "notebooks")?;
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
                    metadata: strings(kind, "metadata")?,
                    extra: Values::read(kind).with_context(|| format!("kinds.{name}"))?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let files = table(top, "files")?
            .iter()
            .map(|(name, item)| owned_file(name, item))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            revision: integer(top, "revision")?,
            dev_pins,
            shell_pins,
            notebook_pins,
            kinds,
            base: Values::read(top)?,
            forbid: PytestForbid::read(table(top, "pytest")?)?,
            files,
            packaging: super::packaging::Rules::read(table(top, "packaging")?)
                .context("packaging")?,
        })
    }

    /// Every value has a `profile` record in the catalog, and every such record a value.
    fn check(&self, catalog: &Catalog) -> Result<()> {
        let all = self
            .kinds
            .iter()
            .fold(self.base.clone(), |values, kind| values.with(&kind.extra));
        Self::check_ruff(catalog, &all.ruff)?;
        for (rule, _) in &all.ty_rules {
            ensure!(
                catalog
                    .ty_rule(rule)
                    .is_some_and(|record| record.key.id == *rule),
                "ty {} has no rule {rule}",
                catalog::TY_RELEASE
            );
        }
        let named: BTreeSet<(Tool, String)> = self.ids(catalog)?.into_iter().collect();
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
                all.plugins.contains(plugin) || self.forbid.plugins.contains(plugin),
                "pytest.plugin-names names {plugin}, which no kind adds and the profile does not remove"
            );
        }
        for plugin in &self.forbid.plugins {
            ensure!(
                self.forbid
                    .plugin_names
                    .iter()
                    .any(|(name, _)| name == plugin),
                "pytest.plugin-names lacks {plugin}, so doctor cannot find a -p that loads it"
            );
        }
        Ok(())
    }

    /// Every selector of the Ruff values names rules of the pinned release, and the Ruff pin is
    /// the release that `required-version` asks for.
    fn check_ruff(catalog: &Catalog, ruff: &RuffWanted) -> Result<()> {
        let selectors = ruff
            .select
            .iter()
            .chain(&ruff.ignore)
            .chain(ruff.per_file_ignores.iter().flat_map(|(_, codes)| codes))
            .chain(&ruff.convention_off);
        for selector in selectors {
            let known = catalog
                .ruff_rules()
                .any(|rule| !rule.is_removed() && super::ruff::selects(selector, rule));
            ensure!(
                known && catalog.redirect(selector) == Some(selector.as_str()),
                "ruff {} has no rule that {selector} selects",
                catalog::RUFF_RELEASE
            );
        }
        if let Some(setting) = ruff
            .settings
            .iter()
            .find(|setting| setting.dotted() == "required-version")
        {
            let wanted = format!("=={}", catalog::RUFF_RELEASE);
            ensure!(
                setting.value.as_str() == Some(wanted.as_str()),
                "ruff.settings.required-version is not {wanted:?}, the pinned release"
            );
        }
        Ok(())
    }

    /// Each value as the tool and id of the catalog record that explains it. A Ruff rule that
    /// the profile's selection reaches is a value, on or off. A setting is explained by the
    /// record of the setting that holds it, such as `lint.flake8-tidy-imports.banned-api`.
    fn ids(&self, catalog: &Catalog) -> Result<Vec<(Tool, String)>> {
        let values = self
            .kinds
            .iter()
            .fold(self.base.clone(), |values, kind| values.with(&kind.extra));
        let mut ids: Vec<(Tool, String)> = values
            .ruff
            .decided(catalog)
            .into_iter()
            .map(|(code, _)| (Tool::Ruff, code))
            .collect();
        let ruff = &values.ruff;
        for (key, present) in [
            ("lint.select", !ruff.select.is_empty()),
            ("lint.ignore", !ruff.ignore.is_empty()),
            ("lint.per-file-ignores", !ruff.per_file_ignores.is_empty()),
            (
                "lint.flake8-tidy-imports.banned-api",
                !ruff.banned_api.is_empty(),
            ),
        ] {
            if present {
                ids.push((Tool::Ruff, key.to_owned()));
            }
        }
        for path in ruff
            .settings
            .iter()
            .map(|setting| setting.path.clone())
            .chain(ruff.off.iter().cloned())
        {
            ids.push((Tool::Ruff, option(catalog, Tool::Ruff, "", &path)?));
        }
        ids.extend(
            values
                .ty_rules
                .iter()
                .map(|(rule, _)| (Tool::Ty, rule.clone())),
        );
        for setting in &values.ty_settings {
            ids.push((
                Tool::Ty,
                option(catalog, Tool::Ty, "tool.ty.", &setting.path)?,
            ));
        }
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
            .chain(forbid.switches.iter().cloned())
            .chain(
                forbid
                    .disabled
                    .iter()
                    .map(|plugin| PytestForbid::disable_id(plugin)),
            )
            .chain(forbid.settings.iter().cloned())
            .chain(
                forbid
                    .false_settings
                    .iter()
                    .map(|key| format!("{key} = false")),
            )
            .chain(forbid.plugins.iter().cloned())
            .map(|id| (Tool::Pytest, id));
        ids.extend(pytest);
        Ok(ids)
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

    /// The tools that a project which tracks `tracked` pins in `[dependency-groups] dev`.
    pub(crate) fn pins(&self, tracked: Tracked) -> Vec<(String, String)> {
        let mut pins = self.dev_pins.clone();
        if tracked.shell {
            pins.extend(self.shell_pins.iter().cloned());
        }
        if tracked.notebooks {
            pins.extend(self.notebook_pins.iter().cloned());
        }
        pins
    }

    /// The release of Tombi that formats `pyproject.toml` after an apply, when the profile owns
    /// `tombi.toml` for the kind `kind`.
    pub(crate) fn formats_toml(&self, kind: Option<&str>) -> Option<String> {
        self.owned(kind)
            .iter()
            .any(|file| file.path == "tombi.toml")
            .then(|| {
                self.dev_pins
                    .iter()
                    .find(|(name, _)| name == "tombi")
                    .map(|(_, release)| release.clone())
            })
            .flatten()
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

/// The id of the catalog record of the setting that holds the value at `path`, below the tool's
/// table: the longest part of the path, from its start, that the catalog records as an option of
/// `tool` once `prefix` is put before it.
pub(crate) fn option(
    catalog: &Catalog,
    tool: Tool,
    prefix: &str,
    path: &[String],
) -> Result<String> {
    (1..=path.len())
        .rev()
        .map(|end| format!("{prefix}{}", path[..end].join(".")))
        .find(|id| {
            catalog
                .get(&Key::pinned(tool, Surface::Project, Kind::Option, id))
                .is_some()
        })
        .with_context(|| {
            format!(
                "{} {} has no setting {prefix}{}",
                tool.name(),
                tool.pinned(),
                path.join(".")
            )
        })
}

/// The tools of `[dev]` `pins`, each with its release from `pins`, the `[pins]` table.
/// The owned file `name` of `[files]`.
fn owned_file(name: &str, item: &Item) -> Result<Owned> {
    let file = item
        .as_table_like()
        .with_context(|| format!("files.{name} is no table"))?;
    let template = string(file, "template")?;
    let text = TEMPLATES
        .iter()
        .find(|(known, _)| *known == template)
        .map(|(_, text)| *text)
        .with_context(|| {
            format!("files.{name} names the template {template}, which src/doctor/profiles/ lacks")
        })?;
    let holds = file
        .get("holds")
        .map(|_| string(file, "holds"))
        .transpose()?;
    if let Some(line) = &holds {
        ensure!(
            text.lines().any(|known| known == line),
            "files.{name} holds a line that its template lacks"
        );
    }
    let insert = match file.get("insert") {
        None => false,
        Some(item) => item
            .as_bool()
            .with_context(|| format!("files.{name}.insert is no boolean"))?,
    };
    ensure!(
        !(insert && holds.is_some()),
        "files.{name} has both holds and insert"
    );
    Ok(Owned {
        name: name.to_owned(),
        path: string(file, "path")?,
        template,
        text,
        holds,
        insert,
        rivals: optional_strings(file, "rivals")?,
        tables: optional_strings(file, "tables")?,
    })
}

/// The tools that `[dev]` names under `key`, each with its release in `[pins]`.
fn dev_pins(top: &dyn TableLike, pins: &dyn TableLike, key: &str) -> Result<Vec<(String, String)>> {
    with_releases(strings(table(top, "dev")?, key).context("dev")?, pins, key)
}

/// The tools that `[dev]` may name under `key`, each with its release in `[pins]`.
fn dev_pins_of(
    top: &dyn TableLike,
    pins: &dyn TableLike,
    key: &str,
) -> Result<Vec<(String, String)>> {
    with_releases(
        optional_strings(table(top, "dev")?, key).context("dev")?,
        pins,
        key,
    )
}

fn with_releases(
    names: Vec<String>,
    pins: &dyn TableLike,
    key: &str,
) -> Result<Vec<(String, String)>> {
    names
        .into_iter()
        .map(|name| {
            let version = string(pins, &name)
                .with_context(|| format!("dev.{key} names {name}, which [pins] lacks"))?;
            Ok((name, version))
        })
        .collect()
}

fn wanted(table: &dyn TableLike) -> Result<PytestWanted> {
    let settings = match optional(table, "settings")? {
        None => Vec::new(),
        Some(settings) => settings
            .iter()
            .map(|(key, item)| {
                let value = item
                    .as_value()
                    .filter(|value| {
                        value.is_bool()
                            || value.is_str()
                            || value
                                .as_array()
                                .is_some_and(|items| items.iter().all(toml_edit::Value::is_str))
                    })
                    .with_context(|| {
                        format!("settings.{key} is no boolean, string or list of strings")
                    })?;
                let mut value = value.clone();
                value.decor_mut().clear();
                Ok((key.to_owned(), value))
            })
            .collect::<Result<Vec<_>>>()?,
    };
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
