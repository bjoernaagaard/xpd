use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::{Value, json};

use crate::catalog::Filters;
use crate::chart_review::runtime::invoke_adapter;
use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, AdapterPolicy, Catalog, ChecklistEntry, Finding, Place, SurveyOutput, Tree,
};
use crate::source;
use crate::workspace::Workspace;

use super::{CATALOG, Kind, static_rules};

const RUNNER: &str = include_str!("../../py/series_facts.py");
const PACKS: [&str; 8] = [
    "arch",
    "coreforecast",
    "hierarchicalforecast",
    "linearmodels",
    "mlforecast",
    "statsforecast",
    "statsmodels",
    "utilsforecast",
];

#[derive(Debug, Serialize)]
pub(super) struct Read {
    pub(super) files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) facts: Option<Value>,
    cv_windows: Vec<Value>,
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
    let directory = if policy == AdapterPolicy::Required || tree.shown_from.is_some() {
        root
    } else {
        &cwd
    };
    let facts = match spec.map(|command| measure(command, directory)).transpose() {
        Ok(facts) => facts,
        Err(error) if policy == AdapterPolicy::BestEffort => {
            Some(json!({"error": error.to_string()}))
        }
        Err(error) => return Err(error),
    };
    let workspace = Workspace::open(
        root.to_path_buf(),
        None,
        &Filters {
            rules: PACKS.map(str::to_owned).to_vec(),
            severity: None,
        },
    )?;
    let configs = static_rules::configs()?;
    let demand = if facts
        .as_ref()
        .is_some_and(|facts| rows(facts, "series").any(is_intermittent))
    {
        static_rules::Demand::Intermittent
    } else {
        static_rules::Demand::Unclassified
    };
    let mut findings = Vec::new();
    let mut cv_windows = Vec::new();
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
        findings.extend(check_catalog(&workspace, path, &text, &shown, &tracked));
        findings.extend(static_rules::findings(
            &configs, &text, &shown, &tracked, demand,
        ));
        cv_windows.extend(static_rules::cv_windows(&text).into_iter().map(|mut call| {
            call["file"] = json!(shown);
            call
        }));
        files.push((shown, tracked));
    }
    let (shown, tracked) = files.first().cloned().unwrap_or_else(|| {
        let shown = paths
            .first()
            .map_or_else(|| ".".to_owned(), |path| tree.shown(path));
        (shown.clone(), shown)
    });
    let checklist = if let Some(facts) = &facts {
        let catalog = Catalog::load(CATALOG)?;
        add_runtime_findings(facts, &shown, &tracked, &mut findings);
        build_series_checklist(facts, &catalog, &tracked)
    } else {
        Vec::new()
    };
    Ok(SurveyOutput {
        read: Read {
            files: files.len(),
            facts,
            cv_windows,
        },
        findings: ledger::named(findings),
        checklist,
    })
}

fn rows<'a>(facts: &'a Value, field: &str) -> impl Iterator<Item = &'a Value> {
    facts[field].as_array().into_iter().flatten()
}

