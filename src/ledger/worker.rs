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

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use duroxide::ActivityContext;
use duroxide::runtime::registry::ActivityRegistry;
use duroxide::runtime::{Runtime, RuntimeOptions};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::git;

use super::super::orchestration::{
    ACT_FINISH_RUN, ACT_GATE_FIX, ACT_INIT_BASELINE, ACT_VALIDATE_PLAN, BaselineOutput, BenchState,
    CheckState, FinishOutput, GateOutput, RunStartInput, Snapshot, Timing, ValidatePlanOutput,
};
use super::super::{count_lines, measure_bench, passes};
use super::{
    LedgerOrchestratorClient, PG_SCHEMA_VAR, PG_URL_VAR, REMOTE_VAR, open_postgres_store,
    orchestration_registry, run_branch,
};

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
}

/// A variable of the environment, when it is set and not empty.
fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Runs the worker until Ctrl-C.
pub(crate) fn run(args: WorkerArgs) -> Result<ExitCode> {
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
    let worker = Arc::new(Worker {
        top,
        remote,
        shelf,
        made: AtomicUsize::new(0),
        xpd: std::env::current_exe().context("cannot find the xpd binary")?,
    });
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
        let activities = Arc::clone(&worker).registry();
        let runtime =
            Runtime::start_with_options(store, activities, orchestration_registry(), options).await;
        eprintln!(
            "xpd ledger worker {}: serving the store with the clone {} and the remote {}",
            std::process::id(),
            worker.top.display(),
            worker.remote
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

#[derive(Debug)]
struct Worker {
    /// The top of the clone.
    top: PathBuf,
    remote: String,
    shelf: Shelf,
    /// How many worktrees this worker has made, which names the next one.
    made: AtomicUsize,
    /// The xpd binary, which surveys a worktree in a process of its own.
    xpd: PathBuf,
}

impl Worker {
    /// The activities of a run, each served by measuring in a worktree.
    fn registry(self: Arc<Self>) -> ActivityRegistry {
        let mut builder = ActivityRegistry::builder();
        for name in [
            ACT_INIT_BASELINE,
            ACT_VALIDATE_PLAN,
            ACT_GATE_FIX,
            ACT_FINISH_RUN,
        ] {
            let worker = Arc::clone(&self);
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
        let command = (instance.split("::").nth(1))
            .with_context(|| format!("the instance {instance} names no command"))?;
        let branch = run_branch(instance);
        self.fetch(&branch)?;
        let head = format!("refs/remotes/{}/{branch}/head", self.remote);
        match (name, snapshot) {
            (ACT_INIT_BASELINE, _) => encode(&self.baseline(&decode(input)?, &head)?),
            (ACT_VALIDATE_PLAN, _) => {
                let paths: Vec<String> = decode(input)?;
                let worktree = self.worktree(&head)?;
                let known = self.findings(&worktree, command, &paths)?;
                encode(&ValidatePlanOutput { known })
            }
            (ACT_GATE_FIX, Some(snapshot)) => {
                encode(&self.gate(command, &decode(input)?, snapshot)?)
            }
            (ACT_FINISH_RUN, Some(snapshot)) => {
                let paths: Vec<String> = decode(input)?;
                encode(&self.finish(command, &paths, &head, snapshot)?)
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

    /// The ids of the findings of `command` under `paths` in a worktree. The survey is xpd
    /// itself, run in the worktree, so its ids are the ids the CLI prints.
    fn findings(
        &self,
        worktree: &Worktree<'_>,
        command: &str,
        paths: &[String],
    ) -> Result<BTreeSet<String>> {
        let output = Command::new(&self.xpd)
            .args([command, "--format", "json", "--limit", "0", "--"])
            .args(paths)
            .current_dir(&worktree.path)
            .stdin(Stdio::null())
            .output()
            .context("cannot run the survey")?;
        if !output.status.success() {
            bail!(
                "the survey `xpd {command}` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let ledger: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("the survey printed no JSON")?;
        (ledger["findings"].as_array())
            .context("the survey printed no findings")?
            .iter()
            .map(|finding| {
                (finding["id"].as_str())
                    .map(str::to_owned)
                    .context("a finding of the survey has no id")
            })
            .collect()
    }

    /// The checks, benchmark, findings and lines of the commit a run starts from.
    fn baseline(&self, start: &RunStartInput, head: &str) -> Result<BaselineOutput> {
        let base = self.resolve(head)?;
        let worktree = self.worktree(&base)?;
        let _in = InFolder::enter(&worktree)?;
        let checks = (start.checks.iter())
            .map(|command| CheckState {
                passed: passes(command).0,
                command: command.clone(),
            })
            .collect();
        let bench = (start.bench.as_ref())
            .map(|(command, runs)| -> Result<BenchState> {
                let timing: Timing = convert(&measure_bench(command, *runs)?)?;
                Ok(BenchState {
                    command: command.clone(),
                    runs: timing.runs,
                    baseline: timing,
                    current: timing,
                })
            })
            .transpose()?;
        let was = self
            .findings(&worktree, &start.command, &start.paths)?
            .len();
        let paths: Vec<PathBuf> = start.paths.iter().map(PathBuf::from).collect();
        Ok(BaselineOutput {
            base,
            paths: start.paths.clone(),
            checks,
            bench,
            was,
            was_lines: count_lines(&paths),
        })
    }

    /// What a gate measures of the fix at `request.head` against the commit the run stands at.
    fn gate(
        &self,
        command: &str,
        request: &GateRequest,
        snapshot: &Snapshot,
    ) -> Result<GateOutput> {
        let run = &snapshot.run;
        let commit = self.resolve(&request.head)?;
        let after_tree = self.worktree(&commit)?;
        let (broken, after, lines_after, timing) = {
            let _in = InFolder::enter(&after_tree)?;
            let broken: Vec<String> = (run.checks.iter())
                .filter(|check| check.passed)
                .filter_map(|check| {
                    let (passed, printed) = passes(&check.command);
                    (!passed).then(|| format!("{}\n{printed}", check.command))
                })
                .collect();
            let after = self.findings(&after_tree, command, &request.paths)?;
            let paths: Vec<PathBuf> = request.paths.iter().map(PathBuf::from).collect();
            let lines_after = count_lines(&paths);
            let timing = match &run.bench {
                Some(bench) if broken.is_empty() => {
                    let after_timing: Timing =
                        convert(&measure_bench(&bench.command, bench.runs)?)?;
                    Some((bench.current, after_timing))
                }
                _ => None,
            };
            (broken, after, lines_after, timing)
        };
        drop(after_tree);
        let before_tree = self.worktree(&self.resolve(&request.current)?)?;
        let before = self.findings(&before_tree, command, &request.paths)?;
        Ok(GateOutput {
            commit,
            broken,
            before,
            after,
            lines_before: run.current_lines,
            lines_after,
            timing,
        })
    }

    /// What `finish` compares: the findings of the start commit and of the last published
    /// commit, and the lines of the latter.
    fn finish(
        &self,
        command: &str,
        paths: &[String],
        head: &str,
        snapshot: &Snapshot,
    ) -> Result<FinishOutput> {
        let head_tree = self.worktree(&self.resolve(head)?)?;
        let after = self.findings(&head_tree, command, paths)?;
        let current_lines = {
            let _in = InFolder::enter(&head_tree)?;
            count_lines(&paths.iter().map(PathBuf::from).collect::<Vec<_>>())
        };
        drop(head_tree);
        let base_tree = self.worktree(&self.resolve(&snapshot.run.base)?)?;
        let before = self.findings(&base_tree, command, paths)?;
        Ok(FinishOutput {
            before,
            after,
            current_lines,
        })
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
