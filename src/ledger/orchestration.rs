//! The durable form of a ledger run, as a `duroxide` orchestration.
//!
//! [`ledger_run`] is the orchestration. It holds no file, clock or git access of
//! its own. Every measurement comes back as an activity result, every command
//! arrives on the `commands` queue, and the run that a caller sees is the JSON
//! mirror the orchestration writes with `set_custom_status` after each turn.
//! One receipt per command id lands in the instance key value store, outside
//! that mirror, so a caller can tell its own command apart from another
//! caller's turn. Replay rebuilds the same state from the same history, so the
//! collections here are ordered maps and sets only.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use duroxide::{OrchestrationContext, RetryPolicy};

use super::cited_missing;

/// The name the orchestration is registered and started under.
pub(crate) const ORCHESTRATION_NAME: &str = "LedgerRun";
/// The durable queue that every mutating command arrives on, in order.
pub(crate) const COMMAND_QUEUE: &str = "commands";
/// How many times a fix may fail its gate before the next stage is to drop it.
pub(crate) const ATTEMPTS: usize = 2;
/// Maximum allowed slowdown factor on `--bench` before a gate rejects a fix.
pub(crate) const MAX_SLOWDOWN: f64 = 1.15;
/// Minimum speedup factor required for a fix whose goal is `faster` or `both`.
pub(crate) const MIN_SPEEDUP: f64 = 0.98;

/// The activity that measures the commit a run starts from.
pub(crate) const ACT_INIT_BASELINE: &str = "InitBaseline";
/// The activity that surveys the working tree a plan cites findings from.
pub(crate) const ACT_VALIDATE_PLAN: &str = "ValidatePlan";
/// The activity that measures one gated fix.
pub(crate) const ACT_GATE_FIX: &str = "GateFix";
/// The activity that surveys the tree a run finishes on.
pub(crate) const ACT_FINISH_RUN: &str = "FinishRun";

/// The key of the receipt of one command in the instance store.
pub(crate) fn receipt_key(command_id: &str) -> String {
    format!("receipt:{command_id}")
}

/// One check of the project, as the baseline measured it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CheckState {
    pub(crate) command: String,
    /// Whether it passed when the run started. A check that failed then gates nothing.
    pub(crate) passed: bool,
}

/// One benchmark point, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct Timing {
    pub(crate) median_ms: f64,
    pub(crate) min_ms: f64,
    pub(crate) max_ms: f64,
    pub(crate) runs: usize,
}

/// A benchmark command tracked across the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BenchState {
    pub(crate) command: String,
    pub(crate) runs: usize,
    pub(crate) baseline: Timing,
    pub(crate) current: Timing,
    /// The class of machine that timed `baseline` and `current`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) host_fingerprint: Option<String>,
}

/// What proves a fix that names no static finding id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Goal {
    Shorter,
    Faster,
    Both,
}

/// How a fix ended. The shape matches the journal a run prints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub(crate) enum Outcome {
    Kept {
        commit: String,
        fixed: Vec<String>,
        added: Vec<String>,
        #[serde(default)]
        lines_before: usize,
        #[serde(default)]
        lines_after: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timing_before: Option<Timing>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timing_after: Option<Timing>,
    },
    Dropped {
        reason: String,
    },
}

/// One fix of a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FixState {
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) findings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) goal: Option<Goal>,
    #[serde(default)]
    pub(crate) changes_interface: bool,
    #[serde(default)]
    pub(crate) approved: bool,
    #[serde(default)]
    pub(crate) attempts: Vec<String>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub(crate) outcome: Option<Outcome>,
}

/// A finding that is right as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AcceptedState {
    pub(crate) finding: String,
    pub(crate) reason: String,
}

/// The state of a run, rebuilt on replay and mirrored in `custom_status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct JournalState {
    pub(crate) base: String,
    pub(crate) current: String,
    pub(crate) paths: Vec<String>,
    pub(crate) checks: Vec<CheckState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bench: Option<BenchState>,
    pub(crate) was: usize,
    #[serde(default)]
    pub(crate) was_lines: usize,
    #[serde(default)]
    pub(crate) current_lines: usize,
    pub(crate) rounds: Vec<Vec<FixState>>,
    pub(crate) accepted: BTreeMap<String, String>,
    pub(crate) finished: bool,
}

