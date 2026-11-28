//! Worktree-scoped durable ledger store, legacy JSON journal migration, and stage execution.
//!
//! Each worktree has its own namespace in the shared Duroxide store, and each start of a run takes
//! the next generation of that namespace, so the instance of a run is
//! `wt-<worktree>::<cli_name>::<generation>` and the run that counts is the highest generation.
//! A lock file in the git folder of the worktree makes one process at a time its writer.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, TryLockError};
use std::hash::Hasher as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use duroxide::runtime::registry::ActivityRegistry;
use duroxide::{ActivityContext, OrchestrationStatus};
use serde::Serialize;
use serde::de::DeserializeOwned;
use siphasher::sip::SipHasher13;

use super::durable::{
    LazyTokio, LedgerOrchestratorClient, ProviderConfig, instance_id, open_store, run_command,
    start_command,
};
use super::orchestration::{
    ACT_FINISH_RUN, ACT_GATE_FIX, ACT_INIT_BASELINE, ACT_VALIDATE_PLAN, BaselineOutput,
    Command as RunCommand, CommandEnvelope, FinishOutput, FixOutcome, GateOutput, JournalState,
    Receipt, ReceiptOutcome, RunStartInput, Snapshot, ValidatePlanOutput,
};
use super::run_journal::RunJournal;
use super::run_probes::{Renamed, RunWorkspace};
use super::workflow_registry::WorkflowId;

/// How long a command waits for the receipt of the run. Every measurement is taken before
/// the command is queued, so a turn takes milliseconds, not the time of the checks.
pub(super) const WAIT: Duration = Duration::from_secs(120);
/// The key of the hash that names the import of a JSON journal. It never changes, so a
/// retried import finds the receipt of the first try.
const IMPORT_KEY: [u8; 16] = *b"xpd-ledger-impor";

/// The ledger store of one workflow in one worktree.
pub(super) struct WorktreeRunStore {
    pub(super) config: ProviderConfig,
    namespace: String,
    pub(super) workflow: WorkflowId,
}

/// Where the highest generation of a worktree's run stands.
pub(super) enum RunStanding {
    Running(Option<Snapshot>),
    Completed(Option<Snapshot>),
    Failed(String),
}

/// The highest generation of a worktree's run.
pub(super) struct LatestRun {
    pub(super) generation: u64,
    pub(super) standing: RunStanding,
}

impl WorktreeRunStore {
    pub(super) fn open(workspace: &RunWorkspace, workflow: WorkflowId) -> Result<Self> {
        let git_dir = git_folder(workspace, "--absolute-git-dir")?;
        let common = git_folder(workspace, "--git-common-dir")?;
        let namespace = if git_dir == common {
            "wt-main".to_owned()
        } else {
            let below = git_dir.strip_prefix(&common).unwrap_or(&git_dir);
            format!("wt-{}", below.display())
        };
        Ok(Self {
            config: ProviderConfig::for_repository(&common),
            namespace,
            workflow,
        })
    }

    pub(super) fn instance(&self, generation: u64) -> String {
        instance_id(&self.namespace, self.workflow, generation)
    }

    /// Measurements for the run of `generation`, with none taken yet.
    pub(super) fn measurements(&self, generation: u64) -> LocalMeasurements {
        LocalMeasurements {
            instance: self.instance(generation),
            ..LocalMeasurements::default()
        }
    }

