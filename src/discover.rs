//! `xpd discover`: the names, strings and modules that one text search does not find.

mod name_collision;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::Serialize;

use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, FindingPlace, Ledger, QuestionCatalog, Select, SurveyOutput, Tree, WorkflowId,
};
use crate::output::Format;
use crate::source;

use name_collision::{NameCounts, PublicDefinitions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    Names,
    Search,
    Types,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// Two or more modules define the same public name.
    NameCollision,
    GenericVerbName,
    VagueModuleName,
    ModulePrivateName,
    InterpolatedKey,
    ErrorMessagePrefix,
    DeprecatedWithoutDecorator,
    IdWithoutNewtype,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;

    const WORKFLOW: WorkflowId = WorkflowId::Discover;
    const GOALS: &'static [ledger::Goal] = &[ledger::Goal::Design];
    const GROUPS: &'static [Group] = &[Group::Names, Group::Search, Group::Types];

    fn source_rule(self) -> Option<&'static str> {
        match self {
            Self::NameCollision => None,
            Self::GenericVerbName => Some("python/generic-verb-name"),
            Self::VagueModuleName => Some("python/vague-module-name"),
            Self::ModulePrivateName => Some("python/module-private-name"),
            Self::InterpolatedKey => Some("python/interpolated-key"),
            Self::ErrorMessagePrefix => Some("python/error-message-prefix"),
            Self::DeprecatedWithoutDecorator => Some("python/deprecated-without-decorator"),
            Self::IdWithoutNewtype => Some("python/id-without-newtype"),
        }
    }

    fn group(self) -> Group {
        match self {
            Self::NameCollision
            | Self::GenericVerbName
            | Self::VagueModuleName
            | Self::ModulePrivateName => Group::Names,
            Self::InterpolatedKey | Self::ErrorMessagePrefix | Self::DeprecatedWithoutDecorator => {
                Group::Search
            }
            Self::IdWithoutNewtype => Group::Types,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Names => "Names that one search does not find",
            Group::Search => "Strings and dead ends that a search misses",
            Group::Types => "Types that let a caller swap arguments",
        }
    }
}

impl Kind {
    /// The kind of a finding of the rule `rule`, or `None` when the rule is not about search.
    fn of_rule(rule: &str) -> Option<Self> {
        Self::value_variants()
            .iter()
            .copied()
            .find(|kind| ledger::LedgerKind::source_rule(*kind) == Some(rule))
    }
}

/// What a survey read: the files, and how many public names one module alone defines.
#[derive(Debug, Serialize)]
struct Read {
    files: usize,
    #[serde(flatten)]
    names: NameCounts,
}

/// Find names, strings and modules that one text search does not find.
///
/// A name that two modules define, a function named only with a verb, a module named after a
/// role, a name built with an f-string and an error message that starts with a value each make a
/// search land in the wrong place or nowhere. `xpd discover start` records a run that gates each
/// fix on those findings and on the project's checks.
#[derive(Debug, clap::Args)]
pub(crate) struct DiscoverArgs {
    #[command(flatten)]
    select: Select<Kind>,
    #[arg(long)]
    questions: bool,
}

impl DiscoverArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let catalog = QuestionCatalog::load(include_str!("../rules/xpd/discover.yml"))?;
        if self.questions {
            return catalog.print(self.select.format_override().unwrap_or(default));
        }
        self.select.print_with(
            default,
            None,
            |paths, tree, top, commit, _, _| {
                let listing = Listing::of(paths, tree, top, commit)?;
                survey(paths, tree, &listing, &catalog)
            },
            |ledger: &Ledger<Kind, Read>| {
                let names = ledger.read.names;
                format!(
                    "xpd discover: {} in {} files, {} of {} public names defined once\n",
                    ledger.count(),
                    ledger.read.files,
                    names.defined_once,
                    names.public_names
                )
            },
        )
    }
}

/// Runs a static discover survey for `xpd audit`.
pub(crate) fn audit_survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&std::path::Path>,
    commit: &str,
) -> Result<ledger::AuditSurvey> {
    let catalog = QuestionCatalog::load(include_str!("../rules/xpd/discover.yml"))?;
    let listing = Listing::of(paths, tree, top, commit)?;
    Ok(ledger::AuditSurvey::from_output(survey(
        paths, tree, &listing, &catalog,
    )?))
}

fn survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    listing: &Listing,
    catalog: &QuestionCatalog,
) -> Result<SurveyOutput<Kind, Read>> {
    let workspace = ledger::rules_workspace(paths, tree)?;
    let mut findings = Vec::new();
    let mut tracked = HashMap::new();
    let mut definitions = PublicDefinitions::default();
    for (path, language) in &listing.files {
        if *language != SupportLang::Python {
            continue;
        }
        let Some(text) = listing.text(path) else {
            continue;
        };
        let shown = tree.shown(path);
        let named = listing
            .absolute(path)
            .and_then(|path| tree.tracked(&path))
            .unwrap_or_else(|| shown.clone());
        let linted = workspace.analyze(source::File::on_disk(path), &text);
        for found in &linted.found {
            let issue = found.issue();
            let rule = issue.id.to_string();
            let Some(kind) = Kind::of_rule(&rule) else {
                continue;
            };
            let line = issue.position.map_or(1, |position| position.line);
            let code = text.lines().nth(line - 1).unwrap_or_default().trim();
            findings.push(ledger::LedgerFinding {
                kind,
                key: format!("{named}\n{rule}\n{code}"),
                places: vec![FindingPlace::line(&shown, line)],
                evidence: issue.message,
                weight: 1,
            });
        }
        definitions.read(&shown, path, &text);
        tracked.insert(shown, named);
    }
    let (collisions, names) = definitions.collisions();
    findings.extend(collisions);
    Ok(catalog.modules(
        Read {
            files: tracked.len(),
            names,
        },
        findings,
        |path| tracked.get(path).cloned(),
    ))
}
