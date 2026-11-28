mod harden_python;

use std::collections::{HashMap, HashSet};
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
use crate::severity::Severity;
use crate::{source, syntax};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    SilentFailure,
    WaitsAndRetries,
    UnreliableTests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    SwallowedException,
    RuleIssue,
    SleepOrRetry,
    DisabledTest,
    FlakyTest,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;

    const WORKFLOW: WorkflowId = WorkflowId::Harden;
    const GOALS: &'static [ledger::Goal] = &[ledger::Goal::Design];
    const GROUPS: &'static [Group] = &[
        Group::SilentFailure,
        Group::WaitsAndRetries,
        Group::UnreliableTests,
    ];

    fn group(self) -> Group {
        match self {
            Self::SwallowedException | Self::RuleIssue => Group::SilentFailure,
            Self::SleepOrRetry => Group::WaitsAndRetries,
            Self::DisabledTest | Self::FlakyTest => Group::UnreliableTests,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::SilentFailure => "Silent failure and reliability errors",
            Group::WaitsAndRetries => "Fixed waits and retries",
            Group::UnreliableTests => "Disabled or unreliable tests",
        }
    }
}

impl Kind {
    fn of_rule(rule: &str, severity: Severity) -> Option<Self> {
        match rule {
            "python/test-skip" => Some(Self::DisabledTest),
            "python/test-sleep"
            | "python/test-rerun"
            | "python/test-suppressed-health-check"
            | "python/mock-import" => Some(Self::FlakyTest),
            "python/pass-through-function" | "python/vague-module-name" => None,
            _ if severity == Severity::Error => Some(Self::RuleIssue),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
struct Read {
    files: usize,
}

/// Find silent failures, fixed waits, retries and unreliable tests.
#[derive(Debug, clap::Args)]
pub(crate) struct HardenArgs {
    #[command(flatten)]
    select: Select<Kind>,
    #[arg(long)]
    questions: bool,
}

impl HardenArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let catalog = QuestionCatalog::load(include_str!("../rules/xpd/harden.yml"))?;
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
                format!(
                    "xpd harden: {} in {} files\n",
                    ledger.count(),
                    ledger.read.files
                )
            },
        )
    }
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
    for (path, language) in &listing.files {
        let Some(text) = listing.text(path) else {
            continue;
        };
        let shown = tree.shown(path);
        let named = listing
            .absolute(path)
            .and_then(|path| tree.tracked(&path))
            .unwrap_or_else(|| shown.clone());
        let root = (*language == SupportLang::Python).then(|| syntax::parse(&text));
        let in_test = source::is_test(path);
        let mut seen = HashSet::new();
        let mut emit = |kind, line, key, evidence, pattern, weight| {
            let scope = root
                .as_ref()
                .filter(|_| kind == Kind::FlakyTest && in_test)
                .map_or(line, |root| harden_python::test_scope(root, line));
            if seen.insert((kind, pattern, scope)) {
                findings.push(ledger::LedgerFinding {
                    kind,
                    key,
                    places: vec![FindingPlace::line(&shown, line)],
                    evidence,
                    weight,
                });
            }
        };
        if *language == SupportLang::Python || workspace.catalog.project.is_some() {
            let linted = workspace.analyze(source::File::on_disk(path), &text);
            for found in &linted.found {
                let issue = found.issue();
                let rule = issue.id.to_string();
                let Some(kind) = Kind::of_rule(&rule, issue.severity) else {
                    continue;
                };
                let line = issue.position.map_or(1, |position| position.line);
                let message = issue.message;
                let (key, evidence) = match kind {
                    Kind::RuleIssue => (
                        format!("{named}\n{rule}\n{message}"),
                        format!("`{rule}`: {message}"),
                    ),
                    Kind::FlakyTest => (format!("{named}\n{rule}"), format!("`{rule}`: {message}")),
                    _ => (format!("{named}\n{message}"), message),
                };
                let pattern = harden_python::rule_pattern(&rule)
                    .unwrap_or(&rule)
                    .to_owned();
                emit(kind, line, key, evidence, pattern, 1);
            }
        }
        if let Some(root) = &root {
            for (line, (kind, evidence, weight), pattern) in harden_python::findings(root, path) {
                let key = format!("{named}\n{evidence}");
                emit(kind, line, key, evidence, pattern.to_owned(), weight);
            }
        }
        tracked.insert(shown, named);
    }
    Ok(catalog.modules(
        Read {
            files: tracked.len(),
        },
        findings,
        |path| tracked.get(path).cloned(),
    ))
}
