//! `xpd ledger worker`: one worker of horizontal mode.
//!
//! A worker hosts the `duroxide` runtime over the PostgreSQL store, so it takes the
//! orchestration turns and the activities of every run in the store. It measures in its own
//! clone of the repository. Each activity fetches the run's branches from the remote, checks
//! the commit out in a detached worktree of its own, runs the checks and the surveys there, and
//! removes the worktree. The runtime renews the lock of an activity while it runs, so a worker
//! that dies stops renewing, and once `--lock-timeout` has passed another worker takes the
//! activity again from the start.
//!
//! A worker runs one activity at a time, in the folder of its worktree, because the checks and
//! the line count read the folder of the process. More workers serve more fixes at once.
//!
//! A benchmark is timed apart from the checks. A general worker started with `--bench-schema`
//! sends each timing as a `LedgerBench` orchestration to that schema, under an id that the run,
//! the commit and the commit it is timed against fix, so a retried gate finds the timing of the
//! first try. A worker started with `--bench` serves only that schema, one timing at a time. When
//! it runs on another class of machine than the one that timed the run's benchmark, it times
//! the commit the run stands at and the fix back to back, and the run keeps its own timings.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use duroxide::providers::Provider;
use duroxide::runtime::registry::{ActivityRegistry, OrchestrationRegistry};
use duroxide::runtime::{Runtime, RuntimeOptions};
use duroxide::{ActivityContext, Client, OrchestrationContext, OrchestrationStatus};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::git;

use super::ChecklistOwner;
use super::durable::{
    LedgerOrchestratorClient, PG_SCHEMA_VAR, PG_URL_VAR, REMOTE_VAR, open_postgres_store,
    orchestration_registry, run_branch,
};
use super::feature_evidence::FeatureEvidence;
use super::gate_judge::gate_checklist_files;
use super::orchestration::{
    ACT_FINISH_RUN, ACT_GATE_FIX, ACT_INIT_BASELINE, ACT_VALIDATE_PLAN, BaselineOutput, BenchState,
    CheckState, FinishOutput, GateOutput, RunStartInput, Snapshot, Timing, ValidatePlanOutput,
};
use super::run_probes::{
    SurveyedFindings, broken_checks, count_lines, diff_changed_files, host_fingerprint,
    measure_bench, shell_check_passes,
};
use super::workflow_registry::{FeatureEvidencePolicy, HeadMeasurementPolicy, WorkflowId};

/// Serve the ledger runs of a PostgreSQL store: take their orchestration turns, and gate
/// their fixes in detached worktrees of a clone of the repository.
///
/// The CLI sends a run to the store when `XPD_LEDGER_PG_URL` is set, and publishes the commits
/// of the run as branches `xpd/<run>/...` on the remote. Each worker fetches them into its own
/// clone. Stop a worker with Ctrl-C. A worker that is killed instead leaves its activity
/// locked until `--lock-timeout` passes, and then another worker takes it.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd ledger worker --pg-url postgres://xpd@db/ledger --repo ~/clones/app
  XPD_LEDGER_PG_URL=postgres://xpd@db/ledger xpd ledger worker --lock-timeout 60")]
pub(crate) struct WorkerArgs {
    /// The PostgreSQL URL of the ledger store. Default: the variable `XPD_LEDGER_PG_URL`.
    #[arg(long, value_name = "URL")]
    pg_url: Option<String>,
    /// The PostgreSQL schema of the runs. Default: the variable `XPD_LEDGER_PG_SCHEMA`, or
    /// `public`.
    #[arg(long)]
    schema: Option<String>,
    /// A clone of the repository, which the worker fetches the commits of each run into.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// The git remote that holds the branches of each run. Default: the variable
    /// `XPD_LEDGER_REMOTE`, or `origin`.
    #[arg(long)]
    remote: Option<String>,
    /// Seconds that an activity stays locked to this worker without a renewal. The worker
    /// renews the lock while it gates, so only a dead worker lets it run out.
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(10..=3600))]
    lock_timeout: u64,
    /// Serve only the benchmark timings that general workers send, one at a time. Run it on the
    /// bench host, with `--schema` set to the schema the general workers name in
    /// `--bench-schema`.
    #[arg(long, conflicts_with = "bench_schema")]
    bench: bool,
    /// The PostgreSQL schema of the bench workers. With it, this worker sends each benchmark
    /// timing there, instead of timing it next to the checks of other runs.
    #[arg(long, value_name = "SCHEMA")]
    bench_schema: Option<String>,
}