fn measure(adapter: &str, directory: &Path) -> Result<Value> {
    let facts = invoke_adapter(adapter, directory, RUNNER)?;
    ensure!(
        facts["series"].is_array(),
        "time series adapter JSON must have a series array"
    );
    let mut ids = BTreeSet::new();
    for series in rows(&facts, "series") {
        ensure!(
            series["unique_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty() && ids.insert(id))
                && series["is_time_series"].is_boolean(),
            "time series adapter entries need a distinct unique_id and a boolean is_time_series"
        );
    }
    Ok(facts)
}

fn is_intermittent(series: &Value) -> bool {
    matches!(
        series["intermittency"]["class"].as_str(),
        Some("intermittent" | "lumpy")
    )
}

fn check_catalog(
    workspace: &Workspace,
    path: &Path,
    text: &str,
    shown: &str,
    tracked: &str,
) -> Vec<Finding<Kind>> {
    workspace
        .analyze(source::File::on_disk(path), text)
        .found
        .iter()
        .filter_map(|report| {
            let issue = report.issue();
            let rule = &issue.id;
            if !PACKS.contains(&rule.package()) {
                return None;
            }
            let kind = Kind::from_str(rule.name(), false).ok()?;
            Some(Finding {
                kind,
                key: format!("{tracked}\n{rule}\n{}", issue.message),
                places: vec![Place::line(
                    shown,
                    issue.position.map_or(1, |position| position.line),
                )],
                evidence: format!("`{rule}` ({}): {}", issue.severity, issue.message),
                weight: 1,
            })
        })
        .collect()
}

fn add_runtime_findings(
    facts: &Value,
    shown: &str,
    tracked: &str,
    findings: &mut Vec<Finding<Kind>>,
) {
    let mut add = |kind, entity, evidence| {
        findings.push(Finding {
            kind,
            key: format!("{tracked}\n{entity}"),
            places: vec![Place::line(shown, 1)],
            evidence,
            weight: 1,
        });
    };
    for series in rows(facts, "series") {
        if series["is_time_series"] == false
            && let Some(id) = series["unique_id"].as_str()
        {
            add(
                Kind::NotATimeSeries,
                id,
                format!(
                    "{id} (warning): stamps or values do not establish a time series. Cadence: {}",
                    series["cadence"]
                ),
            );
        }
    }
    for group in rows(facts, "groups").chain(rows(facts, "clusters")) {
        if group["coherent"] == false
            && let Some(id) = group["entity"].as_str()
        {
            add(
                Kind::IncoherentGroup,
                id,
                format!(
                    "{id} (warning): group members do not share a coherent seasonal profile. Outlying series: {}",
                    group["outliers"]
                ),
            );
        }
    }
}

fn build_series_checklist(facts: &Value, catalog: &Catalog, owner: &str) -> Vec<ChecklistEntry> {
    let mut checklist = Vec::new();
    for series in rows(facts, "series") {
        let Some(id) = series["unique_id"].as_str() else {
            continue;
        };
        let entities = BTreeSet::from([(id.to_owned(), owner.to_owned())]);
        checklist.extend(catalog.checklist(entities, "series").into_iter().filter_map(|mut item| {
            if series["is_time_series"] == false && item.question != "is-it-time" {
                return None;
            }
            match item.question.as_str() {
                "break-cause" if rows(&series["breaks"], "candidates").count() == 0 => return None,
                "metric-fit" if !is_intermittent(series) => {
                    "Check the chronological holdout, availability of the training frame and whether the error measure matches the cost of mistakes.".clone_into(&mut item.look);
                }
                "length-and-history" => {
                    let unsupported = rows(series, "unsupported_periods").any(|_| true);
                    let claimed_too_long = series["cadence"]["n"].as_f64().is_some_and(|n| {
                        rows(series, "claimed_periods").filter_map(Value::as_f64).any(|period| n < 2.0 * period)
                    });
                    if unsupported || claimed_too_long {
                        item.requires_na = true;
                        item.look = format!("{} stamps; declared periods {}. Answer na and explain which unsupported claim was dropped.", series["cadence"]["n"], series["claimed_periods"]);
                    }
                }
                _ => {}
            }
            Some(item)
        }));
    }
    for group in rows(facts, "groups").chain(rows(facts, "clusters")) {
        let Some(id) = group["entity"].as_str() else {
            continue;
        };
        let entities = BTreeSet::from([(id.to_owned(), owner.to_owned())]);
        checklist.extend(catalog.checklist(entities, "group").into_iter().filter_map(
            |mut item| match item.question.as_str() {
                "group-membership" | "strategy" | "derived-knowledge" => {
                    use std::fmt::Write as _;
                    let _ = write!(item.look, " Measured group evidence: {group}");
                    for error in rows(facts, "hierarchy") {
                        if error["max_absolute_error"]
                            .as_f64()
                            .zip(error["threshold"].as_f64())
                            .is_some_and(|(error, threshold)| error > threshold)
                        {
                            let _ = write!(
                                item.look,
                                " Hierarchy parent exceeds its sum tolerance: {error}"
                            );
                        }
                    }
                    Some(item)
                }
                _ => None,
            },
        ));
    }
    checklist
}