    /// The lock file of this workflow's runs in the worktree, and its path.
    fn lock_file(&self, workspace: &RunWorkspace) -> Result<(File, PathBuf)> {
        let path = workspace.git_path(&format!("xpd-{}.lock", self.workflow.as_str()))?;
        let file = File::options()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("cannot open {}", path.display()))?;
        Ok((file, path))
    }

    /// Takes the lock for a command that changes the run. It fails at once while another
    /// process holds it, so two commands never change one run at the same time.
    pub(super) fn lock_writer(&self, workspace: &RunWorkspace) -> Result<File> {
        let (file, path) = self.lock_file(workspace)?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(TryLockError::WouldBlock) => bail!(
                "another `xpd {}` command is changing the run of this worktree. Run this one when it is done",
                self.workflow.as_str()
            ),
            Err(TryLockError::Error(error)) => {
                Err(error).with_context(|| format!("cannot lock {}", path.display()))
            }
        }
    }

    /// Takes the lock for `status` when it has a JSON journal to import, waiting its turn.
    pub(super) fn lock_reader(&self, workspace: &RunWorkspace) -> Result<File> {
        let (file, path) = self.lock_file(workspace)?;
        file.lock()
            .with_context(|| format!("cannot lock {}", path.display()))?;
        Ok(file)
    }

    /// The highest generation of the run, read through a client alone.
    pub(super) fn latest(&self) -> Result<Option<LatestRun>> {
        let config = self.config.clone();
        let namespace = self.namespace.clone();
        let workflow = self.workflow;
        LazyTokio::new().block_on(async move {
            let store = open_store(&config).await?;
            let client = |generation| {
                LedgerOrchestratorClient::new(
                    Arc::clone(&store),
                    instance_id(&namespace, workflow, generation),
                )
            };
            let mut highest = None;
            for generation in 0_u64.. {
                match client(generation).status().await? {
                    OrchestrationStatus::NotFound => break,
                    status => highest = Some((generation, status)),
                }
            }
            let Some((generation, status)) = highest else {
                return Ok(None);
            };
            let standing = match status {
                OrchestrationStatus::Failed { details, .. } => {
                    RunStanding::Failed(format!("{details:?}"))
                }
                OrchestrationStatus::Completed { .. } => {
                    RunStanding::Completed(client(generation).snapshot().await?)
                }
                _ => RunStanding::Running(client(generation).snapshot().await?),
            };
            Ok::<_, anyhow::Error>(Some(LatestRun {
                generation,
                standing,
            }))
        })?
    }

    /// Whether the run of `generation` filed a receipt for `command_id`.
    fn has_receipt(&self, generation: u64, command_id: &str) -> Result<bool> {
        let config = self.config.clone();
        let instance = self.instance(generation);
        let command_id = command_id.to_owned();
        LazyTokio::new().block_on(async move {
            let store = open_store(&config).await?;
            let client = LedgerOrchestratorClient::new(store, instance);
            Ok::<_, anyhow::Error>(client.receipt(&command_id).await?.is_some())
        })?
    }

    /// Asks the run of `generation` to stop, as a restart does with the run it replaces.
    pub(super) fn cancel(&self, generation: u64) -> Result<()> {
        let config = self.config.clone();
        let instance = self.instance(generation);
        LazyTokio::new().block_on(async move {
            let store = open_store(&config).await?;
            LedgerOrchestratorClient::new(store, instance)
                .cancel("restart")
                .await
        })?
    }

    /// Queues one command behind the commands before it, and waits for its receipt.
    pub(super) fn send(
        &self,
        generation: u64,
        measurements: LocalMeasurements,
        kind: &str,
        command: RunCommand,
    ) -> Result<(RunJournal, Receipt)> {
        let envelope = CommandEnvelope {
            command_id: command_id(kind),
            command,
        };
        let (snapshot, receipt) = run_command(
            &self.config,
            &self.instance(generation),
            measurements.registry(),
            &envelope,
            WAIT,
        )?;
        Ok((journal_of(&snapshot.run)?, receipt))
    }
}

/// A git folder of the worktree, as an absolute path.
fn git_folder(workspace: &RunWorkspace, flag: &str) -> Result<PathBuf> {
    let folder = crate::git::output(&workspace.top, &["rev-parse", flag])?;
    let folder = workspace.top.join(folder.trim());
    fs::canonicalize(&folder).with_context(|| format!("cannot read {}", folder.display()))
}

/// The measurements this process took, which the activities of its own run return.
///
/// The CLI measures before it queues a command, as the journal did, so an activity only
/// hands back what was measured. An activity of another instance, which a runtime in this
/// process can pick up from the shared store, gets an error and never this run's numbers.
#[derive(Debug, Default)]
pub(super) struct LocalMeasurements {
    pub(super) instance: String,
    pub(super) baseline: Option<(String, BaselineOutput)>,
    pub(super) known: Option<ValidatePlanOutput>,
    pub(super) gate: Option<(String, GateOutput)>,
    pub(super) finish: Option<FinishOutput>,
}

impl LocalMeasurements {
    fn serve(&self, ctx: &ActivityContext, input: &str) -> Result<String, String> {
        let name = ctx.activity_name();
        if ctx.instance_id() != self.instance {
            return Err(format!(
                "this xpd process serves {}, not {}. Run the command again in the worktree of that run",
                self.instance,
                ctx.instance_id()
            ));
        }
        let encode = |output: &dyn erased::Encode| output.encode();
        let served = match name {
            ACT_INIT_BASELINE => self
                .baseline
                .as_ref()
                .filter(|(id, _)| {
                    serde_json::from_str::<RunStartInput>(input)
                        .is_ok_and(|start| start.command_id == *id)
                })
                .map(|(_, output)| encode(output)),
            ACT_VALIDATE_PLAN => self.known.as_ref().map(|output| encode(output)),
            ACT_GATE_FIX => self
                .gate
                .as_ref()
                .filter(|(fix, output)| {
                    serde_json::from_str::<serde_json::Value>(input).is_ok_and(|asked| {
                        asked["fix"] == fix.as_str() && asked["head"] == output.commit.as_str()
                    })
                })
                .map(|(_, output)| encode(output)),
            ACT_FINISH_RUN => self.finish.as_ref().map(|output| encode(output)),
            _ => None,
        };
        served.unwrap_or_else(|| {
            Err(format!(
                "this xpd process took no measurement for {name}. Run the command again"
            ))
        })
    }

