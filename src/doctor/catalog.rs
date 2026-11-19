//! The reviewed decision about every rule and setting of the pinned Ruff and ty releases, as
//! `xpd doctor` and the catalog verifier read them from `decisions.jsonl`.
//!
//! Each line is one record, keyed by tool, surface and identifier. A rule, its key inside a
//! settings table and its historical alias are different records, linked by `canonical`. Every
//! record carries one decision: leave the project's choice alone, inherit the decision of the
//! record it links to, or suggest a value, always or when a named trigger finds evidence.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

/// The packaged decisions, one JSON record per line.
pub(crate) const DECISIONS: &str = include_str!("decisions.jsonl");

/// The releases the decisions were reviewed for.
pub(crate) const RUFF_RELEASE: &str = "0.17.4";
pub(crate) const TY_RELEASE: &str = "0.0.106";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tool {
    Ruff,
    Ty,
}

impl Tool {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ruff => "ruff",
            Self::Ty => "ty",
        }
    }

    pub(crate) fn pinned(self) -> &'static str {
        match self {
            Self::Ruff => RUFF_RELEASE,
            Self::Ty => TY_RELEASE,
        }
    }
}

/// Where a setting is written, or the registry a rule belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Surface {
    Rules,
    Project,
    Editor,
    Cli,
    Env,
}

impl Surface {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Project => "project",
            Self::Editor => "editor",
            Self::Cli => "cli",
            Self::Env => "env",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        [
            Self::Rules,
            Self::Project,
            Self::Editor,
            Self::Cli,
            Self::Env,
        ]
        .into_iter()
        .find(|surface| surface.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// A rule of the release's registry, including Ruff's removed rules.
    Rule,
    /// A rule the live index lists that the pinned release lacks.
    Newer,
    /// A Ruff selector that the release maps to another selector.
    Redirect,
    /// A historical spelling of a ty rule or setting.
    Alias,
    /// A ty rule the release no longer has.
    Removed,
    /// A setting with its own documentation entry.
    Option,
    /// A Ruff table or a nested member that only the schema names.
    Schema,
    /// An old Ruff path from before the `lint` table.
    Legacy,
    /// A rule's key in a ty rules table.
    RuleMap,
    /// A historical rule spelling as a key in a ty rules table.
    RuleMapAlias,
    /// An editor or language server setting.
    Setting,
    /// An environment variable.
    Variable,
}

impl Kind {
    pub(crate) fn name(self) -> String {
        kebab_name(self)
    }

    fn parse(name: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
    }
}

/// The name that serde writes for a unit variant.
fn kebab_name(variant: impl Serialize) -> String {
    serde_json::to_value(variant)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The identity of a record. A rule, its key in a settings table and its historical alias are
/// different records, and so are a removed rule and the redirect of its old selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Key {
    pub(crate) tool: Tool,
    pub(crate) release: String,
    pub(crate) surface: Surface,
    pub(crate) kind: Kind,
    pub(crate) id: String,
}

impl Key {
    /// The key of a record of the pinned release of `tool`.
    pub(crate) fn pinned(tool: Tool, surface: Surface, kind: Kind, id: &str) -> Self {
        Self {
            tool,
            release: tool.pinned().to_owned(),
            surface,
            kind,
            id: id.to_owned(),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            self.tool.name(),
            self.release,
            self.surface.name(),
            self.kind.name(),
            self.id
        )
    }
}

/// What a project must show before doctor suggests a conditional record. A trigger on code is
/// an evidence rule in `evidence.yml` with the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Trigger {
    ExplicitAny,
    LazyImport,
    NumpyImport,
    PandasValues,
    ManualListCopy,
    ManualDictLoop,
    LocalImport,
    SortedExtreme,
    FirstElementSlice,
    LoggingEagerConversion,
    NoqaComment,
    PydanticModel,
    PydanticValidateCall,
    Division,
    OwnedDecorator,
    ConditionalBinding,
    OptionalTruthiness,
    DynamicReturn,
    DynamicAssignment,
    DynamicYield,
    TyIgnoreComment,
    UnannotatedReturn,
    ContainerIsinstance,
    InlineScriptMetadata,
    /// Ruff's preview mode is on and a selector names a group of rules.
    PreviewBroadSelect,
    /// One of the record's `requires` rules runs.
    RuleActive,
    /// The project lists its dependencies in `[project]`.
    DeclaredDependencies,
    /// Ruff runs on this machine.
    RuffAvailable,
    /// Ruff leaves out a Python file that xpd checks.
    RuffSkipsPythonFile,
}

