//! CLI stage subcommands and stage execution glue for recorded ledger runs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[cfg(feature = "ledger-distributed")]
use anyhow::bail;
use anyhow::{Result, ensure};
use serde::Serialize;

use crate::output::Format;

#[cfg(feature = "ledger-distributed")]
use super::durable::ProviderConfig;
use super::durable::start_command;
use super::durable_store::{
    LatestRun, LocalMeasurements, RunStanding, WAIT, WorktreeRunStore, command_id,
    convert_json_shape, journal_of, migrate_legacy_journal, migrated_path, open_run,
    quiet_runtime_logs, remove_if_present, require_applied_receipt,
};
use super::feature_evidence::FeatureEvidence;
use super::gate_judge::{
    check_gate, measure_finish, measure_gate, report_finish, report_unanswered, report_unexplained,
    unanswered_checklist, unexplained_findings,
};
use super::orchestration::{
    Command as RunCommand, FinishOutput, Receipt, ReceiptOutcome, RunStartInput, ValidatePlanOutput,
};
use super::run_journal::{RunJournal, plan_text, read_plan};
use super::run_probes::{
    Renamed, RunWorkspace, SurveyProbe, SurveyedFindings, build_initial_journal,
    require_locked_feature_head, validate_start_adapter,
};
use super::workflow_registry::{FeatureEvidencePolicy, WorkflowId};
use super::{AdapterPolicy, LedgerKind, SurveyOutput, Tree, at_commit};

/// A stage of a recorded ledger run.
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Action {
    /// Start a run: record the commit it starts from, the paths, line count, checks and optional
    /// benchmark of the project.
    Start {
        /// Files or directories to measure, inside one git repository.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// A command that exits 0 when the project is sound, such as its tests. Repeat the flag
        /// for more than one. Each runs now, and again at every gate.
        #[arg(long = "check", value_name = "COMMAND")]
        checks: Vec<String>,
        /// A command that runs a representative workload to time before and after each fix.
        /// Measured with one warmup run and `--bench-runs` timed runs.
        #[arg(long, value_name = "COMMAND")]
        bench: Option<String>,
        /// How many timed runs each benchmark measurement takes.
        #[arg(long, default_value_t = 5)]
        bench_runs: usize,
        /// A Python adapter file or shell command that emits runtime measurements.
        #[arg(long, value_name = "ADAPTER")]
        adapter: Option<String>,
        /// Target column for data-prep.
        #[arg(long, value_name = "COLUMN")]
        target: Option<String>,
        /// Replace a run that is not finished.
        #[arg(long)]
        restart: bool,
    },
    /// Record the plan of a round: the fixes in order, and the findings that are right as they
    /// are.
    ///
    /// The file holds JSON such as
    /// `{"fixes": [{"id": "one-price-rule", "title": "...", "findings": ["d81b64d1"],
    /// "changes_interface": false}], "accepted": [{"finding": "cc6bc6e7", "reason": "..."}]}`.
    /// A fix with no static finding id may set `"goal": "shorter"`, `"faster"` or `"both"` so
    /// the gate measures line count and `--bench` timing. A design goal names checklist items
    /// in `"questions"`, and `"answered"` records verdicts with reasons.
    Plan {
        /// The JSON file of the plan, or `-` for stdin.
        file: PathBuf,
    },
    /// Record that the user said yes to a fix that changes what callers or users see.
    Approve { fix: String },
    /// Gate a fix that is committed: run the checks, time `--bench` when set, count lines and
    /// compare the findings with the commit before it. Exit 1 when the fix is not kept.
    Gate { fix: String },
    /// Record that a fix was given up, after its files were restored.
    Drop {
        fix: String,
        /// Why the fix was given up.
        #[arg(long)]
        reason: String,
    },
    /// Print the run and the stage that comes next.
    Status,
    /// End the run: compare findings, line count and benchmark timing with the start commit.
    /// Exit 1 while a fix is open or a new finding has no reason.
    Finish,
}

