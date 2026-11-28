//! Unified read-only `xpd audit` command aggregating `xpd check`, static ledger workflows,
//! `--since` regression deltas, and recorded ledger debt without executing project code.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::check::FixMode;
use crate::ledger::{self, AuditSurvey, RecordedDebt, Tree, WorkflowId};
use crate::output::Format;
use crate::report::Issue;
use crate::severity::Severity;

/// Default static workflows run when `--workflows` is not passed.
const DEFAULT_WORKFLOWS: &[WorkflowId] = &[
    WorkflowId::Untangle,
    WorkflowId::Harden,
    WorkflowId::Optimize,
    WorkflowId::Discover,
];

/// Run a unified, read-only audit across `xpd check` and registered static ledger workflows.
///
/// Aggregates linter issue counts, static workflow findings, optional `--since` regression deltas,
/// and recorded ledger debt from the worktree store without executing project code or runtime
/// adapters.
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
    totals: AuditTotals,
    regression: bool,
}

/// Snapshot of `check` issues and all selected workflow surveys at one tree.
struct TreeAudit {
    issues: Vec<IssueKey>,
    severities: SeverityCounts,
    workflows: Vec<(WorkflowId, AuditSurvey)>,
}

/// Stable fingerprint for comparing `xpd check` issues across commits.
type IssueKey = (String, String, String);

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
    let workspace = ledger::rules_workspace(paths, tree)?;
    let check_report = workspace.check(paths, FixMode::Apply(None))?;
    let (issues, severities) = summarize_issues(&check_report.issues, tree);
    let mut surveyed = Vec::with_capacity(workflows.len());
    for &workflow in workflows {
        let result = run_static_workflow(workflow, paths, tree, top, commit)?;
        surveyed.push((workflow, result));
    }
    Ok(TreeAudit {
        issues,
        severities,
        workflows: surveyed,
    })
}

fn summarize_issues(issues: &[Issue], tree: Tree<'_>) -> (Vec<IssueKey>, SeverityCounts) {
    let mut keys = Vec::with_capacity(issues.len());
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
        keys.push((
            tree.shown(&issue.file),
            issue.id.to_string(),
            issue.message.clone(),
        ));
    }
    (keys, counts)
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
    let check = build_check_summary(&current.issues, current.severities, base_issues.as_deref());
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
        totals,
        regression,
    }
}

fn build_check_summary(
    current: &[IssueKey],
    counts: SeverityCounts,
    baseline: Option<&[IssueKey]>,
) -> CheckSummary {
    let (was, fresh, resolved) = match baseline {
        Some(before) => {
            let (added, gone) = diff_issue_multisets(before, current);
            (Some(before.len()), Some(added), Some(gone))
        }
        None => (None, None, None),
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

fn diff_issue_multisets(before: &[IssueKey], current: &[IssueKey]) -> (usize, usize) {
    let mut before_counts: BTreeMap<&IssueKey, usize> = BTreeMap::new();
    for key in before {
        *before_counts.entry(key).or_default() += 1;
    }
    let mut current_counts: BTreeMap<&IssueKey, usize> = BTreeMap::new();
    for key in current {
        *current_counts.entry(key).or_default() += 1;
    }
    let fresh = current_counts
        .iter()
        .map(|(key, &count)| count.saturating_sub(before_counts.get(key).copied().unwrap_or(0)))
        .sum();
    let resolved = before_counts
        .iter()
        .map(|(key, &count)| count.saturating_sub(current_counts.get(key).copied().unwrap_or(0)))
        .sum();
    (fresh, resolved)
}

fn build_workflow_summary(
    workflow: WorkflowId,
    current: &AuditSurvey,
    baseline: Option<&AuditSurvey>,
    debt: Option<RecordedDebt>,
) -> WorkflowSummary {
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for finding in &current.findings {
        *by_kind.entry(finding.kind.clone()).or_default() += 1;
    }
    let (was, fresh, resolved) = match baseline {
        Some(before) => {
            let had: HashSet<&str> = before.findings.iter().map(|f| f.id.as_str()).collect();
            let has: HashSet<&str> = current.findings.iter().map(|f| f.id.as_str()).collect();
            let added = current
                .findings
                .iter()
                .filter(|f| !had.contains(f.id.as_str()))
                .count();
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