/// The orchestration that carries one benchmark timing to the bench workers.
const BENCH_ORCHESTRATION: &str = "LedgerBench";
/// The activity that times a benchmark, which only bench workers register.
const ACT_MEASURE_BENCH: &str = "MeasureBench";
/// How long a gate waits for its benchmark timing, queue included.
const BENCH_WAIT: Duration = Duration::from_secs(3600);

/// One timing that a bench worker takes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchRequest {
    /// The branch prefix of the run, whose branches hold both commits.
    branch: String,
    command: String,
    runs: usize,
    /// The commit to time.
    commit: String,
    /// The commit the run stands at, with its timing and the class of machine that took it.
    /// `None` for the commit a run starts from.
    against: Option<BenchAgainst>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchAgainst {
    commit: String,
    timing: Timing,
    host: Option<String>,
}

/// What a bench worker timed, and on which class of machine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchReply {
    host: String,
    /// The timing of `against`: the run's own when `paired` is false.
    before: Option<Timing>,
    after: Timing,
    /// Whether `before` and `after` were timed back to back here, because the run's timing
    /// comes from another class of machine.
    paired: bool,
}

/// Whether a timing taken on `here` must be paired with a new timing of the commit the run
/// stands at: when the run's timing comes from another class of machine, or from an unknown one.
fn needs_pairing(recorded: Option<&str>, here: &str) -> bool {
    recorded != Some(here)
}

/// The orchestration of one benchmark timing: one activity, on a bench worker.
async fn ledger_bench(ctx: OrchestrationContext, input: String) -> Result<String, String> {
    ctx.schedule_activity(ACT_MEASURE_BENCH, input).await
}

/// A variable of the environment, when it is set and not empty.
fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Runs the distributed ledger worker until Ctrl-C.
pub(crate) fn run_ledger_worker(args: WorkerArgs) -> Result<ExitCode> {
    let url = (args.pg_url.or_else(|| variable(PG_URL_VAR)))
        .context("name the PostgreSQL store with --pg-url or XPD_LEDGER_PG_URL")?;
    let schema = args.schema.or_else(|| variable(PG_SCHEMA_VAR));
    let remote = (args.remote.or_else(|| variable(REMOTE_VAR))).unwrap_or_else(|| "origin".into());
    let top = PathBuf::from(
        git::output(&args.repo, &["rev-parse", "--show-toplevel"])
            .with_context(|| format!("{} is not in a git repository", args.repo.display()))?
            .trim(),
    );
    let common = PathBuf::from(
        git::output(
            &top,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?
        .trim(),
    );
    let shelf = Shelf::claim(&top, &common.join("xpd-worker"))?;
    let xpd = std::env::current_exe().context("cannot find the xpd binary")?;
    let lock = Duration::from_secs(args.lock_timeout);
    let options = RuntimeOptions {
        worker_concurrency: 1,
        worker_lock_timeout: lock,
        worker_lock_renewal_buffer: (lock / 10).max(Duration::from_secs(5)),
        session_idle_timeout: lock + Duration::from_secs(60),
        ..RuntimeOptions::default()
    };
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot build the Tokio runtime of the worker")?;
    tokio.block_on(async move {
        let store = open_postgres_store(&url, schema.as_deref()).await?;
        let bench_store = match &args.bench_schema {
            Some(bench_schema) => Some(open_postgres_store(&url, Some(bench_schema)).await?),
            None => None,
        };
        let worker = Arc::new(Worker {
            top,
            remote,
            shelf,
            made: AtomicUsize::new(0),
            xpd,
            bench_store,
        });
        let (activities, orchestrations, serves) = if args.bench {
            let orchestrations = OrchestrationRegistry::builder()
                .register(BENCH_ORCHESTRATION, ledger_bench)
                .build();
            (worker.bench_registry(), orchestrations, "benchmark timings")
        } else {
            (worker.registry(), orchestration_registry(), "ledger runs")
        };
        let runtime =
            Runtime::start_with_options(store, activities, orchestrations, options).await;
        eprintln!(
            "xpd ledger worker {}: serving {serves} of the schema {} with the clone {} and the remote {}, on {}",
            std::process::id(),
            schema.as_deref().unwrap_or("public"),
            worker.top.display(),
            worker.remote,
            host_fingerprint()
        );
        let stopped = tokio::signal::ctrl_c().await;
        runtime.shutdown(Some(10_000)).await;
        stopped.context("cannot wait for Ctrl-C")?;
        Ok(ExitCode::SUCCESS)
    })
}

/// The folder of this worker's worktrees, `<git-common-dir>/xpd-worker/<pid>`, which it holds
/// by a lock on `<pid>.lock` for as long as it lives.
#[derive(Debug)]
struct Shelf {
    folder: PathBuf,
    /// Held, never read: the lock ends with the process, so the next worker knows it is gone.
    _lock: File,
}

impl Shelf {
    /// Takes the folder of this process, after removing the folders of workers that are gone.
    /// A worker is gone when the lock of its folder is free.
    fn claim(top: &Path, root: &Path) -> Result<Self> {
        fs::create_dir_all(root).with_context(|| format!("cannot create {}", root.display()))?;
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "lock") {
                continue;
            }
            let file = File::options().write(true).open(&path)?;
            if file.try_lock().is_ok() {
                let folder = path.with_extension("");
                if folder.exists() {
                    fs::remove_dir_all(&folder)
                        .with_context(|| format!("cannot remove {}", folder.display()))?;
                }
                fs::remove_file(&path)?;
            }
        }
        git::output(top, &["worktree", "prune"])?;
        let name = std::process::id().to_string();
        let lock = File::create(root.join(format!("{name}.lock")))?;
        lock.lock()?;
        let folder = root.join(name);
        fs::create_dir_all(&folder)?;
        Ok(Self {
            folder,
            _lock: lock,
        })
    }
}