/// Runs one stage of a recorded run for `K::WORKFLOW`, and prints the run with its next stage.
pub(super) fn run_ledger_action<K: LedgerKind, R: Serialize>(
    action: Action,
    format: Format,
    survey: impl Fn(
        &[PathBuf],
        Tree<'_>,
        Option<&Path>,
        &str,
        Option<&str>,
        AdapterPolicy,
    ) -> Result<SurveyOutput<K, R>>,
) -> Result<ExitCode> {
    let workflow = K::WORKFLOW;
    let spec = workflow.spec();
    ensure!(
        spec.adapter.allows_target_flag()
            || !matches!(
                &action,
                Action::Start {
                    target: Some(_),
                    ..
                }
            ),
        "--target is only available for xpd data-prep"
    );
    let here = std::env::current_dir()?;
    let workspace = RunWorkspace::find(workflow)?;
    let starting = matches!(action, Action::Start { .. });
    if let Action::Start { adapter, .. } = &action {
        validate_start_adapter(workflow, &workspace, &here, adapter.as_deref())?;
    }
    let surveyed = |journal: &RunJournal, commit: Option<&str>| -> Result<SurveyOutput<K, R>> {
        let top = Some(workspace.top.as_path());
        let adapter = journal.adapter.as_deref();
        let measured_head = (spec.head.measures_committed_head() && commit.is_none())
            .then(|| workspace.head())
            .transpose()?;
        match commit.or(measured_head.as_deref()) {
            None => {
                let files = Tree {
                    top,
                    shown_from: None,
                };
                survey(
                    &journal.paths,
                    files,
                    top,
                    "HEAD",
                    adapter,
                    AdapterPolicy::Required,
                )
            }
            Some(commit) => {
                at_commit(&journal.paths, &workspace.top, commit, |then, files, at| {
                    survey(then, files, top, at, adapter, AdapterPolicy::Required)
                })?
                .map_or_else(
                    || {
                        anyhow::bail!(
                            "commit {commit} holds none of the paths recorded for this {} run",
                            workflow.as_str()
                        )
                    },
                    Ok,
                )
            }
        }
    };
    let found = |journal: &RunJournal, commit: Option<&str>| -> Result<SurveyedFindings> {
        let output = surveyed(journal, commit)?;
        let checklist = output.checklist_owners();
        if let Some(empty_error) = spec.checklist.empty_inventory_error() {
            ensure!(!starting || !checklist.is_empty(), "{empty_error}");
        }
        let ids = output.findings.into_iter().map(|entry| entry.id).collect();
        let feature_evidence = (spec.feature_evidence == FeatureEvidencePolicy::FoldAblation)
            .then(|| FeatureEvidence::from_read(&serde_json::to_value(output.read)?))
            .transpose()?;
        Ok((ids, checklist, feature_evidence))
    };
    let renamed =
        |journal: &RunJournal, commit: Option<&str>| -> Result<BTreeMap<String, String>> {
            Ok((surveyed(journal, commit)?.findings.into_iter())
                .map(|entry| (entry.legacy_id, entry.id))
                .collect())
        };
    let (journal, kept) =
        execute_durable_stage::<K>(action, &workspace, &here, workflow, &found, &renamed)?;
    match format {
        Format::Json => println!(
            "{}",
            serde_json::json!({
                "run": journal,
                "summary": {
                    "fixed": journal.fixed_count(),
                    "accepted": journal.accepted.len(),
                },
                "next": journal.next(workflow),
            })
        ),
        Format::Text => print!("{}", journal.text(workflow)),
    }
    Ok(if kept {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Runs one stage on the run in the durable store. Returns the run and whether the stage
/// kept what it measured.
fn execute_durable_stage<K: LedgerKind>(
    action: Action,
    workspace: &RunWorkspace,
    here: &Path,
    workflow: WorkflowId,
    found: SurveyProbe<'_>,
    renamed: Renamed<'_>,
) -> Result<(RunJournal, bool)> {
    quiet_runtime_logs();
    let store = WorktreeRunStore::open(workspace, workflow)?;
    let _lock = match action {
        Action::Status if !workspace.journal.is_file() => None,
        Action::Status => Some(store.lock_reader(workspace)?),
        _ => Some(store.lock_writer(workspace)?),
    };
    if !matches!(action, Action::Start { restart: true, .. }) {
        migrate_legacy_journal(&store, workspace, renamed)?;
    }
    let latest = store.latest()?;
    if let Action::Start {
        paths,
        checks,
        bench,
        bench_runs,
        adapter,
        target,
        restart,
    } = action
    {
        let bench = bench.map(|cmd| (cmd, bench_runs));
        let started = StartedArgs {
            paths: &paths,
            checks,
            bench,
            adapter,
            target,
            restart,
        };
        return start_run(&store, workspace, here, latest.as_ref(), started, found);
    }
    let (generation, journal) = open_run(latest.as_ref(), workflow)?;
    let at = RunHandle {
        store: &store,
        workspace,
        generation,
        journal,
    };
    match action {
        Action::Start { .. } => unreachable!("a start returned above"),
        Action::Status => Ok((at.journal, true)),
        Action::Plan { file } => plan_run::<K>(&at, &plan_text(here, &file)?, found),
        Action::Approve { fix } => approve_run(at, fix),
        Action::Drop { fix, reason } => drop_run(at, fix, reason),
        Action::Gate { fix } => gate_run(&at, &fix, found),
        Action::Finish => finish_run(at, found),
    }
}

/// Arguments captured for `start`.
struct StartedArgs<'a> {
    paths: &'a [PathBuf],
    checks: Vec<String>,
    bench: Option<(String, usize)>,
    adapter: Option<String>,
    target: Option<String>,
    restart: bool,
}

/// The open run that a stage works on.
struct RunHandle<'a> {
    store: &'a WorktreeRunStore,
    workspace: &'a RunWorkspace,
    generation: u64,
    journal: RunJournal,
}

impl RunHandle<'_> {
    fn measurements(&self) -> LocalMeasurements {
        self.store.measurements(self.generation)
    }

    fn send(
        &self,
        measurements: LocalMeasurements,
        kind: &str,
        command: RunCommand,
    ) -> Result<(RunJournal, Receipt)> {
        self.store
            .send(self.generation, measurements, kind, command)
    }
}