impl Trigger {
    pub(crate) const ALL: [Self; 29] = [
        Self::ExplicitAny,
        Self::LazyImport,
        Self::NumpyImport,
        Self::PandasValues,
        Self::ManualListCopy,
        Self::ManualDictLoop,
        Self::LocalImport,
        Self::SortedExtreme,
        Self::FirstElementSlice,
        Self::LoggingEagerConversion,
        Self::NoqaComment,
        Self::PydanticModel,
        Self::PydanticValidateCall,
        Self::Division,
        Self::OwnedDecorator,
        Self::ConditionalBinding,
        Self::OptionalTruthiness,
        Self::DynamicReturn,
        Self::DynamicAssignment,
        Self::DynamicYield,
        Self::TyIgnoreComment,
        Self::UnannotatedReturn,
        Self::ContainerIsinstance,
        Self::InlineScriptMetadata,
        Self::PreviewBroadSelect,
        Self::RuleActive,
        Self::DeclaredDependencies,
        Self::RuffAvailable,
        Self::RuffSkipsPythonFile,
    ];

    /// The trigger's name in the catalog, which is also its evidence rule's id.
    pub(crate) fn name(self) -> String {
        kebab_name(self)
    }

    /// Whether the trigger is found in Python source, by the evidence rule of its name.
    pub(crate) fn in_code(self) -> bool {
        !matches!(
            self,
            Self::PreviewBroadSelect
                | Self::RuleActive
                | Self::DeclaredDependencies
                | Self::RuffAvailable
                | Self::RuffSkipsPythonFile
        )
    }
}

/// How doctor tells that a setting already has the suggested effect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Satisfied {
    /// The effective value equals the suggested one.
    #[default]
    Equals,
    /// Any non-empty value is a deliberate policy, such as a list of modules for `ban-lazy`.
    NonEmpty,
    /// The effective table or list holds every key of the suggested table.
    Contains,
}

/// A ty diagnostic level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Level {
    Ignore,
    Warn,
    Error,
}

impl Level {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        match text.trim_matches('"') {
            "ignore" => Some(Self::Ignore),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// What a suggestion changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// Add a Ruff rule's code to a selector array, such as `lint.extend-select`.
    Select { key: String, code: String },
    /// Set a ty rule's level in a rules table, such as `rules.disjoint-cast`.
    Level { key: String, level: Level },
    /// Set a setting, as a TOML value.
    Setting {
        key: String,
        value: String,
        satisfied: Satisfied,
    },
    /// Run a command.
    Command(String),
}

/// When a suggestion applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum When {
    /// Recommended for every project where it is not already effective.
    Always,
    /// Shown when the trigger finds evidence, with the condition the developer decides.
    If { trigger: Trigger, condition: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Suggestion {
    pub(crate) target: Target,
    pub(crate) when: When,
    /// Ruff rules of which one must run for the suggestion to have an effect.
    pub(crate) requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Doctor preserves the project's choice and suggests nothing.
    Leave,
    /// The decision of the linked record applies.
    Inherit(Key),
    /// The table that the suggestions of individual rules are written to.
    Destination,
    Suggest(Suggestion),
}

impl Decision {
    /// The decision's word in the inventories.
    pub(crate) fn word(&self) -> &'static str {
        match self {
            Self::Leave => "leave",
            Self::Inherit(_) => "inherit",
            Self::Destination
            | Self::Suggest(Suggestion {
                when: When::Always, ..
            }) => "recommend",
            Self::Suggest(Suggestion {
                when: When::If { .. },
                ..
            }) => "conditional",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub(crate) key: Key,
    /// A Ruff rule's name, beside its code.
    pub(crate) name: Option<String>,
    /// A Ruff rule's selector prefix: the linter's, plus its category for pycodestyle and
    /// Pylint, such as `PTH`, `E` or `PLC`.
    pub(crate) prefix: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) since: Option<String>,
    /// As the inventory writes it: `on / off` for a Ruff rule in the stable and preview defaults,
    /// a level for a ty rule, a TOML value for a setting.
    pub(crate) default: Option<String>,
    pub(crate) canonical: Option<Key>,
    /// The selector a Ruff redirect maps its old selector to. `None` for a redirect whose target
    /// the release rejects too.
    pub(crate) target: Option<String>,
    pub(crate) source: String,
    /// The decision and its reason, as the reviewed inventory states them.
    pub(crate) reason: String,
    pub(crate) decision: Decision,
}

/// Ruff's mode, which decides the default rules and whether preview rules can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Stable,
    Preview,
}

impl Record {
    /// Whether the Ruff rule runs by default in `mode`.
    pub(crate) fn ruff_default(&self, mode: Mode) -> bool {
        let Some(default) = &self.default else {
            return false;
        };
        let index = match mode {
            Mode::Stable => 0,
            Mode::Preview => 1,
        };
        default.split('/').map(str::trim).nth(index) == Some("on")
    }