/// The stage that comes next, with what to do in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NextState {
    pub(crate) stage: String,
    pub(crate) text: String,
}

/// What `status` reads: the run and its next stage, as one JSON value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) run: JournalState,
    pub(crate) next: NextState,
}

impl JournalState {
    pub(crate) fn fixes(&self) -> impl Iterator<Item = &FixState> {
        self.rounds.iter().flatten()
    }

    /// The first fix that is neither kept nor dropped.
    pub(crate) fn open(&self) -> Option<&FixState> {
        self.fixes().find(|fix| fix.outcome.is_none())
    }

    fn fix_mut(&mut self, id: &str) -> Result<&mut FixState, String> {
        let fix = (self.rounds.iter_mut().flatten()).find(|fix| fix.id == id);
        let Some(fix) = fix else {
            return Err(format!("the plan has no fix named {id}"));
        };
        if fix.outcome.is_some() {
            return Err(format!("the fix {id} is already settled"));
        }
        Ok(fix)
    }

    /// The stage that comes next. `command` is the command of the ledger.
    pub(crate) fn next(&self, command: &str) -> NextState {
        if self.finished {
            return NextState {
                stage: "finished".to_owned(),
                text: format!("The run is finished. `xpd {command} start` starts another."),
            };
        }
        let Some(fix) = self.open() else {
            return NextState {
                stage: "plan".to_owned(),
                text: format!(
                    "Read the ledger with `xpd {command} --format json` and judge each \
                     finding against the code. Write the plan as JSON and run \
                     `xpd {command} plan <file>`. When no finding is worth a fix, run \
                     `xpd {command} finish`."
                ),
            };
        };
        let id = &fix.id;
        if fix.changes_interface && !fix.approved {
            return NextState {
                stage: "ask".to_owned(),
                text: format!(
                    "The fix {id}, \"{}\", changes what callers or users see. Ask the \
                     user. After a yes run `xpd {command} approve {id}`, and after a no \
                     run `xpd {command} drop {id} --reason \"the user said no\"`.",
                    fix.title
                ),
            };
        }
        if fix.attempts.len() >= ATTEMPTS {
            return NextState {
                stage: "drop".to_owned(),
                text: format!(
                    "The fix {id} failed its gate {} times. Restore the files with \
                     `git reset --hard {}` and run `xpd {command} drop {id} \
                     --reason \"...\"`.",
                    fix.attempts.len(),
                    self.current
                ),
            };
        }
        NextState {
            stage: "fix".to_owned(),
            text: if fix.findings.is_empty() {
                format!(
                    "Make the fix {id}, \"{}\", commit it, and run \
                     `xpd {command} gate {id}`.",
                    fix.title
                )
            } else {
                format!(
                    "Make the fix {id}, \"{}\", which removes the findings {}. Keep the \
                     behavior the same, commit it, and run `xpd {command} gate {id}`.",
                    fix.title,
                    fix.findings.join(", ")
                )
            },
        }
    }

    pub(crate) fn snapshot(&self, command: &str) -> Snapshot {
        Snapshot {
            run: self.clone(),
            next: self.next(command),
        }
    }

    fn snapshot_json(&self, command: &str) -> String {
        serde_json::to_string(&self.snapshot(command))
            .expect("a journal snapshot always serializes")
    }
}

/// What starts a run. The baseline comes from `InitBaseline`, unless
/// `initial_state` carries a migrated journal, which skips that activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RunStartInput {
    /// The command id the start itself is receipted under.
    pub(crate) command_id: String,
    /// The command of the ledger, such as `untangle`.
    pub(crate) command: String,
    pub(crate) paths: Vec<String>,
    pub(crate) checks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bench: Option<(String, usize)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) initial_state: Option<JournalState>,
}

/// What `InitBaseline` measures at the commit a run starts from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BaselineOutput {
    pub(crate) base: String,
    pub(crate) paths: Vec<String>,
    pub(crate) checks: Vec<CheckState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bench: Option<BenchState>,
    pub(crate) was: usize,
    pub(crate) was_lines: usize,
}

/// What `ValidatePlan` returns: the finding ids of the working tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ValidatePlanOutput {
    pub(crate) known: BTreeSet<String>,
}