/// A detached worktree of the clone at one commit, removed when it is dropped.
struct Worktree<'w> {
    top: &'w Path,
    path: PathBuf,
}

impl Drop for Worktree<'_> {
    fn drop(&mut self) {
        let path = self.path.to_string_lossy().into_owned();
        let _ = git::output(self.top, &["worktree", "remove", "--force", &path]);
    }
}

/// The folder of the process, set to a worktree until it is dropped.
struct InFolder<'w>(&'w Path);

impl InFolder<'_> {
    fn enter<'w>(worktree: &Worktree<'w>) -> Result<InFolder<'w>> {
        std::env::set_current_dir(&worktree.path)
            .with_context(|| format!("cannot enter {}", worktree.path.display()))?;
        Ok(InFolder(worktree.top))
    }
}

impl Drop for InFolder<'_> {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(self.0);
    }
}

/// What `GateFix` asks for.
#[derive(Debug, serde::Deserialize)]
struct GateRequest {
    current: String,
    head: String,
    paths: Vec<String>,
}

struct Worker {
    /// The top of the clone.
    top: PathBuf,
    remote: String,
    shelf: Shelf,
    /// How many worktrees this worker has made, which names the next one.
    made: AtomicUsize,
    /// The xpd binary, which surveys a worktree in a process of its own.
    xpd: PathBuf,
    /// The store of the bench workers, which time each benchmark when it is set.
    bench_store: Option<Arc<dyn Provider>>,
}

impl std::fmt::Debug for Worker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Worker")
            .field("top", &self.top)
            .field("remote", &self.remote)
            .field("shelf", &self.shelf)
            .field("sends_bench", &self.bench_store.is_some())
            .finish_non_exhaustive()
    }
}

impl Worker {
    /// The one activity of a bench worker: a benchmark timing, off the Tokio threads.
    fn bench_registry(self: &Arc<Self>) -> ActivityRegistry {
        let worker = Arc::clone(self);
        ActivityRegistry::builder()
            .register(
                ACT_MEASURE_BENCH,
                move |ctx: ActivityContext, input: String| {
                    let worker = Arc::clone(&worker);
                    async move {
                        let pid = std::process::id();
                        let instance = ctx.instance_id().to_owned();
                        eprintln!("xpd ledger worker {pid}: {ACT_MEASURE_BENCH} of {instance} starts");
                        let timed = tokio::task::spawn_blocking(move || {
                            worker.time_bench(&decode(&input)?).and_then(|reply| encode(&reply))
                        })
                        .await
                        .map_err(|error| format!("the timing thread ended: {error}"))?;
                        match &timed {
                            Ok(reply) => eprintln!("xpd ledger worker {pid}: {ACT_MEASURE_BENCH} of {instance} done: {reply}"),
                            Err(error) => eprintln!("xpd ledger worker {pid}: {ACT_MEASURE_BENCH} of {instance} failed: {error:#}"),
                        }
                        timed.map_err(|error| format!("{error:#}"))
                    }
                },
            )
            .build()
    }

