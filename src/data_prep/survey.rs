use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::Value;

use crate::catalog::Filters;
use crate::graph::Graph;
use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, AdapterPolicy, FindingPlace, LedgerFinding, QuestionCatalog, SurveyOutput, Tree,
};
use crate::source;
use crate::style::Style;
use crate::workspace::Workspace;

use super::{CATALOG, Kind, column_adapter};

#[derive(Debug, Serialize)]
pub(super) struct Read {
    pub(super) files: usize,
    graph: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) facts: Option<column_adapter::Facts>,
}

pub(super) fn survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
    spec: Option<&str>,
    policy: AdapterPolicy,
) -> Result<SurveyOutput<Kind, Read>> {
    let listing = Listing::of(paths, tree, top, commit)?;
    let cwd = std::env::current_dir()?;
    let root = tree.top.unwrap_or(&cwd);
    let filters = Filters {
        rules: ["pandas", "polars", "numpy", "sklearn"]
            .map(str::to_owned)
            .to_vec(),
        severity: None,
    };
    let workspace = Workspace::open(root.to_path_buf(), None, &filters)
        .or_else(|_| Workspace::bundled(Style::Default))?;
    let mut findings = Vec::new();
    let mut files = Vec::new();
    for (path, language) in &listing.files {
        if *language != SupportLang::Python {
            continue;
        }
        let Some((text, absolute)) = listing.text(path).zip(listing.absolute(path)) else {
            continue;
        };
        let shown = tree.shown(path);
        let tracked = tree.tracked(&absolute).unwrap_or_else(|| shown.clone());
        files.push(absolute);
        findings.extend(check_file(&workspace, path, &text, &shown, &tracked));
    }

    let directory = if policy == AdapterPolicy::Required || tree.shown_from.is_some() {
        root
    } else {
        &cwd
    };
    let facts = match spec
        .map(|spec| column_adapter::measure(spec, directory))
        .transpose()
    {
        Ok(facts) => facts.flatten(),
        Err(error) if policy == AdapterPolicy::BestEffort => Some(column_adapter::Facts {
            error: Some(error.to_string()),
            ..column_adapter::Facts::default()
        }),
        Err(error) => return Err(error),
    };
    let mut checklist = Vec::new();
    if let Some(facts) = &facts {
        let catalog = QuestionCatalog::load(CATALOG)?;
        let owner = files.first().map_or_else(
            || {
                paths
                    .first()
                    .map_or_else(String::new, |path| tree.shown(path))
            },
            |path| tree.tracked(path).unwrap_or_else(|| tree.shown(path)),
        );
        for column in &facts.columns {
            let is_target = facts.target.as_ref() == Some(&column.name);
            let entity = BTreeSet::from([(column.name.clone(), owner.clone())]);
            checklist.extend(
                catalog
                    .checklist(entity, "column")
                    .into_iter()
                    .filter(|item| match item.question.as_str() {
                        "target-loss" | "target-rows" | "time-split" => is_target,
                        "rare-levels" => column.categorical,
                        "model-family" => {
                            !is_target && matches!(facts.model_family.as_str(), "" | "unknown")
                        }
                        "target-leakage" => !is_target,
                        "zero-inflation" => column.zero_share.is_some_and(|share| share > 0.5),
                        "outlier-meaning" | "sentinel-meaning" => column.numeric != Some(false),
                        "duplicate-keys" => column.key_like || facts.duplicate_rows() || is_target,
                        _ => true,
                    }),
            );
        }
    }
    Ok(SurveyOutput {
        read: Read {
            files: files.len(),
            graph: Graph::read(
                root,
                &files
                    .iter()
                    .map(|file| file.strip_prefix(root).unwrap_or(file).to_path_buf())
                    .collect::<Vec<_>>(),
            )
            .to_json(),
            facts,
        },
        findings: ledger::named(findings),
        checklist,
    })
}

fn check_file(
    workspace: &Workspace,
    path: &Path,
    text: &str,
    shown: &str,
    tracked: &str,
) -> Vec<LedgerFinding<Kind>> {
    let linted = workspace.analyze(source::File::on_disk(path), text);
    let keyed_joins: BTreeSet<_> = linted
        .found
        .iter()
        .filter_map(|report| {
            let issue = report.issue();
            matches!(
                issue.id.name(),
                "join-key-unvalidated" | "polars-join-key-unvalidated"
            )
            .then_some(issue.position)
        })
        .collect();
    linted
        .found
        .iter()
        .filter_map(move |report| {
            let issue = report.issue();
            let rule = &issue.id;
            let (pack, name) = (rule.package(), rule.name());
            if !matches!(pack, "pandas" | "polars" | "numpy" | "sklearn") {
                return None;
            }
            if matches!(
                name,
                "merge-without-validate" | "polars-join-without-validate"
            ) && keyed_joins.contains(&issue.position)
            {
                return None;
            }
            let kind = match name {
                "polars-join-key-unvalidated" => Kind::JoinKeyUnvalidated,
                "polars-silent-dtype-coercion" => Kind::SilentDtypeCoercion,
                _ => Kind::from_str(name, false).ok()?,
            };
            Some(LedgerFinding {
                kind,
                key: format!("{tracked}\n{rule}\n{}", issue.message),
                places: vec![FindingPlace::line(
                    shown,
                    issue.position.map_or(1, |pos| pos.line),
                )],
                evidence: format!("`{rule}` ({}): {}", issue.severity, issue.message),
                weight: 1,
            })
        })
        .collect()
}