    pub(crate) fn is_preview(&self) -> bool {
        self.status.as_deref() == Some("preview")
    }

    pub(crate) fn is_removed(&self) -> bool {
        self.status.as_deref() == Some("removed")
    }

    /// The reason without the inventory's Markdown.
    pub(crate) fn plain_reason(&self) -> String {
        self.reason.replace(['*', '`'], "")
    }
}

/// One line of `decisions.jsonl`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    tool: Tool,
    release: String,
    surface: Surface,
    kind: Kind,
    id: String,
    name: Option<String>,
    prefix: Option<String>,
    status: Option<String>,
    since: Option<String>,
    default: Option<String>,
    source: String,
    decision: String,
    reason: String,
    canonical: Option<String>,
    target: Option<String>,
    role: Option<String>,
    destination: Option<String>,
    value: Option<String>,
    trigger: Option<Trigger>,
    condition: Option<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    satisfied: Satisfied,
}

impl Line {
    fn into_record(self) -> Result<Record> {
        let key = Key {
            tool: self.tool,
            release: self.release,
            surface: self.surface,
            kind: self.kind,
            id: self.id,
        };
        ensure!(!self.reason.trim().is_empty(), "{key} has no reason");
        ensure!(!self.source.trim().is_empty(), "{key} has no source");
        ensure!(
            self.target.is_none() || key.kind == Kind::Redirect,
            "{key} names a target but is no redirect"
        );
        let canonical = self
            .canonical
            .as_deref()
            .map(|link| link_key(&key, link))
            .transpose()?;
        let proposal = Proposal {
            destination: self.destination,
            value: self.value,
            trigger: self.trigger,
            condition: self.condition,
            requires: self.requires,
            satisfied: self.satisfied,
        };
        let decision = match (self.decision.as_str(), self.role.as_deref()) {
            ("leave", None) if proposal.is_empty() => Decision::Leave,
            ("inherit", None) if proposal.is_empty() => Decision::Inherit(
                canonical
                    .clone()
                    .with_context(|| format!("{key} inherits from no record"))?,
            ),
            ("recommend", Some("destination")) if proposal.is_empty() => Decision::Destination,
            (word @ ("recommend" | "conditional"), None) => {
                Decision::Suggest(proposal.suggestion(&key, word)?)
            }
            (word, role) => {
                bail!("{key} has the decision {word} with the role {role:?} and {proposal:?}")
            }
        };
        Ok(Record {
            key,
            name: self.name,
            prefix: self.prefix,
            status: self.status,
            since: self.since,
            default: self.default,
            canonical,
            target: self.target,
            source: self.source,
            reason: self.reason,
            decision,
        })
    }
}

/// The key that the link `surface:kind:id` of the record `key` names, within its tool and
/// release.
fn link_key(key: &Key, link: &str) -> Result<Key> {
    let mut parts = link.splitn(3, ':');
    let (Some(surface), Some(kind), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        bail!("{key} links to {link}, which is no surface:kind:id");
    };
    Ok(Key {
        tool: key.tool,
        release: key.release.clone(),
        surface: Surface::parse(surface)
            .with_context(|| format!("{key} links to the unknown surface {surface}"))?,
        kind: Kind::parse(kind)
            .with_context(|| format!("{key} links to the unknown kind {kind}"))?,
        id: id.to_owned(),
    })
}

/// The fields of a line that describe a suggestion.
#[derive(Debug)]
struct Proposal {
    destination: Option<String>,
    value: Option<String>,
    trigger: Option<Trigger>,
    condition: Option<String>,
    requires: Vec<String>,
    satisfied: Satisfied,
}

impl Proposal {
    fn is_empty(&self) -> bool {
        self.destination.is_none()
            && self.value.is_none()
            && self.trigger.is_none()
            && self.condition.is_none()
            && self.requires.is_empty()
    }

    /// The suggestion of a record whose decision is `word`: `recommend` for every project, or
    /// `conditional` on a trigger with a stated condition.
    fn suggestion(self, key: &Key, word: &str) -> Result<Suggestion> {
        let when = match (word, self.trigger, self.condition) {
            ("recommend", None, None) => When::Always,
            ("recommend", _, _) => bail!("{key} is recommended but names a condition"),
            (_, Some(trigger), Some(condition)) if !condition.trim().is_empty() => {
                When::If { trigger, condition }
            }
            (_, None, _) => bail!("{key} is conditional without a trigger"),
            (_, Some(_), _) => bail!("{key} is conditional without a condition"),
        };
        ensure!(
            self.trigger != Some(Trigger::RuleActive) || !self.requires.is_empty(),
            "{key} is triggered by an active rule but requires none"
        );
        let destination = self
            .destination
            .with_context(|| format!("{key} has no destination"))?;
        let value = self.value.with_context(|| format!("{key} has no value"))?;
        let target = match (key.tool, key.surface) {
            (_, Surface::Cli) => Target::Command(value),
            (Tool::Ruff, Surface::Rules) => Target::Select {
                key: destination,
                code: value.trim_matches('"').to_owned(),
            },
            (Tool::Ty, Surface::Rules) => Target::Level {
                key: destination,
                level: Level::parse(&value)
                    .with_context(|| format!("{key} suggests the level {value}"))?,
            },
            _ => Target::Setting {
                key: destination,
                value,
                satisfied: self.satisfied,
            },
        };
        Ok(Suggestion {
            target,
            when,
            requires: self.requires,
        })
    }
}