    /// Times a benchmark on this machine. The commit the run stands at is timed again, just
    /// before the commit of the request, when the run's timing comes from another class of
    /// machine.
    fn time_bench(&self, request: &BenchRequest) -> Result<BenchReply> {
        self.fetch(&request.branch)?;
        let host = host_fingerprint();
        let time = |commit: &str| -> Result<Timing> {
            let worktree = self.worktree(&self.resolve(commit)?)?;
            let _in = InFolder::enter(&worktree)?;
            convert(&measure_bench(&request.command, request.runs)?)
        };
        let (before, paired) = match &request.against {
            None => (None, false),
            Some(against) if needs_pairing(against.host.as_deref(), &host) => {
                (Some(time(&against.commit)?), true)
            }
            Some(against) => (Some(against.timing), false),
        };
        let after = time(&request.commit)?;
        Ok(BenchReply {
            host,
            before,
            after,
            paired,
        })
    }

    /// A benchmark timing: from the bench workers when their schema is set, and on this worker
    /// otherwise. The id of the bench orchestration comes from the request alone, so a gate
    /// that runs again after its worker died waits for the timing it asked for before.
    fn bench(&self, instance: &str, request: &BenchRequest) -> Result<BenchReply> {
        let Some(store) = &self.bench_store else {
            return self.time_bench(request);
        };
        let against = (request.against.as_ref()).map_or("start", |against| against.commit.as_str());
        let id = format!("bench::{instance}::{}::{against}", request.commit);
        let input = encode(request)?;
        let client = Client::new(Arc::clone(store));
        tokio::runtime::Handle::current().block_on(async {
            if matches!(
                client.get_orchestration_status(&id).await?,
                OrchestrationStatus::NotFound
            ) {
                client
                    .start_orchestration(&id, BENCH_ORCHESTRATION, input)
                    .await?;
            }
            match client
                .wait_for_orchestration_typed::<BenchReply>(&id, BENCH_WAIT)
                .await?
            {
                Ok(reply) => Ok(reply),
                Err(failed) => bail!("the bench worker could not time {id}: {failed}"),
            }
        })
    }

    /// The activities of a run, each served by measuring in a worktree.
    fn registry(self: &Arc<Self>) -> ActivityRegistry {
        let mut builder = ActivityRegistry::builder();
        for name in [
            ACT_INIT_BASELINE,
            ACT_VALIDATE_PLAN,
            ACT_GATE_FIX,
            ACT_FINISH_RUN,
        ] {
            let worker = Arc::clone(self);
            builder = builder.register(name, move |ctx: ActivityContext, input: String| {
                Arc::clone(&worker).serve(ctx, input)
            });
        }
        builder.build()
    }

    /// Serves one activity. The measuring runs off the Tokio threads, so the runtime keeps
    /// renewing the lock of the activity while the checks run.
    async fn serve(self: Arc<Self>, ctx: ActivityContext, input: String) -> Result<String, String> {
        let name = ctx.activity_name().to_owned();
        let instance = ctx.instance_id().to_owned();
        let snapshot = if name == ACT_INIT_BASELINE {
            None
        } else {
            let client = LedgerOrchestratorClient::of(ctx.get_client(), instance.clone());
            let read = client
                .snapshot()
                .await
                .map_err(|error| format!("{error:#}"))?;
            Some(read.ok_or("the run has published no state yet")?)
        };
        let pid = std::process::id();
        eprintln!("xpd ledger worker {pid}: {name} of {instance} starts");
        let started = std::time::Instant::now();
        let measured = tokio::task::spawn_blocking({
            let name = name.clone();
            let instance = instance.clone();
            move || self.measure(&name, &instance, &input, snapshot.as_ref())
        })
        .await
        .map_err(|error| format!("the measuring thread ended: {error}"))?;
        let took = started.elapsed().as_secs_f64();
        match &measured {
            Ok(_) => eprintln!("xpd ledger worker {pid}: {name} of {instance} done in {took:.1} s"),
            Err(error) => {
                eprintln!("xpd ledger worker {pid}: {name} of {instance} failed: {error:#}");
            }
        }
        measured.map_err(|error| format!("{error:#}"))
    }

