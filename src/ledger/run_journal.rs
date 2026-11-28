//! Journal data types, plan parsing, and stage presentation for recorded ledger runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use super::LedgerKind;
use super::feature_evidence::FeatureEvidence;
use super::orchestration::{FixOutcome, MAX_GATE_ATTEMPTS};
use super::workflow_registry::WorkflowId;

/// One check command recorded at the start of a ledger run.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JournalCheck {
    pub(super) command: String,
    /// Whether it passed when the run started. A check that failed then gates nothing.
    pub(super) passed: bool,
}

/// Measured wall-clock duration of one benchmark point, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) struct Timing {
    pub(super) median_ms: f64,
    pub(super) min_ms: f64,
    pub(super) max_ms: f64,
    pub(super) runs: usize,
}

/// A benchmark command tracked across a recorded ledger run.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JournalBench {
    pub(super) command: String,
    pub(super) runs: usize,
    pub(super) baseline: Timing,
    pub(super) current: Timing,
    /// The class of machine that timed `baseline` and `current`, from `host_fingerprint`.
    /// A gate timed on another class of machine times both commits there, back to back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) host_fingerprint: Option<String>,
}

/// What proves a fix that names no static finding id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Goal {
    /// Must reduce total code lines in the measured paths.
    Shorter,
    /// Must reduce median `--bench` time by at least 2%.
    Faster,
    /// Must reduce both code lines and median `--bench` time.
    Both,
    /// Must address cited review checklist questions and edit at least one owning figure file.
    Design,
}

/// How a checklist question was settled in a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ChecklistVerdict {
    Pass,
    #[serde(rename = "na", alias = "not-applicable")]
    NotApplicable,
    #[serde(rename = "accepted", alias = "exception")]
    Exception,
}

/// A recorded settlement of one per-figure or per-module checklist item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub(super) enum AnswerRecord {
    Fixed { fix: String, commit: String },
    Pass { reason: String },
    NotApplicable { reason: String },
    Exception { reason: String },
}

/// One checklist verdict submitted in a round plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Answered {
    pub(super) item: String,
    pub(super) verdict: ChecklistVerdict,
    pub(super) reason: String,
}

impl Answered {
    /// Converts a submitted plan answer into its stored `(item_id, record)` pair.
    pub(super) fn into_pair(self) -> (String, AnswerRecord) {
        let record = match self.verdict {
            ChecklistVerdict::Pass => AnswerRecord::Pass {
                reason: self.reason,
            },
            ChecklistVerdict::NotApplicable => AnswerRecord::NotApplicable {
                reason: self.reason,
            },
            ChecklistVerdict::Exception => AnswerRecord::Exception {
                reason: self.reason,
            },
        };
        (self.item, record)
    }
}

/// One fix entry recorded inside a round plan.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PlannedFix {
    pub(super) id: String,
    pub(super) title: String,
    /// The ids of the findings that the fix is to remove. May be empty when `goal` or `questions` is set.
    #[serde(default)]
    pub(super) findings: Vec<String>,
    /// The checklist item ids that the fix addresses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) questions: Vec<String>,
    /// Measurable target for a judgment-based fix with no static finding id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) goal: Option<Goal>,
    /// The fix changes what code outside the paths can call, or what a user sees.
    #[serde(default)]
    pub(super) changes_interface: bool,
    #[serde(default)]
    pub(super) approved: bool,
    /// Why each failed gate did not keep the fix.
    #[serde(default)]
    pub(super) attempts: Vec<String>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub(super) outcome: Option<FixOutcome>,
}

/// A finding accepted as intentional with a recorded reason.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct AcceptedFinding {
    pub(super) finding: String,
    pub(super) reason: String,
}

/// JSON payload parsed by `xpd <workflow> plan`.
#[derive(Debug, Deserialize)]
pub(super) struct RoundPlan {
    #[serde(default)]
    pub(super) fixes: Vec<PlannedFix>,
    #[serde(default)]
    pub(super) accepted: Vec<AcceptedFinding>,
    #[serde(default)]
    pub(super) answered: Vec<Answered>,
}

/// The recorded state of one ledger run printed by CLI stages.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct RunJournal {
    /// The commit that the run started from.
    pub(super) base: String,
    /// The commit that the next fix starts from: the base, or the last fix that was kept.
    pub(super) current: String,
    /// The paths that are measured, from the top of the repository.
    pub(super) paths: Vec<PathBuf>,
    pub(super) checks: Vec<JournalCheck>,
    /// Optional representative workload timed at start and at every gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) bench: Option<JournalBench>,
    /// Optional runtime figure adapter recorded at start and run at every stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) adapter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) feature_evidence: Option<FeatureEvidence>,
    /// How many findings the base had.
    pub(super) was: usize,
    /// Total lines of code in `paths` at the start commit.
    #[serde(default)]
    pub(super) was_lines: usize,
    /// Total lines of code in `paths` at `current`.
    #[serde(default)]
    pub(super) current_lines: usize,
    /// The fixes of each round, in the order of the plan.
    pub(super) rounds: Vec<Vec<PlannedFix>>,
    /// The findings that are right as they are, each id with its reason.
    pub(super) accepted: BTreeMap<String, String>,
    /// The per-figure checklist items settled by a kept fix or a plan verdict.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) answered: BTreeMap<String, AnswerRecord>,
    pub(super) finished: bool,
}