    /// The activities of a run, each a static entry that returns what was measured.
    pub(super) fn registry(self) -> ActivityRegistry {
        let this = Arc::new(self);
        let mut builder = ActivityRegistry::builder();
        for name in [
            ACT_INIT_BASELINE,
            ACT_VALIDATE_PLAN,
            ACT_GATE_FIX,
            ACT_FINISH_RUN,
        ] {
            let this = Arc::clone(&this);
            builder = builder.register(name, move |ctx: ActivityContext, input: String| {
                let served = this.serve(&ctx, &input);
                async move { served }
            });
        }
        builder.build()
    }
}

/// Encoding of an activity output, behind one object type.
mod erased {
    pub(super) trait Encode {
        fn encode(&self) -> Result<String, String>;
    }

    impl<T: serde::Serialize> Encode for T {
        fn encode(&self) -> Result<String, String> {
            serde_json::to_string(self).map_err(|error| error.to_string())
        }
    }
}

/// One value as another type of the same JSON shape.
pub(super) fn convert_json_shape<T: DeserializeOwned>(value: &impl Serialize) -> Result<T> {
    serde_json::from_value(serde_json::to_value(value)?)
        .context("the run and the ledger store disagree on the shape of a run")
}

/// The journal of a mirrored run.
pub(super) fn journal_of(state: &JournalState) -> Result<RunJournal> {
    convert_json_shape(state)
}

/// A command id that no other command of any process takes.
pub(super) fn command_id(kind: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{kind}-{}-{nanos}", std::process::id())
}

/// The command id of the import of the JSON journal `text`. A retry of the same import
/// gets the same id, so it finds the receipt of the first try.
fn import_id(text: &str) -> String {
    let mut hasher = SipHasher13::new_with_key(&IMPORT_KEY);
    hasher.write(text.as_bytes());
    format!("import-{:016x}", hasher.finish())
}

/// Where a JSON journal goes once the store holds its run.
pub(super) fn migrated_path(journal: &Path) -> PathBuf {
    let mut name = journal.as_os_str().to_owned();
    name.push(".migrated");
    PathBuf::from(name)
}

pub(super) fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(error).with_context(|| format!("cannot remove {}", path.display()))
        }
        _ => Ok(()),
    }
}

pub(super) fn require_applied_receipt(receipt: &Receipt) -> Result<()> {
    match receipt.outcome {
        ReceiptOutcome::Applied => Ok(()),
        ReceiptOutcome::Rejected => bail!(
            "{}",
            receipt
                .detail
                .as_deref()
                .unwrap_or("the run turned the command away")
        ),
    }
}

/// Opens the latest recorded `feature-review` journal for `workspace`, if one exists.
pub(super) fn feature_journal(workspace: &RunWorkspace) -> Result<Option<RunJournal>> {
    let store = WorktreeRunStore::open(workspace, WorkflowId::FeatureReview)?;
    let latest = store.latest()?;
    latest
        .as_ref()
        .map(|_| open_run(latest.as_ref(), store.workflow).map(|(_, journal)| journal))
        .transpose()
}

/// The run that is open, as a journal, with its generation.
pub(super) fn open_run(
    latest: Option<&LatestRun>,
    workflow: WorkflowId,
) -> Result<(u64, RunJournal)> {
    let cli = workflow.as_str();
    let Some(latest) = latest else {
        bail!("no run is open. Start one with `xpd {cli} start`");
    };
    let snapshot = match &latest.standing {
        RunStanding::Failed(details) => bail!(
            "the run failed in the ledger store: {details}. Replace it with `xpd {cli} start --restart`"
        ),
        RunStanding::Running(snapshot) | RunStanding::Completed(snapshot) => snapshot
            .as_ref()
            .context("the run has not published its state yet. Run the command again")?,
    };
    Ok((latest.generation, journal_of(&snapshot.run)?))
}

/// The commits whose findings a journal cites: the start, each kept fix and the current
/// commit. `None` stands for the working tree, which a plan is checked against.
fn cited_commits(journal: &RunJournal) -> BTreeSet<Option<&str>> {
    let kept = journal.fixes().filter_map(|fix| match &fix.outcome {
        Some(FixOutcome::Kept { commit, .. }) => Some(commit.as_str()),
        _ => None,
    });
    ([journal.base.as_str(), journal.current.as_str()].into_iter())
        .chain(kept)
        .map(Some)
        .chain([None])
        .collect()
}