    fn measure(
        &self,
        name: &str,
        instance: &str,
        input: &str,
        snapshot: Option<&Snapshot>,
    ) -> Result<String> {
        let workflow: WorkflowId = (instance.split("::").nth(1))
            .with_context(|| format!("the instance {instance} names no command"))?
            .parse()?;
        let branch = run_branch(instance);
        self.fetch(&branch)?;
        let head = format!("refs/remotes/{}/{branch}/head", self.remote);
        match (name, snapshot) {
            (ACT_INIT_BASELINE, _) => {
                encode(&self.baseline(instance, &branch, &decode(input)?, &head)?)
            }
            (ACT_VALIDATE_PLAN, Some(snapshot)) => {
                let paths: Vec<String> = decode(input)?;
                let worktree = self.worktree(&head)?;
                self.require_feature_head(workflow, &head, snapshot)?;
                let (known, checklist, feature_evidence) =
                    self.findings(&worktree, workflow, &paths, snapshot.run.adapter.as_deref())?;
                encode(&ValidatePlanOutput {
                    known,
                    checklist: super::checklist_files(checklist),
                    feature_evidence,
                })
            }
            (ACT_GATE_FIX, Some(snapshot)) => {
                encode(&self.gate(instance, &branch, workflow, &decode(input)?, snapshot)?)
            }
            (ACT_FINISH_RUN, Some(snapshot)) => {
                let paths: Vec<String> = decode(input)?;
                encode(&self.finish(workflow, &paths, &head, snapshot)?)
            }
            _ => bail!("this worker has no activity {name}"),
        }
    }

    /// The branches of the run, from the remote into the clone.
    fn fetch(&self, branch: &str) -> Result<()> {
        let remote = &self.remote;
        git::output(
            &self.top,
            &[
                "fetch",
                "--quiet",
                remote,
                &format!("+refs/heads/{branch}/*:refs/remotes/{remote}/{branch}/*"),
            ],
        )
        .with_context(|| format!("cannot fetch the branches of the run from {remote}"))?;
        Ok(())
    }

