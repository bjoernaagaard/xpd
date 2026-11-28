//! Pure gate judgment rules and gate/finish measurement aggregation for recorded ledger runs.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail, ensure};
use serde::Serialize;

use super::ChecklistOwners;
use super::feature_evidence::{FeatureEvidence, plan_invalidation};
use super::run_journal::{AnswerRecord, Goal, RunJournal, Timing, format_speedup};
use super::run_probes::{
    RunWorkspace, SurveyProbe, broken_checks, count_lines, diff_changed_files, measure_bench,
};
use super::workflow_registry::{BenchmarkPolicy, FeatureEvidencePolicy, WorkflowId};

/// Maximum allowed slowdown factor on `--bench` before a gate rejects a fix as a regression.
const MAX_SLOWDOWN: f64 = 1.15;
/// Minimum speedup factor required for a fix whose goal is `faster` or `both`.
const MIN_SPEEDUP: f64 = 0.98;

/// Why a gate cannot judge a fix that cites `findings`, when the commit before the fix lacks
/// any of them. Such an id cannot be gone after the fix either, so without this rule the gate
/// would count it as removed and keep a fix that removed nothing.
pub(super) fn cited_missing(findings: &[String], before: &BTreeSet<String>) -> Option<String> {
    let missing: Vec<&str> = (findings.iter())
        .filter(|id| !before.contains(*id))
        .map(String::as_str)
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "The commit before the fix has no finding {}, so the gate cannot tell whether the fix removed it. Drop the fix, and plan it again with the ids that the ledger prints now.",
            missing.join(", ")
        )
    })
}

/// What a gate found, with no file or command in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Compared {
    pub(super) fixed: Vec<String>,
    pub(super) added: Vec<String>,
}

/// All measurements that [`judge_fix`] evaluates without touching files or commands.
#[derive(Debug)]
pub(super) struct GateEvidence<'a> {
    pub(super) workflow: WorkflowId,
    pub(super) findings: &'a [String],
    pub(super) questions: &'a [String],
    pub(super) goal: Option<Goal>,
    pub(super) before: &'a BTreeSet<String>,
    pub(super) after: &'a BTreeSet<String>,
    pub(super) broken: &'a [String],
    pub(super) lines: (usize, usize),
    pub(super) timing: Option<(Timing, Timing)>,
    pub(super) changed_files: &'a BTreeSet<String>,
    pub(super) checklist_before: &'a BTreeMap<String, String>,
}