/// Starts a run at the next generation, after cancelling an open one on `--restart`.
fn start_run(
    store: &WorktreeRunStore,
    workspace: &RunWorkspace,
    here: &Path,
    latest: Option<&LatestRun>,
    started: StartedArgs<'_>,
    found: SurveyProbe<'_>,
) -> Result<(RunJournal, bool)> {
    let cli = store.workflow.as_str();
    let open = latest.and_then(|latest| {
        matches!(latest.standing, RunStanding::Running(_)).then_some(latest.generation)
    });
    ensure!(
        open.is_none() || started.restart,
        "a run is open. Go on with `xpd {cli} status`, or replace it with --restart"
    );
    let journal = build_initial_journal(
        workspace,
        here,
        started.paths,
        started.checks,
        started.bench,
        (started.adapter, started.target),
        found,
    )?;
    #[cfg(feature = "ledger-distributed")]
    if matches!(store.config, ProviderConfig::Postgres { .. })
        && let Some(spec) = &journal.adapter
        && let Some(adapter) = crate::data_prep::adapter_command(spec)?
        && workspace.top.join(&adapter).is_file()
        && crate::git::output(&workspace.top, &["ls-files", "--error-unmatch", &adapter]).is_err()
    {
        bail!("adapter {adapter} is untracked in git; commit it before starting a distributed run");
    }
    if let Some(open) = open {
        store.cancel(open)?;
    }
    let generation = latest.map_or(0, |latest| latest.generation + 1);
    let id = command_id("start");
    let input = RunStartInput {
        command_id: id.clone(),
        command: store.workflow,
        paths: convert_json_shape(&journal.paths)?,
        checks: (journal.checks.iter())
            .map(|check| check.command.clone())
            .collect(),
        bench: (journal.bench.as_ref()).map(|bench| (bench.command.clone(), bench.runs)),
        adapter: journal.adapter.clone(),
        initial_state: None,
    };
    let measurements = LocalMeasurements {
        baseline: Some((id, convert_json_shape(&journal)?)),
        ..store.measurements(generation)
    };
    let (snapshot, receipt) = start_command(
        &store.config,
        &store.instance(generation),
        measurements.registry(),
        &input,
        WAIT,
    )?;
    require_applied_receipt(&receipt)?;
    if started.restart {
        remove_if_present(&workspace.journal)?;
    }
    remove_if_present(&migrated_path(&workspace.journal))?;
    Ok((journal_of(&snapshot.run)?, true))
}

fn plan_run<K: LedgerKind>(
    at: &RunHandle<'_>,
    text: &str,
    found: SurveyProbe<'_>,
) -> Result<(RunJournal, bool)> {
    let plan = read_plan::<K>(&at.journal, text, at.store.workflow)?;
    require_locked_feature_head(&at.journal, at.workspace)?;
    let (known, checklist, feature_evidence) = found(&at.journal, None)?;
    let measurements = LocalMeasurements {
        known: Some(ValidatePlanOutput {
            known,
            checklist: crate::ledger::checklist_files(checklist),
            feature_evidence,
        }),
        ..at.measurements()
    };
    let planned = RunCommand::Plan {
        fixes: convert_json_shape(&plan.fixes)?,
        accepted: convert_json_shape(&plan.accepted)?,
        answered: plan.answered,
    };
    let (journal, receipt) = at.send(measurements, "plan", planned)?;
    require_applied_receipt(&receipt)?;
    Ok((journal, true))
}