/// The stage that comes next in a recorded run, with guidance text.
#[derive(Debug, Serialize)]
pub(super) struct NextStage {
    pub(super) stage: &'static str,
    pub(super) text: String,
}

impl RunJournal {
    /// Iterates over every planned fix across all rounds in order.
    pub(super) fn fixes(&self) -> impl Iterator<Item = &PlannedFix> {
        self.rounds.iter().flatten()
    }

    /// The first fix that is neither kept nor dropped.
    pub(super) fn open(&self) -> Option<&PlannedFix> {
        self.fixes().find(|fix| fix.outcome.is_none())
    }

    /// Mutable reference to the open fix named `id`.
    pub(super) fn fix_mut(&mut self, id: &str) -> Result<&mut PlannedFix> {
        let fix = (self.rounds.iter_mut().flatten())
            .find(|fix| fix.id == id)
            .with_context(|| format!("the plan has no fix named {id}"))?;
        ensure!(fix.outcome.is_none(), "the fix {id} is already settled");
        Ok(fix)
    }

    /// The stage that comes next for `workflow`.
    pub(super) fn next(&self, workflow: WorkflowId) -> NextStage {
        let cli = workflow.as_str();
        if self.finished {
            return NextStage {
                stage: "finished",
                text: format!("The run is finished. `xpd {cli} start` starts another."),
            };
        }
        let Some(fix) = self.open() else {
            return NextStage {
                stage: "plan",
                text: format!(
                    "Read the ledger with `xpd {cli} --format json` and judge each finding \
                     against the code. Settle each checklist item in a design fix or an answered verdict. Group the findings that have one cause under one fix, \
                     and accept a finding that is right as it is with a reason. Write the plan \
                     as JSON, {{\"fixes\": [{{\"id\": \"short-name\", \"title\": \"...\", \
                     \"findings\": [\"<id>\"], \"changes_interface\": false}}], \"accepted\": \
                     [{{\"finding\": \"<id>\", \"reason\": \"...\"}}]}}, with the fixes in the \
                     order to make them, and run `xpd {cli} plan <file>`. When no finding \
                     is worth a fix, run `xpd {cli} finish`."
                ),
            };
        };
        let id = &fix.id;
        if fix.changes_interface && !fix.approved {
            return NextStage {
                stage: "ask",
                text: format!(
                    "The fix {id}, \"{}\", changes what callers or users see. Ask the user. \
                     After a yes run `xpd {cli} approve {id}`, and after a no run \
                     `xpd {cli} drop {id} --reason \"the user said no\"`.",
                    fix.title
                ),
            };
        }
        if fix.attempts.len() >= MAX_GATE_ATTEMPTS {
            return NextStage {
                stage: "drop",
                text: format!(
                    "The fix {id} failed its gate {} times. Restore the files with \
                     `git reset --hard {}` and run `xpd {cli} drop {id} --reason \"...\"`.",
                    fix.attempts.len(),
                    self.current
                ),
            };
        }
        NextStage {
            stage: "fix",
            text: if fix.findings.is_empty() {
                format!(
                    "Make the fix {id}, \"{}\", commit it, and run `xpd {cli} gate {id}`. \
                     The gate keeps the fix when each check passes as it did at the start, \
                     the goal is met, and the count of findings is not higher than at {}.",
                    fix.title,
                    &self.current[..self.current.len().min(12)]
                )
            } else {
                format!(
                    "Make the fix {id}, \"{}\", which removes the findings {}. Keep the behavior \
                     the same, commit it, and run `xpd {cli} gate {id}`. The gate keeps the \
                     fix when each check passes as it did at the start, a finding of the fix is \
                     gone, and the count of findings is not higher than at {}.",
                    fix.title,
                    fix.findings.join(", "),
                    &self.current[..self.current.len().min(12)]
                )
            },
        }
    }

