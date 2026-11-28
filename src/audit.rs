//! Unified read-only `xpd audit` command aggregating `xpd check`, static ledger workflows,
//! `--since` regression deltas, and recorded ledger debt without executing project code.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Serialize;

use crate::check::FixMode;
use crate::ledger::{self, AuditSurvey, RecordedDebt, Tree, WorkflowId};
use crate::output::Format;
use crate::report::{Issue, Position};
use crate::severity::Severity;

/// Default static workflows run when `--workflows` is not passed.
const DEFAULT_WORKFLOWS: &[WorkflowId] = &[
    WorkflowId::Untangle,
    WorkflowId::Harden,
    WorkflowId::Optimize,
    WorkflowId::Discover,
];

/// Maximum regression entries printed inline in text mode.
const MAX_TEXT_REGRESSIONS: usize = 20;

/// Run a unified, read-only audit across `xpd check` and registered static ledger workflows.
///
/// Aggregates linter issue counts, static workflow findings, optional `--since` regression deltas
/// with file and line locations, and recorded ledger debt from the worktree store without
/// executing project code or runtime adapters.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd audit
  xpd audit src/ --format json
  xpd audit src/ --since HEAD~1 --fail-on-regression
  xpd audit src/ --workflows untangle,harden,optimize,discover
  xpd audit src/ --workflows all --format json")]