    /// A new detached worktree at `commit`, which may be a ref.
    fn worktree(&self, commit: &str) -> Result<Worktree<'_>> {
        let made = self.made.fetch_add(1, Ordering::Relaxed);
        let path = self.shelf.folder.join(made.to_string());
        let shown = path.to_string_lossy().into_owned();
        git::output(
            &self.top,
            &["worktree", "add", "--quiet", "--detach", &shown, commit],
        )
        .with_context(|| format!("cannot check {commit} out"))?;
        Ok(Worktree {
            top: &self.top,
            path,
        })
    }

    /// The full id of the commit that `rev` names in the clone.
    fn resolve(&self, rev: &str) -> Result<String> {
        let resolved = git::output(
            &self.top,
            &["rev-parse", "--verify", &format!("{rev}^{{commit}}")],
        )
        .with_context(|| format!("{rev} names no commit of the run's branches"))?;
        Ok(resolved.trim().to_owned())
    }

    /// The finding ids and checklist map (`id -> tracked file`) of `workflow` under `paths` in
    /// a worktree. The survey is xpd itself, run in the worktree, so its ids match the CLI.
    fn findings(
        &self,
        worktree: &Worktree<'_>,
        workflow: WorkflowId,
        paths: &[String],
        adapter: Option<&str>,
    ) -> Result<SurveyedFindings> {
        let cli = workflow.as_str();
        let mut cmd = Command::new(&self.xpd);
        cmd.args([cli, "--format", "json", "--limit", "0"]);
        if let Some(adapter) = adapter {
            cmd.args(["--adapter", adapter]);
        }
        let output = cmd
            .arg("--")
            .args(paths)
            .current_dir(&worktree.path)
            .stdin(Stdio::null())
            .output()
            .context("cannot run the survey")?;
        if !output.status.success() {
            bail!(
                "the survey `xpd {cli}` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let ledger: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("the survey printed no JSON")?;
        validate_worker_survey(workflow, &ledger)?;
        let ids: BTreeSet<String> = (ledger["findings"].as_array())
            .context("the survey printed no findings")?
            .iter()
            .map(|finding| {
                (finding["id"].as_str())
                    .map(str::to_owned)
                    .context("a finding of the survey has no id")
            })
            .collect::<Result<_>>()?;
        let mut checklist = BTreeMap::new();
        if let Some(items) = ledger["checklist"].as_array() {
            for item in items {
                if let (Some(id), Some(file)) = (item["id"].as_str(), item["file"].as_str()) {
                    checklist.insert(
                        id.to_owned(),
                        ChecklistOwner {
                            file: file.to_owned(),
                            requires_na: item["requires_na"] == true,
                        },
                    );
                }
            }
        }
        let feature_evidence = (workflow.spec().feature_evidence
            == FeatureEvidencePolicy::FoldAblation)
            .then(|| FeatureEvidence::from_read(&ledger["read"]))
            .transpose()?;
        Ok((ids, checklist, feature_evidence))
    }

    /// The checks, benchmark, findings and lines of the commit a run starts from.
    fn baseline(
        &self,
        instance: &str,
        branch: &str,
        start: &RunStartInput,
        head: &str,
    ) -> Result<BaselineOutput> {
        let base = self.resolve(head)?;
        let worktree = self.worktree(&base)?;
        let (checks, was, was_lines, feature_evidence) = {
            let _in = InFolder::enter(&worktree)?;
            let checks = (start.checks.iter())
                .map(|command| CheckState {
                    passed: shell_check_passes(command).0,
                    command: command.clone(),
                })
                .collect();
            let (findings, checklist, feature_evidence) = self.findings(
                &worktree,
                start.command,
                &start.paths,
                start.adapter.as_deref(),
            )?;
            if let Some(empty_error) = start.command.spec().checklist.empty_inventory_error() {
                ensure!(!checklist.is_empty(), "{empty_error}");
            }
            let paths: Vec<PathBuf> = start.paths.iter().map(PathBuf::from).collect();
            (
                checks,
                findings.len(),
                count_lines(&paths),
                feature_evidence,
            )
        };
        drop(worktree);
        let bench = (start.bench.as_ref())
            .map(|(command, runs)| -> Result<BenchState> {
                let request = BenchRequest {
                    branch: branch.to_owned(),
                    command: command.clone(),
                    runs: *runs,
                    commit: base.clone(),
                    against: None,
                };
                let reply = self.bench(instance, &request)?;
                Ok(BenchState {
                    command: command.clone(),
                    runs: reply.after.runs,
                    baseline: reply.after,
                    current: reply.after,
                    host_fingerprint: Some(reply.host),
                })
            })
            .transpose()?;
        Ok(BaselineOutput {
            base,
            paths: start.paths.clone(),
            checks,
            bench,
            adapter: start.adapter.clone(),
            was,
            was_lines,
            feature_evidence,
        })
    }

    /// What a gate measures of the fix at `request.head` against the commit the run stands at.
    fn gate(
        &self,
        instance: &str,
        branch: &str,
        workflow: WorkflowId,
        request: &GateRequest,
        snapshot: &Snapshot,
    ) -> Result<GateOutput> {
        let run = &snapshot.run;
        let adapter = run.adapter.as_deref();
        let commit = self.resolve(&request.head)?;
        let current = self.resolve(&request.current)?;
        let after_tree = self.worktree(&commit)?;
        let (broken, after, lines_after, checklist_after, feature_after) = {
            let _in = InFolder::enter(&after_tree)?;
            let broken = broken_checks(
                workflow,
                run.checks
                    .iter()
                    .map(|check| (check.command.as_str(), check.passed)),
            );
            let (after, checklist_after, feature_after) =
                self.findings(&after_tree, workflow, &request.paths, adapter)?;
            let paths: Vec<PathBuf> = request.paths.iter().map(PathBuf::from).collect();
            (
                broken,
                after,
                count_lines(&paths),
                checklist_after,
                feature_after,
            )
        };
        drop(after_tree);
        let timed = match &run.bench {
            Some(bench) if broken.is_empty() => {
                let request = BenchRequest {
                    branch: branch.to_owned(),
                    command: bench.command.clone(),
                    runs: bench.runs,
                    commit: commit.clone(),
                    against: Some(BenchAgainst {
                        commit: request.current.clone(),
                        timing: bench.current,
                        host: bench.host_fingerprint.clone(),
                    }),
                };
                Some(self.bench(instance, &request)?)
            }
            _ => None,
        };
        let paired = timed.as_ref().is_some_and(|reply| reply.paired);
        let timing = timed.map(|reply| (reply.before.unwrap_or(reply.after), reply.after));
        let before_tree = self.worktree(&current)?;
        let (before, checklist_before, feature_before) =
            self.findings(&before_tree, workflow, &request.paths, adapter)?;
        let changed = diff_changed_files(&self.top, &current, &commit)?;
        Ok(GateOutput {
            commit,
            broken,
            before,
            after,
            lines_before: run.current_lines,
            lines_after,
            timing,
            paired,
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

    fn require_feature_head(
        &self,
        workflow: WorkflowId,
        head: &str,
        snapshot: &Snapshot,
    ) -> Result<()> {
        if workflow.spec().head == HeadMeasurementPolicy::LockedKeptCommit {
            ensure!(
                self.resolve(head)? == snapshot.run.current,
                "feature review must measure the last kept commit {}",
                snapshot.run.current
            );
        }
        Ok(())
    }

    /// What `finish` compares: the findings of the start commit and of the last published
    /// commit, and the lines of the latter.
    fn finish(
        &self,
        workflow: WorkflowId,
        paths: &[String],
        head: &str,
        snapshot: &Snapshot,
    ) -> Result<FinishOutput> {
        self.require_feature_head(workflow, head, snapshot)?;
        let adapter = snapshot.run.adapter.as_deref();
        let head_tree = self.worktree(&self.resolve(head)?)?;
        let (after, checklist, feature_evidence) =
            self.findings(&head_tree, workflow, paths, adapter)?;
        let current_lines = {
            let _in = InFolder::enter(&head_tree)?;
            count_lines(&paths.iter().map(PathBuf::from).collect::<Vec<_>>())
        };
        drop(head_tree);
        let base_tree = self.worktree(&self.resolve(&snapshot.run.base)?)?;
        let (before, _, _) = self.findings(&base_tree, workflow, paths, adapter)?;
        Ok(FinishOutput {
            before,
            after,
            current_lines,
            checklist_requires_na: checklist
                .iter()
                .filter(|(_, owner)| owner.requires_na)
                .map(|(id, _)| id.clone())
                .collect(),
            checklist_after: checklist.into_keys().collect(),
            feature_evidence,
        })
    }
}

/// Validates workflow-specific invariants on the JSON emitted by a worker's `xpd <workflow>` run.
///
/// Matches every [`WorkflowId`] variant explicitly without a wildcard arm so adding a workflow
/// without defining its worker validation fails compilation with `E0004`.
fn validate_worker_survey(workflow: WorkflowId, ledger: &serde_json::Value) -> Result<()> {
    let check_facts_error = || -> Result<()> {
        if let Some(error) = ledger["read"]["facts"]["error"].as_str() {
            bail!("the {} adapter failed: {error}", workflow.as_str());
        }
        Ok(())
    };
    match workflow {
        WorkflowId::ChartReview
        | WorkflowId::Discover
        | WorkflowId::FeatureReview
        | WorkflowId::Harden
        | WorkflowId::OptReview
        | WorkflowId::Optimize
        | WorkflowId::Untangle => Ok(()),
        WorkflowId::DataPrep | WorkflowId::TsReview => check_facts_error(),
        WorkflowId::ModelReview => {
            check_facts_error()?;
            ensure!(
                ledger["read"]["ambiguities"]
                    .as_array()
                    .is_some_and(Vec::is_empty),
                "ambiguous model bindings in the selected Python files"
            );
            Ok(())
        }
    }
}

fn decode<T: DeserializeOwned>(input: &str) -> Result<T> {
    serde_json::from_str(input).context("the activity input is not what this worker reads")
}

fn encode(output: &impl Serialize) -> Result<String> {
    Ok(serde_json::to_string(output)?)
}

/// One value as another type of the same JSON shape.
fn convert<T: DeserializeOwned>(value: &impl Serialize) -> Result<T> {
    Ok(serde_json::from_value(serde_json::to_value(value)?)?)
}