/// What `GateFix` measures for one fix. The judgment itself stays pure, in
/// [`judge`], so an activity error and a gate rejection never mix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GateOutput {
    pub(crate) commit: String,
    pub(crate) broken: Vec<String>,
    pub(crate) before: BTreeSet<String>,
    pub(crate) after: BTreeSet<String>,
    pub(crate) lines_before: usize,
    pub(crate) lines_after: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) timing: Option<(Timing, Timing)>,
    /// Whether `timing` times both commits back to back, on a class of machine other than
    /// the one that timed the run's benchmark. Such a pair judges the fix, and the run keeps
    /// its own timings, which are on another scale.
    #[serde(default)]
    pub(crate) paired: bool,
}

/// What `FinishRun` surveys at the end of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FinishOutput {
    pub(crate) before: BTreeSet<String>,
    pub(crate) after: BTreeSet<String>,
    pub(crate) current_lines: usize,
}

/// One mutating command on the `commands` queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Command {
    Plan {
        fixes: Vec<FixState>,
        #[serde(default)]
        accepted: Vec<AcceptedState>,
    },
    Approve {
        fix: String,
    },
    Gate {
        fix: String,
        /// The commit the fix was made at. No host path travels with a command.
        head: String,
    },
    Drop {
        fix: String,
        reason: String,
        /// The commit the submitting checkout stands at.
        head: String,
    },
    Finish,
}

/// A command with the id its receipt is filed under.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CommandEnvelope {
    pub(crate) command_id: String,
    pub(crate) command: Command,
}

/// Whether a command was applied or turned away, with the stage after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReceiptOutcome {
    Applied,
    Rejected,
}

/// The receipt of one command, kept in the instance store, never in the mirror.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Receipt {
    pub(crate) command_id: String,
    pub(crate) outcome: ReceiptOutcome,
    pub(crate) stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) detail: Option<String>,
}

impl Receipt {
    fn applied(command_id: &str, stage: &str) -> Self {
        Self {
            command_id: command_id.to_owned(),
            outcome: ReceiptOutcome::Applied,
            stage: stage.to_owned(),
            detail: None,
        }
    }

    fn rejected(command_id: &str, stage: &str, detail: String) -> Self {
        Self {
            command_id: command_id.to_owned(),
            outcome: ReceiptOutcome::Rejected,
            stage: stage.to_owned(),
            detail: Some(detail),
        }
    }
}

/// What a gate found, with no file or command in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compared {
    pub(crate) fixed: Vec<String>,
    pub(crate) added: Vec<String>,
}