pub(crate) struct AuditArgs {
    /// Files or directories to audit (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// Compare findings against this commit and report new and resolved counts.
    #[arg(long, value_name = "COMMIT")]
    since: Option<String>,
    /// Comma-separated static workflows to run, or `all` for every registered workflow.
    #[arg(long, value_delimiter = ',')]
    workflows: Vec<String>,
    /// Exit 1 when `--since` detects net new issues or findings, or when `check` finds errors.
    #[arg(long)]
    fail_on_regression: bool,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

/// Summary of `xpd check` diagnostics for an audit run.
#[derive(Debug, Serialize)]
struct CheckSummary {
    total: usize,
    errors: usize,
    warnings: usize,
    info: usize,
    hints: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    #[serde(rename = "new", skip_serializing_if = "Option::is_none")]
    fresh: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<usize>,
}

/// Summary of one static ledger workflow inside an audit report.
#[derive(Debug, Serialize)]
struct WorkflowSummary {
    workflow: WorkflowId,
    findings: usize,
    checklist_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    #[serde(rename = "new", skip_serializing_if = "Option::is_none")]
    fresh: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<usize>,
    by_kind: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debt: Option<RecordedDebt>,
}

/// One newly introduced issue or finding (or blocking `check` error without `--since`).
#[derive(Debug, Clone, Serialize)]
struct AuditRegression {
    source: String,
    kind: String,
    place: String,
    message: String,
}

/// Aggregate totals across `check` and all audited workflows.
#[derive(Debug, Serialize)]
struct AuditTotals {
    check_issues: usize,
    workflow_findings: usize,
    checklist_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_check_issues: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_check_issues: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_workflow_findings: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_workflow_findings: Option<usize>,
    accepted_findings: usize,
    accepted_checklist: usize,
    open_fixes: usize,
}

/// Complete JSON report emitted by `xpd audit --format json`.
#[derive(Debug, Serialize)]
struct AuditReport {
    schema_version: u32,
    paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
    check: CheckSummary,
    workflows: Vec<WorkflowSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    regressions: Vec<AuditRegression>,
    totals: AuditTotals,
    regression: bool,
}

/// Snapshot of `check` issues and all selected workflow surveys at one tree.
struct TreeAudit {
    issues: Vec<IssueRecord>,
    severities: SeverityCounts,
    workflows: Vec<(WorkflowId, AuditSurvey)>,
}

/// Stable fingerprint for comparing `xpd check` issues across commits.
type IssueKey = (String, String, String);

/// Captured `xpd check` issue with its fingerprint, severity, and display place.
struct IssueRecord {
    key: IssueKey,
    severity: Severity,
    place: String,
}

/// Severity tally for a single `xpd check` pass.
#[derive(Debug, Default, Clone, Copy)]
struct SeverityCounts {
    total: usize,
    errors: usize,
    warnings: usize,
    info: usize,
    hints: usize,
}

impl AuditArgs {
    /// Runs the unified read-only audit and prints text or JSON output.
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let selected = resolve_workflows(&self.workflows)?;
        let top = ledger::git_repository(&self.paths[0]);
        let current_tree = Tree {
            top: top.as_deref(),
            shown_from: None,
        };
        let current = run_tree_audit(&self.paths, current_tree, top.as_deref(), "HEAD", &selected)?;
        let baseline = match &self.since {
            Some(commit) => {
                let repo = top.as_deref().with_context(|| {
                    format!(
                        "{} is in no git repository, which --since needs",
                        self.paths[0].display()
                    )
                })?;
                let snapshot = ledger::at_commit(&self.paths, repo, commit, |then, tree, hash| {
                    run_tree_audit(then, tree, Some(repo), hash, &selected)
                })?;
                Some((commit.clone(), snapshot))
            }
            None => None,
        };
        let debt = match top.as_deref() {
            Some(repo) => ledger::recorded_ledger_debt(repo, &selected)?,
            None => BTreeMap::new(),
        };
        let report = build_report(&self.paths, current, baseline, debt);
        match self.format.unwrap_or(default) {
            Format::Json => println!("{}", serde_json::to_string(&report)?),
            Format::Text => print!("{}", format_text(&report)),
        }
        if self.fail_on_regression && report.regression {
            Ok(ExitCode::from(1))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn resolve_workflows(requested: &[String]) -> Result<Vec<WorkflowId>> {
    if requested.is_empty() {
        return Ok(DEFAULT_WORKFLOWS.to_vec());
    }
    if requested
        .iter()
        .any(|name| name.trim().eq_ignore_ascii_case("all"))
    {
        return Ok(WorkflowId::ALL.to_vec());
    }
    let mut out = Vec::new();
    for raw in requested {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let workflow: WorkflowId = trimmed.parse()?;
        if !out.contains(&workflow) {
            out.push(workflow);
        }
    }
    if out.is_empty() {
        Ok(DEFAULT_WORKFLOWS.to_vec())
    } else {
        Ok(out)
    }
}

fn run_tree_audit(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
    workflows: &[WorkflowId],
) -> Result<TreeAudit> {
    let (check_res, workflows_res) = rayon::join(
        || {
            let workspace = ledger::rules_workspace(paths, tree)?;
            let check_report = workspace.check(paths, FixMode::Apply(None))?;
            Ok::<_, anyhow::Error>(summarize_issues(&check_report.issues, tree))
        },
        || {
            workflows
                .par_iter()
                .map(|&workflow| {
                    let result = run_static_workflow(workflow, paths, tree, top, commit)?;
                    Ok((workflow, result))
                })
                .collect::<Result<Vec<(WorkflowId, AuditSurvey)>>>()
        },
    );
    let (issues, severities) = check_res?;
    let surveyed = workflows_res?;
    Ok(TreeAudit {
        issues,
        severities,
        workflows: surveyed,
    })
}

fn summarize_issues(issues: &[Issue], tree: Tree<'_>) -> (Vec<IssueRecord>, SeverityCounts) {
    let mut records = Vec::with_capacity(issues.len());
    let mut counts = SeverityCounts {
        total: issues.len(),
        ..SeverityCounts::default()
    };
    for issue in issues {
        match issue.severity {
            Severity::Error => counts.errors += 1,
            Severity::Warning => counts.warnings += 1,
            Severity::Info => counts.info += 1,
            Severity::Hint => counts.hints += 1,
            Severity::Off => {}
        }
        let shown = tree.shown(&issue.file);
        let place = match issue.position {
            Some(Position { line, column }) => format!("{shown}:{line}:{column}"),
            None => shown.clone(),
        };
        records.push(IssueRecord {
            key: (shown, issue.id.to_string(), issue.message.clone()),
            severity: issue.severity,
            place,
        });
    }
    (records, counts)
}

fn run_static_workflow(
    workflow: WorkflowId,
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
) -> Result<AuditSurvey> {
    match workflow {
        WorkflowId::ChartReview => crate::chart_review::audit_survey(paths, tree, top, commit),
        WorkflowId::DataPrep => crate::data_prep::audit_survey(paths, tree, top, commit),
        WorkflowId::Discover => crate::discover::audit_survey(paths, tree, top, commit),
        WorkflowId::FeatureReview => crate::feature_review::audit_survey(paths, tree, top, commit),
        WorkflowId::Harden => crate::harden::audit_survey(paths, tree, top, commit),
        WorkflowId::ModelReview => crate::model_review::audit_survey(paths, tree, top, commit),
        WorkflowId::OptReview => crate::opt_review::audit_survey(paths, tree, top, commit),
        WorkflowId::Optimize => crate::optimize::audit_survey(paths, tree, top, commit),
        WorkflowId::TsReview => crate::ts_review::audit_survey(paths, tree, top, commit),
        WorkflowId::Untangle => crate::untangle::audit_survey(paths, tree, top, commit),
    }
}

fn build_report(
    paths: &[PathBuf],
    current: TreeAudit,
    baseline: Option<(String, Option<TreeAudit>)>,
    mut debt: BTreeMap<WorkflowId, RecordedDebt>,
) -> AuditReport {
    let (since, base_issues, mut base_workflows) = match baseline {
        Some((commit, Some(base))) => (
            Some(commit),
            Some(base.issues),
            Some(
                base.workflows
                    .into_iter()
                    .collect::<BTreeMap<WorkflowId, AuditSurvey>>(),
            ),
        ),
        Some((commit, None)) => (Some(commit), Some(Vec::new()), Some(BTreeMap::new())),
        None => (None, None, None),
    };
    let mut regressions = Vec::new();
    let check = build_check_summary(
        &current.issues,
        current.severities,
        base_issues.as_deref(),
        &mut regressions,
    );
    let mut workflows = Vec::with_capacity(current.workflows.len());
    for (workflow, survey) in current.workflows {
        let base_survey = base_workflows
            .as_mut()
            .map(|map| map.remove(&workflow).unwrap_or_default());
        workflows.push(build_workflow_summary(
            workflow,
            &survey,
            base_survey.as_ref(),
            debt.remove(&workflow),
            &mut regressions,
        ));
    }
    let totals = aggregate_totals(&check, &workflows, since.as_deref());
    let regression = if since.is_some() {
        totals.new_check_issues.unwrap_or(0) > 0 || totals.new_workflow_findings.unwrap_or(0) > 0
    } else {
        check.errors > 0
    };
    AuditReport {
        schema_version: 1,
        paths: paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        since,
        check,
        workflows,
        regressions,
        totals,
        regression,
    }
}

fn build_check_summary(
    current: &[IssueRecord],
    counts: SeverityCounts,
    baseline: Option<&[IssueRecord]>,
    regressions: &mut Vec<AuditRegression>,
) -> CheckSummary {
    let (was, fresh, resolved) = if let Some(before) = baseline {
        let (added, gone, new_records) = diff_issue_records(before, current);
        for record in new_records {
            regressions.push(AuditRegression {
                source: "check".to_owned(),
                kind: record.key.1.clone(),
                place: record.place.clone(),
                message: record.key.2.clone(),
            });
        }
        (Some(before.len()), Some(added), Some(gone))
    } else {
        for record in current
            .iter()
            .filter(|record| record.severity == Severity::Error)
        {
            regressions.push(AuditRegression {
                source: "check".to_owned(),
                kind: record.key.1.clone(),
                place: record.place.clone(),
                message: record.key.2.clone(),
            });
        }
        (None, None, None)
    };
    CheckSummary {
        total: counts.total,
        errors: counts.errors,
        warnings: counts.warnings,
        info: counts.info,
        hints: counts.hints,
        was,
        fresh,
        resolved,
    }
}

fn diff_issue_records<'a>(
    before: &[IssueRecord],
    current: &'a [IssueRecord],
) -> (usize, usize, Vec<&'a IssueRecord>) {
    let mut remaining_before: BTreeMap<&IssueKey, usize> = BTreeMap::new();
    for record in before {
        *remaining_before.entry(&record.key).or_default() += 1;
    }
    let mut new_records = Vec::new();
    for record in current {
        if let Some(count) = remaining_before.get_mut(&record.key)
            && *count > 0
        {
            *count -= 1;
        } else {
            new_records.push(record);
        }
    }
    let resolved = remaining_before.values().sum();
    (new_records.len(), resolved, new_records)
}

fn build_workflow_summary(
    workflow: WorkflowId,
    current: &AuditSurvey,
    baseline: Option<&AuditSurvey>,
    debt: Option<RecordedDebt>,
    regressions: &mut Vec<AuditRegression>,
) -> WorkflowSummary {
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for finding in &current.findings {
        *by_kind.entry(finding.kind.clone()).or_default() += 1;
    }
    let (was, fresh, resolved) = match baseline {
        Some(before) => {
            let had: HashSet<&str> = before.findings.iter().map(|f| f.id.as_str()).collect();
            let has: HashSet<&str> = current.findings.iter().map(|f| f.id.as_str()).collect();
            let mut added = 0;
            for finding in &current.findings {
                if !had.contains(finding.id.as_str()) {
                    added += 1;
                    regressions.push(AuditRegression {
                        source: workflow.as_str().to_owned(),
                        kind: finding.kind.clone(),
                        place: finding.place.clone(),
                        message: finding.evidence.clone(),
                    });
                }
            }
            let gone = before
                .findings
                .iter()
                .filter(|f| !has.contains(f.id.as_str()))
                .count();
            (Some(before.findings.len()), Some(added), Some(gone))
        }
        None => (None, None, None),
    };
    WorkflowSummary {
        workflow,
        findings: current.findings.len(),
        checklist_items: current.checklist_items,
        was,
        fresh,
        resolved,
        by_kind,
        debt,
    }
}

fn aggregate_totals(
    check: &CheckSummary,
    workflows: &[WorkflowSummary],
    since: Option<&str>,
) -> AuditTotals {
    let workflow_findings = workflows.iter().map(|w| w.findings).sum();
    let checklist_items = workflows.iter().map(|w| w.checklist_items).sum();
    let accepted_findings = workflows
        .iter()
        .filter_map(|w| w.debt.as_ref())
        .map(|d| d.accepted_findings)
        .sum();
    let accepted_checklist = workflows
        .iter()
        .filter_map(|w| w.debt.as_ref())
        .map(|d| d.accepted_checklist)
        .sum();
    let open_fixes = workflows
        .iter()
        .filter_map(|w| w.debt.as_ref())
        .map(|d| d.open_fixes)
        .sum();
    let (new_workflow_findings, resolved_workflow_findings) = if since.is_some() {
        (
            Some(workflows.iter().filter_map(|w| w.fresh).sum()),
            Some(workflows.iter().filter_map(|w| w.resolved).sum()),
        )
    } else {
        (None, None)
    };
    AuditTotals {
        check_issues: check.total,
        workflow_findings,
        checklist_items,
        new_check_issues: check.fresh,
        resolved_check_issues: check.resolved,
        new_workflow_findings,
        resolved_workflow_findings,
        accepted_findings,
        accepted_checklist,
        open_fixes,
    }
}

fn format_text(report: &AuditReport) -> String {
    let mut out = String::new();
    match &report.since {
        Some(commit) => {
            let _ = writeln!(
                out,
                "xpd audit ({}) since {commit}",
                report.paths.join(", ")
            );
        }
        None => {
            let _ = writeln!(out, "xpd audit ({})", report.paths.join(", "));
        }
    }
    let check_delta = format_delta(report.check.was, report.check.fresh, report.check.resolved);
    let _ = writeln!(
        out,
        "  check: {} issues ({} error, {} warning, {} info, {} hint){check_delta}",
        report.check.total,
        report.check.errors,
        report.check.warnings,
        report.check.info,
        report.check.hints
    );
    for workflow in &report.workflows {
        let delta = format_delta(workflow.was, workflow.fresh, workflow.resolved);
        let debt_note = workflow.debt.as_ref().map_or_else(String::new, |debt| {
            format!(
                " [ledger: {}, {} accepted findings, {} checklist exceptions, {} open fixes]",
                debt.status, debt.accepted_findings, debt.accepted_checklist, debt.open_fixes
            )
        });
        let _ = writeln!(
            out,
            "  {}: {} findings, {} checklist items{delta}{debt_note}",
            workflow.workflow.as_str(),
            workflow.findings,
            workflow.checklist_items
        );
    }
    if !report.regressions.is_empty() {
        let heading = if report.since.is_some() {
            "Regressions"
        } else {
            "Blocking errors"
        };
        let _ = writeln!(out, "{heading}: {}", report.regressions.len());
        for item in report.regressions.iter().take(MAX_TEXT_REGRESSIONS) {
            let _ = writeln!(
                out,
                "  [{} / {}] {}  {}",
                item.source, item.kind, item.place, item.message
            );
        }
        if report.regressions.len() > MAX_TEXT_REGRESSIONS {
            let _ = writeln!(
                out,
                "  and {} more (use --format json for the full list)",
                report.regressions.len() - MAX_TEXT_REGRESSIONS
            );
        }
    }
    let totals = &report.totals;
    let _ = writeln!(
        out,
        "Totals: {} check issues, {} workflow findings, {} checklist items, {} accepted debt items",
        totals.check_issues,
        totals.workflow_findings,
        totals.checklist_items,
        totals.accepted_findings + totals.accepted_checklist
    );
    out
}

fn format_delta(was: Option<usize>, fresh: Option<usize>, resolved: Option<usize>) -> String {
    match (was, fresh, resolved) {
        (Some(was), Some(fresh), Some(resolved)) => {
            format!(" (was {was}, +{fresh} new, -{resolved} resolved)")
        }
        _ => String::new(),
    }
}