/// Verifies that every cited checklist item existed before the fix and, for a design fix,
/// that the commit modified at least one file owning a cited item.
fn check_design_questions(evidence: &GateEvidence<'_>) -> Result<(), String> {
    let missing_questions: Vec<&str> = (evidence.questions.iter())
        .filter(|item| !evidence.checklist_before.contains_key(*item))
        .map(String::as_str)
        .collect();
    if !missing_questions.is_empty() {
        return Err(format!(
            "The commit before the fix has no checklist item {}, so the gate cannot verify it. Drop the fix and plan it again.",
            missing_questions.join(", ")
        ));
    }
    let allows_without_design = evidence
        .workflow
        .spec()
        .checklist
        .allows_questions_without_design_goal();
    if (evidence.findings.is_empty() || !allows_without_design)
        && (!evidence.questions.is_empty() || evidence.goal == Some(Goal::Design))
    {
        if evidence.questions.is_empty() {
            return Err("A design fix must name at least one checklist item.".to_owned());
        }
        let touches_figure = (evidence.questions.iter()).any(|item| {
            evidence
                .checklist_before
                .get(item)
                .is_some_and(|file| evidence.changed_files.contains(file))
        });
        if !touches_figure {
            let owned: BTreeSet<&str> = (evidence.questions.iter())
                .filter_map(|item| evidence.checklist_before.get(item).map(String::as_str))
                .collect();
            return Err(format!(
                "The design fix did not change any file owning its checklist items ({}).",
                owned.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    Ok(())
}

/// Whether a fix is kept. `before` and `after` hold the ids of the findings at the commit
/// before the fix and after it, and `broken` the checks that passed at the start and fail now.
pub(super) fn judge_fix(evidence: &GateEvidence<'_>) -> Result<Compared, String> {
    if !evidence.broken.is_empty() {
        return Err(format!(
            "These checks passed at the start and fail now: {}.",
            evidence.broken.join("; ")
        ));
    }
    if let Some(reason) = cited_missing(evidence.findings, evidence.before) {
        return Err(reason);
    }
    let spec = evidence.workflow.spec();
    let finding_removed = evidence
        .findings
        .iter()
        .any(|id| !evidence.after.contains(id));
    let strict_removal = spec.checklist.strict_finding_removal();
    if !evidence.findings.is_empty()
        && !finding_removed
        && (strict_removal || evidence.goal.is_none())
    {
        return Err("None of the findings the fix was to remove is gone.".to_owned());
    }
    if evidence.after.len() > evidence.before.len() {
        return Err(format!(
            "The count of findings rose from {} to {}. The new ones: {}.",
            evidence.before.len(),
            evidence.after.len(),
            evidence
                .after
                .difference(evidence.before)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    check_design_questions(evidence)?;
    if let Some(goal) = evidence.goal
        && (strict_removal || !finding_removed)
    {
        let (lines_before, lines_after) = evidence.lines;
        if matches!(goal, Goal::Shorter | Goal::Both) && lines_after >= lines_before {
            return Err(format!(
                "The goal is shorter code, but lines went from {lines_before} to {lines_after}."
            ));
        }
        if matches!(goal, Goal::Faster | Goal::Both) {
            let Some((tb, ta)) = evidence.timing else {
                return Err("The goal is faster code, which needs `--bench` on `start`.".to_owned());
            };
            if tb.median_ms > 0.0 && ta.median_ms > tb.median_ms * MIN_SPEEDUP {
                return Err(format!(
                    "The goal is faster code, but benchmark median went from {:.2} ms to {:.2} ms.",
                    tb.median_ms, ta.median_ms
                ));
            }
        }
    }
    if let Some((tb, ta)) = evidence.timing
        && tb.median_ms > 0.0
        && ta.median_ms > tb.median_ms * MAX_SLOWDOWN
        && (spec.benchmark == BenchmarkPolicy::StrictSlowdown
            || (ta.median_ms - tb.median_ms > 5.0 && ta.min_ms > tb.max_ms))
    {
        return Err(format!(
            "Benchmark regressed from {:.2} ms to {:.2} ms ({:.2}x slower).",
            tb.median_ms,
            ta.median_ms,
            ta.median_ms / tb.median_ms
        ));
    }
    Ok(Compared {
        fixed: evidence
            .before
            .difference(evidence.after)
            .cloned()
            .collect(),
        added: evidence
            .after
            .difference(evidence.before)
            .cloned()
            .collect(),
    })
}

/// Fails unless `id` is the open fix, and the user said yes to it when it changes an interface.
pub(super) fn check_gate(journal: &RunJournal, id: &str) -> Result<()> {
    let open = journal.open().map(|open| open.id.clone());
    ensure!(
        open.as_deref() == Some(id),
        "the fix that is open is {}",
        open.as_deref().unwrap_or("none")
    );
    let asked = journal
        .open()
        .is_some_and(|fix| fix.changes_interface && !fix.approved);
    ensure!(
        !asked,
        "the fix {id} changes what callers or users see, and the user has not said yes. Ask the user first"
    );
    Ok(())
}

/// What a gate measures of the commit that is checked out. The fields match `GateOutput`.
#[derive(Debug, Serialize)]
pub(super) struct MeasuredGate {
    pub(super) commit: String,
    /// The checks that passed at the start and fail now, each with the end of its output.
    pub(super) broken: Vec<String>,
    /// The ids of the findings at the commit the run stands at.
    pub(super) before: BTreeSet<String>,
    /// The ids of the findings of today.
    pub(super) after: BTreeSet<String>,
    pub(super) lines_before: usize,
    pub(super) lines_after: usize,
    pub(super) timing: Option<(Timing, Timing)>,
    pub(super) changed_files: BTreeSet<String>,
    pub(super) checklist_before: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) feature_before: Option<FeatureEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) feature_after: Option<FeatureEvidence>,
}

/// Measures the commit that is checked out against the commit the run stands at.
pub(super) fn measure_gate(
    journal: &RunJournal,
    workspace: &RunWorkspace,
    found: SurveyProbe<'_>,
    workflow: WorkflowId,
) -> Result<MeasuredGate> {
    workspace.clean()?;
    let commit = workspace.head()?;
    ensure!(
        commit != journal.current,
        "no commit was made since {}",
        journal.current
    );
    let broken = broken_checks(
        workflow,
        journal
            .checks
            .iter()
            .map(|check| (check.command.as_str(), check.passed)),
    );
    let (before, checklist_before, feature_before) = found(journal, Some(&journal.current))?;
    let (after, checklist_after, feature_after) = found(journal, None)?;
    let changed = diff_changed_files(&workspace.top, &journal.current, &commit)?;
    let lines_before = journal.current_lines;
    let lines_after = count_lines(&journal.paths);
    let timing = if broken.is_empty()
        && let Some(bench) = &journal.bench
    {
        let after_timing = measure_bench(&bench.command, bench.runs)?;
        Some((bench.current, after_timing))
    } else {
        None
    };
    Ok(MeasuredGate {
        commit,
        broken,
        before,
        after,
        lines_before,
        lines_after,
        timing,
        checklist_before: gate_checklist_files(
            workflow,
            checklist_before,
            &checklist_after,
            &changed,
        ),
        changed_files: changed,
        feature_before,
        feature_after,
    })
}

/// Maps each checklist item before a fix to the owning file checked against `changed`.
pub(super) fn gate_checklist_files(
    workflow: WorkflowId,
    before: ChecklistOwners,
    after: &ChecklistOwners,
    changed: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut files = super::checklist_files(before);
    if workflow.spec().feature_evidence == FeatureEvidencePolicy::FoldAblation {
        for (id, file) in &mut files {
            if let Some(owner) = after.get(id).filter(|owner| changed.contains(&owner.file)) {
                file.clone_from(&owner.file);
            }
        }
    }
    files
}

/// What `finish` measures across the start commit and the current tree.
#[derive(Debug)]
pub(super) struct MeasuredFinish {
    /// The ids of the findings at the commit the run started from.
    pub(super) before: BTreeSet<String>,
    /// The ids of the findings of today.
    pub(super) after: BTreeSet<String>,
    /// The lines of code of today in the paths of the run.
    pub(super) current_lines: usize,
    /// The checklist item ids of today.
    pub(super) checklist_after: ChecklistOwners,
    pub(super) feature_evidence: Option<FeatureEvidence>,
    pub(super) stale: BTreeSet<String>,
}

/// Measures the files of today against the commit the run started from.
pub(super) fn measure_finish(
    journal: &RunJournal,
    workspace: &RunWorkspace,
    found: SurveyProbe<'_>,
) -> Result<MeasuredFinish> {
    if let Some(open) = journal.open() {
        bail!("the fix {} is still open", open.id);
    }
    workspace.clean()?;
    let (before, _, _) = found(journal, Some(&journal.base))?;
    let (after, checklist, feature_evidence) = found(journal, None)?;
    let stale = plan_invalidation(
        journal.feature_evidence.as_ref(),
        feature_evidence.as_ref(),
        &[],
    )?;
    Ok(MeasuredFinish {
        before,
        after,
        current_lines: count_lines(&journal.paths),
        checklist_after: checklist,
        feature_evidence,
        stale,
    })
}

/// The findings that are new since the start and have no reason.
pub(super) fn unexplained_findings<'s>(
    journal: &RunJournal,
    surveyed: &'s MeasuredFinish,
) -> Vec<&'s str> {
    (surveyed.after.difference(&surveyed.before))
        .filter(|id| !journal.accepted.contains_key(*id))
        .map(String::as_str)
        .collect()
}

/// The checklist items of today that have no recorded fix or verdict.
pub(super) fn unanswered_checklist<'s>(
    journal: &RunJournal,
    surveyed: &'s MeasuredFinish,
) -> Vec<&'s str> {
    surveyed
        .checklist_after
        .iter()
        .filter(|(item, owner)| {
            match journal
                .answered
                .get(*item)
                .filter(|_| !surveyed.stale.contains(*item))
            {
                None => true,
                Some(AnswerRecord::NotApplicable { .. }) => false,
                Some(_) => owner.requires_na,
            }
        })
        .map(|(item, _)| item.as_str())
        .collect()
}

/// Prints new findings without recorded reasons to stderr.
pub(super) fn report_unexplained(unexplained: &[&str]) {
    eprintln!(
        "xpd: these findings are new since the start and have no reason: {}. Remove them, or accept them in a plan",
        unexplained.join(", ")
    );
}

/// Prints remaining unanswered checklist items to stderr.
pub(super) fn report_unanswered(unanswered: &[&str]) {
    eprintln!(
        "xpd: these checklist items are unanswered: {}. Address them in a fix, or answer them in a plan",
        unanswered.join(", ")
    );
}

/// Prints what a finished run changed, from the start commit to the files of today.
pub(super) fn report_finish(journal: &RunJournal, surveyed: &MeasuredFinish) {
    let MeasuredFinish {
        before,
        after,
        current_lines,
        ..
    } = surveyed;
    let bench_summary = journal.bench.as_ref().map_or_else(String::new, |bench| {
        format!(", bench {}", format_speedup(bench.baseline, bench.current))
    });
    let fixed = before.difference(after).count();
    let added = after.difference(before).count();
    let accepted = after
        .iter()
        .filter(|id| journal.accepted.contains_key(*id))
        .count();
    let unexplained = after.len().saturating_sub(accepted);
    eprintln!(
        "xpd: {} findings at the start, {} now: {fixed} fixed, {added} added, {accepted} accepted, {unexplained} left without a reason; lines {} -> {current_lines}{bench_summary}",
        before.len(),
        after.len(),
        journal.was_lines,
    );
}