    /// Unique finding ids removed by kept fixes in this run.
    pub(super) fn fixed_count(&self) -> usize {
        self.fixes()
            .filter_map(|fix| match &fix.outcome {
                Some(FixOutcome::Kept { fixed, .. }) => Some(fixed.iter().map(String::as_str)),
                _ => None,
            })
            .flatten()
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Formats the journal summary and next stage in plain text.
    pub(super) fn text(&self, workflow: WorkflowId) -> String {
        let cli = workflow.as_str();
        let mut text = String::new();
        let _ = writeln!(
            text,
            "xpd {cli} run: started at {} with {} findings ({} lines), now at {} ({} lines)",
            &self.base[..self.base.len().min(12)],
            self.was,
            self.was_lines,
            &self.current[..self.current.len().min(12)],
            self.current_lines,
        );
        for check in &self.checks {
            let state = if check.passed {
                "passed"
            } else {
                "failed at the start, so it gates nothing"
            };
            let _ = writeln!(text, "  check: {} ({state})", check.command);
        }
        if let Some(bench) = &self.bench {
            let _ = writeln!(
                text,
                "  bench: {} ({:.2} ms -> {:.2} ms, {} runs)",
                bench.command, bench.baseline.median_ms, bench.current.median_ms, bench.runs
            );
        }
        if let Some(adapter) = &self.adapter {
            let _ = writeln!(text, "  adapter: {adapter}");
        }
        for (round, fixes) in self.rounds.iter().enumerate() {
            let _ = writeln!(text, "  round {}", round + 1);
            for fix in fixes {
                let state = match &fix.outcome {
                    Some(FixOutcome::Kept {
                        fixed,
                        added,
                        lines_before,
                        lines_after,
                        timing_before,
                        timing_after,
                        ..
                    }) => {
                        let line_delta = format_line_delta(*lines_before, *lines_after);
                        let bench_delta = match (timing_before, timing_after) {
                            (Some(before), Some(after)) => {
                                format!(", {}", format_speedup(*before, *after))
                            }
                            _ => String::new(),
                        };
                        format!(
                            "kept, {} fixed, {} added{line_delta}{bench_delta}",
                            fixed.len(),
                            added.len()
                        )
                    }
                    Some(FixOutcome::Dropped { reason }) => format!("dropped: {reason}"),
                    None => format!("open, {} failed gates", fix.attempts.len()),
                };
                let _ = writeln!(text, "    {}  {}  ({state})", fix.id, fix.title);
            }
        }
        let _ = writeln!(
            text,
            "  {} findings fixed, {} accepted with a reason",
            self.fixed_count(),
            self.accepted.len()
        );
        if !self.answered.is_empty() {
            let fixed_items = self
                .answered
                .values()
                .filter(|record| matches!(record, AnswerRecord::Fixed { .. }))
                .count();
            let excepted_items = self
                .answered
                .values()
                .filter(|record| matches!(record, AnswerRecord::Exception { .. }))
                .count();
            let _ = writeln!(
                text,
                "  {} checklist items answered ({fixed_items} fixed, {excepted_items} accepted)",
                self.answered.len()
            );
        }
        let _ = writeln!(text, "Next: {}", self.next(workflow).text);
        text
    }
}

/// Formats a line count change when the two counts differ.
pub(super) fn format_line_delta(before: usize, after: usize) -> String {
    if before == 0 && after == 0 || before == after {
        return String::new();
    }
    if after > before {
        format!(", +{} lines", after - before)
    } else {
        format!(", -{} lines", before - after)
    }
}

/// Formats a benchmark comparison between two median timings.
pub(super) fn format_speedup(before: Timing, after: Timing) -> String {
    if before.median_ms > 0.0 && after.median_ms > 0.0 {
        let ratio = before.median_ms / after.median_ms;
        format!(
            "{:.2} ms -> {:.2} ms ({ratio:.2}x)",
            before.median_ms, after.median_ms
        )
    } else {
        format!("{:.2} ms -> {:.2} ms", before.median_ms, after.median_ms)
    }
}

/// Reads the plan file, or stdin for `-`. `file` is given from the folder `here`.
pub(super) fn plan_text(here: &Path, file: &Path) -> Result<String> {
    if file == Path::new("-") {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        fs::read_to_string(here.join(file))
            .with_context(|| format!("cannot read {}", file.display()))
    }
}

/// Parses and validates a round plan in `text` for `workflow` against the open journal state.
pub(super) fn read_plan<K: LedgerKind>(
    journal: &RunJournal,
    text: &str,
    workflow: WorkflowId,
) -> Result<RoundPlan> {
    ensure!(!journal.finished, "the run is finished");
    if let Some(open) = journal.open() {
        bail!(
            "the fix {} is still open. Gate it or drop it before the next plan",
            open.id
        );
    }
    let cli = workflow.as_str();
    let plan: RoundPlan = serde_json::from_str(text)
        .with_context(|| format!("the plan is not the JSON that `xpd {cli} plan --help` shows"))?;
    let allow_questions_without_design = workflow
        .spec()
        .checklist
        .allows_questions_without_design_goal();
    for fix in &plan.fixes {
        ensure!(
            allow_questions_without_design
                || fix.questions.is_empty()
                || fix.goal == Some(Goal::Design),
            "the fix {} names checklist items, which need the design goal",
            fix.id
        );
        if let Some(goal) = fix.goal {
            ensure!(
                K::GOALS.contains(&goal),
                "xpd {cli} does not accept goal {goal:?}"
            );
        }
    }
    Ok(plan)
}