/// Rewrites each finding id that `journal` cites to the id of today.
fn rewrite_ids(journal: &mut RunJournal, renamed: &BTreeMap<String, String>) -> Result<()> {
    let today: BTreeSet<&String> = renamed.values().collect();
    let mut unmatched = BTreeSet::new();
    let mut rewrite = |id: &mut String| {
        if let Some(new) = renamed.get(id.as_str()) {
            new.clone_into(id);
        } else if !today.contains(id) {
            unmatched.insert(id.clone());
        }
    };
    for fix in journal.rounds.iter_mut().flatten() {
        fix.findings.iter_mut().for_each(&mut rewrite);
        if let Some(FixOutcome::Kept { fixed, added, .. }) = &mut fix.outcome {
            fixed
                .iter_mut()
                .chain(added.iter_mut())
                .for_each(&mut rewrite);
        }
    }
    journal.accepted = std::mem::take(&mut journal.accepted)
        .into_iter()
        .map(|(mut id, reason)| {
            rewrite(&mut id);
            (id, reason)
        })
        .collect();
    ensure!(
        unmatched.is_empty(),
        "the run cites findings that none of its commits has: {}",
        unmatched.into_iter().collect::<Vec<_>>().join(", ")
    );
    Ok(())
}

/// Moves the legacy JSON journal of this worktree into the durable store, once.
pub(super) fn migrate_legacy_journal(
    store: &WorktreeRunStore,
    workspace: &RunWorkspace,
    renamed: Renamed<'_>,
) -> Result<()> {
    if !workspace.journal.is_file() {
        return Ok(());
    }
    let cli = store.workflow.as_str();
    let shown = workspace.journal.display();
    let text =
        fs::read_to_string(&workspace.journal).with_context(|| format!("cannot read {shown}"))?;
    let read: JournalState =
        serde_json::from_str(&text).with_context(|| format!("cannot read {shown}"))?;
    let mut journal = journal_of(&read)?;
    let mut ids = BTreeMap::new();
    for commit in cited_commits(&journal) {
        ids.extend(renamed(&journal, commit)?);
    }
    rewrite_ids(&mut journal, &ids).with_context(|| {
        format!(
            "{shown} stays. Finish its run with the xpd that wrote it, or replace it with `xpd {cli} start --restart`"
        )
    })?;
    let state: JournalState = convert_json_shape(&journal)?;
    let id = import_id(&text);
    let latest = store.latest()?;
    let generation = match &latest {
        None => 0,
        Some(latest) if store.has_receipt(latest.generation, &id)? => latest.generation,
        Some(LatestRun {
            standing: RunStanding::Running(_),
            ..
        }) => bail!(
            "{shown} holds a run, and the ledger store holds another open run of this worktree. Replace both with `xpd {cli} start --restart`"
        ),
        Some(latest) => latest.generation + 1,
    };
    let input = RunStartInput {
        command_id: id,
        command: store.workflow,
        paths: state.paths.clone(),
        checks: (state.checks.iter())
            .map(|check| check.command.clone())
            .collect(),
        bench: (state.bench.as_ref()).map(|bench| (bench.command.clone(), bench.runs)),
        adapter: state.adapter.clone(),
        initial_state: Some(state.clone()),
    };
    let (snapshot, receipt) = start_command(
        &store.config,
        &store.instance(generation),
        store.measurements(generation).registry(),
        &input,
        WAIT,
    )?;
    ensure!(
        receipt.outcome == ReceiptOutcome::Applied && snapshot.run == state,
        "the ledger store did not take the run of {shown} as it is, so the file stays"
    );
    let target = migrated_path(&workspace.journal);
    fs::rename(&workspace.journal, &target).with_context(|| format!("cannot rename {shown}"))?;
    eprintln!(
        "xpd: the run of {shown} is in the ledger store now, and the file is {}",
        target.display()
    );
    Ok(())
}

/// Keeps the log lines of the embedded runtime off stdout, where the run is printed.
///
/// A booting duroxide runtime installs a log subscriber that writes to stdout, filtered by
/// `RUST_LOG` or else at `warn`, so a retried SQLite lock would put a line into the JSON
/// of the run. Nothing else in xpd reads `RUST_LOG`, and a rejected command reaches the
/// user through its receipt, not a log line. `set_var` is unsound only while another
/// thread reads or writes the environment, and this runs at the start of a ledger stage,
/// which `main` reaches on one thread, before any Tokio runtime, SQLite connection or survey
/// thread pool exists.
pub(super) fn quiet_runtime_logs() {
    // SAFETY: no other thread exists yet, as the doc comment above explains.
    unsafe { std::env::set_var("RUST_LOG", "off") };
}