/// Whether a fix is kept, with the exact rules of the journal gate.
pub(crate) fn judge(
    fix: &FixState,
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    broken: &[String],
    lines: (usize, usize),
    timing: Option<(Timing, Timing)>,
) -> Result<Compared, String> {
    if !broken.is_empty() {
        return Err(format!(
            "These checks passed at the start and fail now: {}.",
            broken.join("; ")
        ));
    }
    if let Some(reason) = cited_missing(&fix.findings, before) {
        return Err(reason);
    }
    if !fix.findings.is_empty() && !fix.findings.iter().any(|id| !after.contains(id)) {
        return Err("None of the findings the fix was to remove is gone.".to_owned());
    }
    if after.len() > before.len() {
        return Err(format!(
            "The count of findings rose from {} to {}. The new ones: {}.",
            before.len(),
            after.len(),
            after
                .difference(before)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(goal) = fix.goal {
        let (lines_before, lines_after) = lines;
        if matches!(goal, Goal::Shorter | Goal::Both) && lines_after >= lines_before {
            return Err(format!(
                "The goal is shorter code, but lines went from {lines_before} to {lines_after}."
            ));
        }
        if matches!(goal, Goal::Faster | Goal::Both) {
            let Some((tb, ta)) = timing else {
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
    if let Some((tb, ta)) = timing
        && tb.median_ms > 0.0
        && ta.median_ms > tb.median_ms * MAX_SLOWDOWN
        && ta.median_ms - tb.median_ms > 5.0
        && ta.min_ms > tb.max_ms
    {
        return Err(format!(
            "Benchmark regressed from {:.2} ms to {:.2} ms ({:.2}x slower).",
            tb.median_ms,
            ta.median_ms,
            ta.median_ms / tb.median_ms
        ));
    }
    Ok(Compared {
        fixed: before.difference(after).cloned().collect(),
        added: after.difference(before).cloned().collect(),
    })
}

fn publish(ctx: &OrchestrationContext, state: &JournalState, command: &str) {
    ctx.set_custom_status(state.snapshot_json(command));
}

fn receipt(ctx: &OrchestrationContext, receipt: &Receipt) {
    ctx.set_kv_value_typed(receipt_key(&receipt.command_id), receipt);
}

/// Transient activity errors retry here. A gate that rejects a fix is an
/// `Ok` activity result, never an error, so it never reaches this policy.
fn transient() -> RetryPolicy {
    RetryPolicy::new(3)
}

async fn apply_plan(
    ctx: &OrchestrationContext,
    state: &mut JournalState,
    fixes: Vec<FixState>,
    accepted: Vec<AcceptedState>,
) -> Result<(), String> {
    if state.finished {
        return Err("the run is finished".to_owned());
    }
    if let Some(open) = state.open() {
        return Err(format!(
            "the fix {} is still open. Gate it or drop it before the next plan",
            open.id
        ));
    }
    let known: ValidatePlanOutput = ctx
        .schedule_activity_with_retry_typed(ACT_VALIDATE_PLAN, &state.paths, transient())
        .await
        .map_err(|error| format!("the plan survey failed: {error}"))?;
    let mut named: BTreeSet<&str> = state.fixes().map(|fix| fix.id.as_str()).collect();
    for fix in &fixes {
        if !named.insert(fix.id.as_str()) {
            return Err(format!("two fixes are named {}", fix.id));
        }
        if fix.findings.is_empty() && fix.goal.is_none() {
            return Err(format!(
                "the fix {} names no finding and sets no goal",
                fix.id
            ));
        }
        if matches!(fix.goal, Some(Goal::Faster | Goal::Both)) && state.bench.is_none() {
            return Err(format!(
                "the fix {} has a speed goal, which needs `--bench` on `start`",
                fix.id
            ));
        }
        if fix.outcome.is_some() || !fix.attempts.is_empty() || fix.approved {
            return Err(format!(
                "the fix {} holds fields that only xpd writes",
                fix.id
            ));
        }
    }
    let cited = (fixes.iter().flat_map(|fix| &fix.findings))
        .chain(accepted.iter().map(|item| &item.finding));
    for id in cited {
        if !known.known.contains(id) {
            return Err(format!("the ledger of today has no finding {id}"));
        }
    }
    for item in accepted {
        if item.reason.trim().is_empty() {
            return Err(format!(
                "the accepted finding {} has no reason",
                item.finding
            ));
        }
        state.accepted.insert(item.finding, item.reason);
    }
    if !fixes.is_empty() {
        state.rounds.push(fixes);
    }
    Ok(())
}

async fn apply_gate(
    ctx: &OrchestrationContext,
    state: &mut JournalState,
    id: &str,
    head: &str,
) -> Result<(), String> {
    let open = state.open().map(|fix| fix.id.clone());
    if open.as_deref() != Some(id) {
        return Err(format!(
            "the fix that is open is {}",
            open.as_deref().unwrap_or("none")
        ));
    }
    let asked = state
        .open()
        .is_some_and(|fix| fix.changes_interface && !fix.approved);
    if asked {
        return Err(format!(
            "the fix {id} changes what callers or users see, and the user has not said yes"
        ));
    }
    if head == state.current {
        return Err(format!("no commit was made since {}", state.current));
    }
    let input = serde_json::json!({
        "fix": id,
        "current": state.current,
        "head": head,
        "paths": state.paths,
    });
    let output: GateOutput = ctx
        .schedule_activity_with_retry_typed(ACT_GATE_FIX, &input, transient())
        .await
        .map_err(|error| format!("the gate survey failed: {error}"))?;
    if output.commit != head {
        return Err(format!(
            "the gate measured {}, not the submitted {}",
            output.commit, head
        ));
    }
    let judged = {
        let fix = state.fix_mut(id)?;
        judge(
            fix,
            &output.before,
            &output.after,
            &output.broken,
            (output.lines_before, output.lines_after),
            output.timing,
        )
    };
    let fix = state.fix_mut(id)?;
    match judged {
        Ok(compared) => {
            fix.outcome = Some(Outcome::Kept {
                commit: output.commit.clone(),
                fixed: compared.fixed,
                added: compared.added,
                lines_before: output.lines_before,
                lines_after: output.lines_after,
                timing_before: output.timing.map(|(before, _)| before),
                timing_after: output.timing.map(|(_, after)| after),
            });
            state.current = output.commit;
            state.current_lines = output.lines_after;
            if let (Some(bench), Some((_, after)), false) =
                (&mut state.bench, output.timing, output.paired)
            {
                bench.current = after;
            }
            Ok(())
        }
        Err(reason) => {
            fix.attempts.push(reason.clone());
            Err(reason)
        }
    }
}

async fn apply_finish(ctx: &OrchestrationContext, state: &mut JournalState) -> Result<(), String> {
    if let Some(open) = state.open() {
        return Err(format!("the fix {} is still open", open.id));
    }
    let output: FinishOutput = ctx
        .schedule_activity_with_retry_typed(ACT_FINISH_RUN, &state.paths, transient())
        .await
        .map_err(|error| format!("the finish survey failed: {error}"))?;
    let unexplained: Vec<&str> = (output.after.difference(&output.before))
        .filter(|id| !state.accepted.contains_key(*id))
        .map(String::as_str)
        .collect();
    if !unexplained.is_empty() {
        return Err(format!(
            "these findings are new since the start and have no reason: {}",
            unexplained.join(", ")
        ));
    }
    state.current_lines = output.current_lines;
    state.finished = true;
    Ok(())
}

/// The orchestration of one ledger run. It returns the final snapshot JSON,
/// which completes the instance, when a `finish` command ends the run.
#[expect(dead_code, reason = "wired into the ledger in the next slice")]
pub(crate) async fn ledger_run(ctx: OrchestrationContext, input: String) -> Result<String, String> {
    let start: RunStartInput =
        serde_json::from_str(&input).map_err(|error| format!("bad run start: {error}"))?;
    let command = start.command.clone();
    let mut state = if let Some(state) = start.initial_state {
        state
    } else {
        let baseline: BaselineOutput = ctx
            .schedule_activity_with_retry_typed(ACT_INIT_BASELINE, &start, transient())
            .await?;
        JournalState {
            current: baseline.base.clone(),
            base: baseline.base,
            paths: baseline.paths,
            checks: baseline.checks,
            bench: baseline.bench,
            was: baseline.was,
            was_lines: baseline.was_lines,
            current_lines: baseline.was_lines,
            rounds: Vec::new(),
            accepted: BTreeMap::new(),
            finished: false,
        }
    };
    publish(&ctx, &state, &command);
    receipt(
        &ctx,
        &Receipt::applied(&start.command_id, &state.next(&command).stage),
    );
    if state.finished {
        return Ok(state.snapshot_json(&command));
    }
    loop {
        let raw = ctx.dequeue_event(COMMAND_QUEUE).await;
        let envelope: CommandEnvelope = match serde_json::from_str(&raw) {
            Ok(envelope) => envelope,
            Err(error) => {
                ctx.trace_error(format!("a command was not understood: {error}"));
                continue;
            }
        };
        if ctx
            .get_kv_value(&receipt_key(&envelope.command_id))
            .is_some()
        {
            continue;
        }
        let applied = match envelope.command {
            Command::Plan { fixes, accepted } => {
                apply_plan(&ctx, &mut state, fixes, accepted).await
            }
            Command::Approve { fix } => state.fix_mut(&fix).map(|fix| {
                fix.approved = true;
            }),
            Command::Gate { fix, head } => apply_gate(&ctx, &mut state, &fix, &head).await,
            Command::Drop { fix, reason, head } => {
                if head == state.current {
                    state.fix_mut(&fix).map(|fix| {
                        fix.outcome = Some(Outcome::Dropped { reason });
                    })
                } else {
                    Err(format!(
                        "the commit is not {}, where the run stands",
                        state.current
                    ))
                }
            }
            Command::Finish => apply_finish(&ctx, &mut state).await,
        };
        let stage_after = state.next(&command).stage;
        let receipt_value = match applied {
            Ok(()) => Receipt::applied(&envelope.command_id, &stage_after),
            Err(detail) => Receipt::rejected(&envelope.command_id, &stage_after, detail),
        };
        publish(&ctx, &state, &command);
        receipt(&ctx, &receipt_value);
        if state.finished {
            return Ok(state.snapshot_json(&command));
        }
    }
}