/// Every record of `text`, in order, duplicates included. A line that is not a valid record is
/// an error that names the line.
pub(crate) fn parse_lines(text: &str) -> Result<Vec<Record>> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str::<Line>(line)
                .map_err(anyhow::Error::from)
                .and_then(Line::into_record)
                .with_context(|| format!("decisions.jsonl line {}", index + 1))
        })
        .collect()
}

/// The records, each once, with every link resolved.
#[derive(Debug)]
pub(crate) struct Catalog {
    records: Vec<Record>,
    index: HashMap<Key, usize>,
}

impl Catalog {
    /// The packaged decisions.
    pub(crate) fn packaged() -> Result<Self> {
        Self::new(parse_lines(DECISIONS)?)
    }

    /// Fails on a duplicate key, a link to no record and an inheritance that does not end in
    /// exactly one decision.
    pub(crate) fn new(records: Vec<Record>) -> Result<Self> {
        let mut index = HashMap::new();
        let mut duplicates = BTreeMap::new();
        for (position, record) in records.iter().enumerate() {
            if index.insert(record.key.clone(), position).is_some() {
                *duplicates.entry(record.key.to_string()).or_insert(1) += 1;
            }
        }
        ensure!(
            duplicates.is_empty(),
            "duplicate records: {}",
            duplicates.into_keys().collect::<Vec<_>>().join(", ")
        );
        let catalog = Self { records, index };
        for record in &catalog.records {
            if let Some(canonical) = &record.canonical {
                ensure!(
                    catalog.get(canonical).is_some(),
                    "{} links to {canonical}, which is no record",
                    record.key
                );
            }
            catalog.resolve(record)?;
        }
        Ok(catalog)
    }

    pub(crate) fn records(&self) -> &[Record] {
        &self.records
    }

    pub(crate) fn get(&self, key: &Key) -> Option<&Record> {
        self.index.get(key).map(|&position| &self.records[position])
    }

    /// The record whose own decision applies to `record`: itself, or the end of its chain of
    /// inheritance.
    pub(crate) fn resolve<'a>(&'a self, record: &'a Record) -> Result<&'a Record> {
        let mut current = record;
        for _ in 0..self.records.len() {
            match &current.decision {
                Decision::Inherit(parent) => {
                    current = self.get(parent).with_context(|| {
                        format!("{} inherits from {parent}, which is no record", record.key)
                    })?;
                }
                _ => return Ok(current),
            }
        }
        bail!("{} inherits in a cycle", record.key)
    }

    /// The records that doctor evaluates, each with its suggestion.
    pub(crate) fn suggestions(&self) -> impl Iterator<Item = (&Record, &Suggestion)> {
        self.records
            .iter()
            .filter_map(|record| match &record.decision {
                Decision::Suggest(suggestion) => Some((record, suggestion)),
                _ => None,
            })
    }

    /// Ruff's rules of the pinned release, removed rules included.
    pub(crate) fn ruff_rules(&self) -> impl Iterator<Item = &Record> {
        self.records.iter().filter(|record| {
            record.key.tool == Tool::Ruff
                && record.key.release == RUFF_RELEASE
                && record.key.kind == Kind::Rule
        })
    }

    /// The selector a Ruff redirect maps `selector` to, or `selector` itself.
    pub(crate) fn redirect<'a>(&'a self, selector: &'a str) -> &'a str {
        let key = Key::pinned(Tool::Ruff, Surface::Rules, Kind::Redirect, selector);
        self.get(&key)
            .and_then(|record| record.target.as_deref())
            .unwrap_or(selector)
    }

    /// The current ty rule that `name` spells, which is `name` itself or a historical alias.
    pub(crate) fn ty_rule<'a>(&'a self, name: &str) -> Option<&'a Record> {
        let rule = Key::pinned(Tool::Ty, Surface::Rules, Kind::Rule, name);
        let alias = Key::pinned(Tool::Ty, Surface::Rules, Kind::Alias, name);
        self.get(&rule).or_else(|| {
            let canonical = self.get(&alias)?.canonical.as_ref()?;
            self.get(canonical)
        })
    }
}