fn approve_run(mut at: RunHandle<'_>, fix: String) -> Result<(RunJournal, bool)> {
    at.journal.fix_mut(&fix)?;
    let (journal, receipt) = at.send(at.measurements(), "approve", RunCommand::Approve { fix })?;
    require_applied_receipt(&receipt)?;
    Ok((journal, true))
}

fn drop_run(mut at: RunHandle<'_>, fix: String, reason: String) -> Result<(RunJournal, bool)> {
    let head = at.workspace.head()?;
    let current = &at.journal.current;
    ensure!(
        head == *current,
        "the commit is not {current}, where the run stands. Restore it with `git reset --hard {current}`"
    );
    at.journal.fix_mut(&fix)?;
    let dropped = RunCommand::Drop { fix, reason, head };
    let (journal, receipt) = at.send(at.measurements(), "drop", dropped)?;
    require_applied_receipt(&receipt)?;
    Ok((journal, true))
}

/// Gates the fix `fix`. A gate that the run judged and did not keep returns `false`, and any
/// other rejection is an error.
fn gate_run(at: &RunHandle<'_>, fix: &str, found: SurveyProbe<'_>) -> Result<(RunJournal, bool)> {
    check_gate(&at.journal, fix)?;
    let failed_before = gate_attempts(&at.journal, fix);
    let measured = measure_gate(&at.journal, at.workspace, found, at.store.workflow)?;
    let head = measured.commit.clone();
    let measurements = LocalMeasurements {
        gate: Some((fix.to_owned(), convert_json_shape(&measured)?)),
        ..at.measurements()
    };
    let gated = RunCommand::Gate {
        fix: fix.to_owned(),
        head,
    };
    let (journal, receipt) = at.send(measurements, "gate", gated)?;
    if receipt.outcome == ReceiptOutcome::Applied {
        return Ok((journal, true));
    }
    let reason = receipt.detail.unwrap_or_default();
    ensure!(gate_attempts(&journal, fix) > failed_before, "{reason}");
    eprintln!("xpd: the gate did not keep {fix}. {reason}");
    Ok((journal, false))
}

fn finish_run(at: RunHandle<'_>, found: SurveyProbe<'_>) -> Result<(RunJournal, bool)> {
    ensure!(
        !at.journal.finished,
        "the run is finished. `xpd {} start` starts another.",
        at.store.workflow.as_str()
    );
    if at
        .store
        .workflow
        .spec()
        .checklist
        .soft_fail_open_fix_on_finish()
        && let Some(open) = at.journal.open()
    {
        eprintln!("xpd: the fix {} is still open", open.id);
        return Ok((at.journal, false));
    }
    require_locked_feature_head(&at.journal, at.workspace)?;
    let surveyed = measure_finish(&at.journal, at.workspace, found)?;
    let measurements = LocalMeasurements {
        finish: Some(FinishOutput {
            checklist_requires_na: surveyed
                .checklist_after
                .iter()
                .filter(|(_, owner)| owner.requires_na)
                .map(|(id, _)| id.clone())
                .collect(),
            checklist_after: surveyed.checklist_after.keys().cloned().collect(),
            before: surveyed.before.clone(),
            after: surveyed.after.clone(),
            current_lines: surveyed.current_lines,
            feature_evidence: surveyed.feature_evidence.clone(),
        }),
        ..at.measurements()
    };
    let (journal, receipt) = at.send(measurements, "finish", RunCommand::Finish)?;
    if receipt.outcome == ReceiptOutcome::Applied {
        report_finish(&journal, &surveyed);
        remove_if_present(&migrated_path(&at.workspace.journal))?;
        return Ok((journal, true));
    }
    let unexplained = unexplained_findings(&journal, &surveyed);
    if !unexplained.is_empty() {
        report_unexplained(&unexplained);
        return Ok((journal, false));
    }
    let unanswered = unanswered_checklist(&journal, &surveyed);
    ensure!(
        !unanswered.is_empty(),
        "{}",
        receipt.detail.unwrap_or_default()
    );
    report_unanswered(&unanswered);
    Ok((journal, false))
}

/// How many gates of the fix `id` failed.
fn gate_attempts(journal: &RunJournal, id: &str) -> usize {
    (journal.fixes())
        .find(|fix| fix.id == id)
        .map_or(0, |fix| fix.attempts.len())
}
