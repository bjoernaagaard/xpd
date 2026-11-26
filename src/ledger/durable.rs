//! The adapter between a ledger run and `duroxide`.
//!
//! Everything `duroxide` knows about lives behind this module and
//! [`super::orchestration`]. A caller opens one SQLite store in the common git
//! folder, boots one embedded runtime for a mutating command, and reads
//! `status` back through a client alone. With `ledger-distributed` and
//! `XPD_LEDGER_PG_URL` set, the store is PostgreSQL instead: the CLI boots no
//! runtime, publishes its commits to the run's branches on a git remote, and
//! `xpd ledger worker` processes measure and gate in their own clones. No runtime, and no Tokio runtime,
//! exists until a caller asks for one: [`LazyTokio`] holds the runtime it
//! builds on first use and nothing else in this module builds one, so a hot
//! path that never names this module pays nothing for it.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use duroxide::providers::Provider;
use duroxide::providers::sqlite::SqliteProvider;
use duroxide::runtime::Runtime;
use duroxide::runtime::registry::{ActivityRegistry, OrchestrationRegistry};
use duroxide::{Client, OrchestrationStatus};

#[cfg(feature = "ledger-distributed")]
use super::orchestration::Command;
use super::orchestration::{
    COMMAND_QUEUE, CommandEnvelope, ORCHESTRATION_NAME, Receipt, RunStartInput, Snapshot,
    ledger_run, receipt_key,
};

/// The variable that names the PostgreSQL store of horizontal mode.
#[cfg(feature = "ledger-distributed")]
pub(crate) const PG_URL_VAR: &str = "XPD_LEDGER_PG_URL";
/// The variable that names the PostgreSQL schema of a repository in horizontal mode.
#[cfg(feature = "ledger-distributed")]
pub(crate) const PG_SCHEMA_VAR: &str = "XPD_LEDGER_PG_SCHEMA";
/// The variable that names the git remote that holds the branches of each run.
#[cfg(feature = "ledger-distributed")]
pub(crate) const REMOTE_VAR: &str = "XPD_LEDGER_REMOTE";

/// The store a run lives in, chosen by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderConfig {
    /// Local default: `<git-common-dir>/xpd-ledger.sqlite`, shared by worktrees.
    Sqlite { db_path: PathBuf },
    /// Horizontal mode, compiled only with `ledger-distributed`. `remote` is the git remote
    /// that the CLI publishes each run's commits to, and that every worker fetches from.
    #[cfg(feature = "ledger-distributed")]
    Postgres {
        connection_url: String,
        schema: Option<String>,
        remote: String,
    },
}

impl ProviderConfig {
    /// The store of the repository whose common git folder is `git_common_dir`: PostgreSQL
    /// when `XPD_LEDGER_PG_URL` is set in a build with `ledger-distributed`, and otherwise the
    /// SQLite file in that folder.
    pub(crate) fn for_repository(git_common_dir: &Path) -> Self {
        #[cfg(feature = "ledger-distributed")]
        if let Some(connection_url) = std::env::var(PG_URL_VAR).ok().filter(|url| !url.is_empty()) {
            return Self::Postgres {
                connection_url,
                schema: std::env::var(PG_SCHEMA_VAR)
                    .ok()
                    .filter(|name| !name.is_empty()),
                remote: std::env::var(REMOTE_VAR)
                    .ok()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "origin".to_owned()),
            };
        }
        Self::Sqlite {
            db_path: git_common_dir.join("xpd-ledger.sqlite"),
        }
    }
}

/// Opens the store of `config`: the SQLite file, or the PostgreSQL schema, whose migrations
/// run on this first open.
pub(crate) async fn open_store(config: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    match config {
        ProviderConfig::Sqlite { db_path } => Ok(open_sqlite_store(db_path).await?),
        #[cfg(feature = "ledger-distributed")]
        ProviderConfig::Postgres {
            connection_url,
            schema,
            ..
        } => open_postgres_store(connection_url, schema.as_deref()).await,
    }
}

/// Opens a PostgreSQL store, in `schema` when one is given.
#[cfg(feature = "ledger-distributed")]
pub(crate) async fn open_postgres_store(
    connection_url: &str,
    schema: Option<&str>,
) -> Result<Arc<dyn Provider>> {
    let provider = duroxide_pg::PostgresProvider::new_with_schema(connection_url, schema)
        .await
        .context("cannot open the PostgreSQL ledger store")?;
    Ok(Arc::new(provider))
}

/// The branch prefix of a run on the git remote, `xpd/<namespace>/<command>/<generation>`.
/// Its `base` branch holds the commit the run started from, `head` the commit of the last
/// command, and `fix/<id>` the commit each fix was gated at, so that every commit the run
/// names stays on the remote.
#[cfg(feature = "ledger-distributed")]
pub(crate) fn run_branch(instance_id: &str) -> String {
    format!("xpd/{}", instance_id.replace("::", "/"))
}

/// The `sqlite:` URL of a store file, created when it is missing.
pub(crate) fn sqlite_url(db_path: &Path) -> String {
    format!("sqlite:{}?mode=rwc", db_path.display())
}

/// The instance of one run. A generation keeps a restarted run apart from the
/// run it replaced, and a namespace keeps one worktree's runs apart from
/// another's in the shared store.
pub(crate) fn instance_id(namespace: &str, command: &str, generation: u64) -> String {
    let clean = |part: &str| {
        part.chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                    ch
                } else {
                    '-'
                }
            })
            .collect::<String>()
    };
    format!("{}::{}::{}", clean(namespace), clean(command), generation)
}

/// A Tokio runtime that does not exist until it is asked for.
///
/// Building one is the only Tokio work this module ever does, and it happens
/// inside [`LazyTokio::block_on`] or [`LazyTokio::runtime`], never at
/// construction. The hook and every other hot path can hold one of these, or
/// never name it, at no cost.
pub(crate) struct LazyTokio {
    runtime: Option<tokio::runtime::Runtime>,
}

impl fmt::Debug for LazyTokio {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LazyTokio")
            .field("booted", &self.runtime.is_some())
            .finish()
    }
}

impl LazyTokio {
    pub(crate) fn new() -> Self {
        Self { runtime: None }
    }

    /// The runtime, built on this first call.
    pub(crate) fn runtime(&mut self) -> Result<&tokio::runtime::Runtime> {
        if self.runtime.is_none() {
            let built = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("cannot build the Tokio runtime of a ledger run")?;
            self.runtime = Some(built);
        }
        Ok(self
            .runtime
            .as_ref()
            .expect("the runtime was just built or already held"))
    }

    /// Runs `future` on the runtime, building it on this first call.
    pub(crate) fn block_on<F: std::future::Future>(&mut self, future: F) -> Result<F::Output> {
        let runtime = self.runtime()?;
        Ok(runtime.block_on(future))
    }
}

/// Opens the SQLite store of a repository. This only opens the file. No
/// runtime reads or writes it until one is booted over it.
pub(crate) async fn open_sqlite_store(db_path: &Path) -> Result<Arc<SqliteProvider>> {
    let provider = SqliteProvider::new(&sqlite_url(db_path), None)
        .await
        .with_context(|| format!("cannot open {}", db_path.display()))?;
    Ok(Arc::new(provider))
}

/// The orchestration registry: one static list, no rule or workspace loading.
pub(crate) fn orchestration_registry() -> OrchestrationRegistry {
    OrchestrationRegistry::builder()
        .register(ORCHESTRATION_NAME, |ctx, input: String| async move {
            ledger_run(ctx, input).await
        })
        .build()
}

/// Boots the embedded runtime that serves one mutating command, over the
/// store the caller opened. Activities are the caller's list, kept static.
pub(crate) async fn start_local_runtime(
    store: Arc<SqliteProvider>,
    activities: ActivityRegistry,
) -> Arc<Runtime> {
    Runtime::start_with_store(store, activities, orchestration_registry()).await
}

/// The control plane of one run: a client and the instance it speaks for.
///
/// The `Debug` view names only the instance, so no store detail leaks into a
/// log line.
pub(crate) struct LedgerOrchestratorClient {
    client: Client,
    instance_id: String,
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "the client view names only its instance by design"
)]
impl fmt::Debug for LedgerOrchestratorClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LedgerOrchestratorClient")
            .field("instance_id", &self.instance_id)
            .finish()
    }
}

impl LedgerOrchestratorClient {
    pub(crate) fn new(store: Arc<dyn Provider>, instance_id: String) -> Self {
        Self::of(Client::new(store), instance_id)
    }

    /// The control plane of `instance_id` through a client that already exists, such as the
    /// one an activity gets from its context.
    pub(crate) fn of(client: Client, instance_id: String) -> Self {
        Self {
            client,
            instance_id,
        }
    }

    /// Starts the run and returns at once. The start is done when the receipt
    /// of its command id reads back through [`Self::wait_receipt`].
    pub(crate) async fn start_run(&self, input: &RunStartInput) -> Result<()> {
        let payload = serde_json::to_string(input).context("cannot encode the run start")?;
        self.client
            .start_orchestration(&self.instance_id, ORCHESTRATION_NAME, payload)
            .await
            .context("cannot start the ledger run")?;
        Ok(())
    }

    /// Puts one command on the durable queue, behind every command before it.
    pub(crate) async fn enqueue_command(&self, envelope: &CommandEnvelope) -> Result<()> {
        self.client
            .enqueue_event_typed(&self.instance_id, COMMAND_QUEUE, envelope)
            .await
            .context("cannot queue the ledger command")?;
        Ok(())
    }

    /// The receipt of one command, once the orchestration has filed it.
    pub(crate) async fn wait_receipt(
        &self,
        command_id: &str,
        timeout: Duration,
    ) -> Result<Receipt> {
        let receipt = self
            .client
            .wait_for_kv_value_typed::<Receipt>(
                &self.instance_id,
                &receipt_key(command_id),
                timeout,
            )
            .await
            .with_context(|| format!("no receipt for command {command_id}"))?;
        Ok(receipt)
    }

    /// The receipt of one command when it is already filed, without waiting.
    pub(crate) async fn receipt(&self, command_id: &str) -> Result<Option<Receipt>> {
        let receipt = self
            .client
            .get_kv_value_typed::<Receipt>(&self.instance_id, &receipt_key(command_id))
            .await
            .context("cannot read the command receipt")?;
        Ok(receipt)
    }

    /// The raw status of the instance. This reads the store alone and boots
    /// no runtime, which is what `status` is.
    pub(crate) async fn status(&self) -> Result<OrchestrationStatus> {
        let status = self
            .client
            .get_orchestration_status(&self.instance_id)
            .await
            .context("cannot read the run status")?;
        Ok(status)
    }

    /// The mirrored snapshot of the run, from `custom_status` while it runs
    /// and from its output once it has finished.
    pub(crate) async fn snapshot(&self) -> Result<Option<Snapshot>> {
        let text = match self.status().await? {
            OrchestrationStatus::NotFound => return Ok(None),
            OrchestrationStatus::Running { custom_status, .. } => custom_status,
            OrchestrationStatus::Completed {
                output,
                custom_status,
                ..
            } => Some(output)
                .filter(|output| !output.is_empty())
                .or(custom_status),
            OrchestrationStatus::Failed { details, .. } => {
                anyhow::bail!("the ledger run failed: {details:?}")
            }
        };
        let Some(text) = text else {
            return Ok(None);
        };
        let snapshot =
            serde_json::from_str(&text).context("cannot read the mirrored run snapshot")?;
        Ok(Some(snapshot))
    }

    /// Cancels the instance, as a restart does with the run it replaces.
    pub(crate) async fn cancel(&self, reason: &str) -> Result<()> {
        self.client
            .cancel_instance(&self.instance_id, reason)
            .await
            .context("cannot cancel the ledger run")?;
        Ok(())
    }
}

/// One mutating command, end to end, on a runtime built only for it.
///
/// This is the lazy boot in full: the Tokio runtime, the store and the
/// embedded `duroxide` runtime all come up here, the command waits for its
/// own receipt, and both runtimes shut down before the snapshot returns. A
/// receipt that says the command was rejected still returns its snapshot, so
/// the caller can print the stage that comes next and choose its exit code.
pub(crate) fn run_command(
    config: &ProviderConfig,
    instance_id: &str,
    activities: ActivityRegistry,
    envelope: &CommandEnvelope,
    timeout: Duration,
) -> Result<(Snapshot, Receipt)> {
    let db_path = match config {
        ProviderConfig::Sqlite { db_path } => db_path.clone(),
        #[cfg(feature = "ledger-distributed")]
        ProviderConfig::Postgres { .. } => {
            return send_remote(
                config,
                instance_id,
                Sent::Command(envelope.clone()),
                timeout,
            );
        }
    };
    let instance_id = instance_id.to_owned();
    let envelope = envelope.clone();
    let mut lazy = LazyTokio::new();
    let (snapshot, receipt) = lazy.block_on(async move {
        let store = open_sqlite_store(&db_path).await?;
        let runtime = start_local_runtime(Arc::clone(&store), activities).await;
        let client = LedgerOrchestratorClient::new(store, instance_id);
        let sent = client.enqueue_command(&envelope).await;
        let waited = match sent {
            Ok(()) => client.wait_receipt(&envelope.command_id, timeout).await,
            Err(error) => Err(error),
        };
        runtime.shutdown(Some(1_000)).await;
        let receipt = waited?;
        let snapshot = client
            .snapshot()
            .await?
            .context("the run has no mirrored snapshot after its command")?;
        Ok::<(Snapshot, Receipt), anyhow::Error>((snapshot, receipt))
    })??;
    Ok((snapshot, receipt))
}

/// Starts a run on a runtime built only for it, and waits for its receipt.
pub(crate) fn start_command(
    config: &ProviderConfig,
    instance_id: &str,
    activities: ActivityRegistry,
    input: &RunStartInput,
    timeout: Duration,
) -> Result<(Snapshot, Receipt)> {
    let db_path = match config {
        ProviderConfig::Sqlite { db_path } => db_path.clone(),
        #[cfg(feature = "ledger-distributed")]
        ProviderConfig::Postgres { .. } => {
            return send_remote(
                config,
                instance_id,
                Sent::Start(Box::new(input.clone())),
                timeout,
            );
        }
    };
    let instance_id = instance_id.to_owned();
    let input = input.clone();
    let mut lazy = LazyTokio::new();
    let (snapshot, receipt) = lazy.block_on(async move {
        let store = open_sqlite_store(&db_path).await?;
        let runtime = start_local_runtime(Arc::clone(&store), activities).await;
        let client = LedgerOrchestratorClient::new(store, instance_id);
        let started = client.start_run(&input).await;
        let waited = match started {
            Ok(()) => client.wait_receipt(&input.command_id, timeout).await,
            Err(error) => Err(error),
        };
        runtime.shutdown(Some(1_000)).await;
        let receipt = waited?;
        let snapshot = client
            .snapshot()
            .await?
            .context("the run has no mirrored snapshot after its start")?;
        Ok::<(Snapshot, Receipt), anyhow::Error>((snapshot, receipt))
    })??;
    Ok((snapshot, receipt))
}

/// How long the CLI waits for the receipt of a command that workers serve, at the least. A
/// gate waits for a free worker and for its checks, and a worker that dies holds its lease
/// until the lock times out.
#[cfg(feature = "ledger-distributed")]
const REMOTE_WAIT: Duration = Duration::from_secs(3600);

/// What the CLI sends to a run that workers serve.
#[cfg(feature = "ledger-distributed")]
enum Sent {
    Start(Box<RunStartInput>),
    Command(CommandEnvelope),
}

/// Sends a start or a command to a run in the PostgreSQL store, and waits for its receipt.
///
/// No runtime boots here: the workers run every activity. Before the send, the commits the
/// workers need go to the run's branches on the remote, and a fix that was made before the
/// commit the run stands at is rebased onto it first.
#[cfg(feature = "ledger-distributed")]
fn send_remote(
    config: &ProviderConfig,
    instance_id: &str,
    sent: Sent,
    timeout: Duration,
) -> Result<(Snapshot, Receipt)> {
    let ProviderConfig::Postgres { remote, .. } = config else {
        anyhow::bail!("only the PostgreSQL store is served by workers")
    };
    let config = config.clone();
    let remote = remote.clone();
    let instance_id = instance_id.to_owned();
    let mut lazy = LazyTokio::new();
    lazy.block_on(async move {
        let store = open_store(&config).await?;
        let client = LedgerOrchestratorClient::new(store, instance_id.clone());
        let branch = run_branch(&instance_id);
        let command_id = match sent {
            Sent::Start(input) => {
                publish::start(&remote, &branch)?;
                client.start_run(&input).await?;
                input.command_id
            }
            Sent::Command(mut envelope) => {
                if let Command::Gate { fix, head } = &mut envelope.command {
                    let current = (client.snapshot().await?)
                        .context("the run has not published its state yet")?
                        .run
                        .current;
                    *head = publish::fix(&remote, &branch, fix, head, &current)?;
                } else {
                    publish::head(&remote, &branch)?;
                }
                client.enqueue_command(&envelope).await?;
                envelope.command_id
            }
        };
        let receipt = client
            .wait_receipt(&command_id, timeout.max(REMOTE_WAIT))
            .await?;
        let snapshot = client
            .snapshot()
            .await?
            .context("the run has no mirrored snapshot after its command")?;
        Ok::<(Snapshot, Receipt), anyhow::Error>((snapshot, receipt))
    })?
}

/// The commits of a run on the git remote, published from the checkout of the CLI, which
/// `Place::find` made the folder of the process.
#[cfg(feature = "ledger-distributed")]
mod publish {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use anyhow::{Context, Result, bail};

    use crate::git;

    fn here() -> &'static Path {
        Path::new(".")
    }

    fn push(remote: &str, refspecs: &[String]) -> Result<()> {
        let mut args = vec!["push", "--quiet", "--force", remote];
        args.extend(refspecs.iter().map(String::as_str));
        git::output(here(), &args)
            .with_context(|| format!("cannot push the commits of the run to {remote}"))?;
        Ok(())
    }

    /// Publishes the commit a run starts from as its `base` and its `head`.
    pub(super) fn start(remote: &str, branch: &str) -> Result<()> {
        push(
            remote,
            &[
                format!("HEAD:refs/heads/{branch}/base"),
                format!("HEAD:refs/heads/{branch}/head"),
            ],
        )
    }

    /// Publishes the commit of the checkout as the `head` of the run.
    pub(super) fn head(remote: &str, branch: &str) -> Result<()> {
        push(remote, &[format!("HEAD:refs/heads/{branch}/head")])
    }

    /// Publishes the fix `fix`, made at `head`, as the branch `fix/<fix>` of the run, and
    /// returns the commit to gate. That is `head` when it stands on `current`, the commit the
    /// run stands at. A fix made before an earlier fix was kept is rebased onto `current` in
    /// a worktree of its own, and the rebased commit is the one the gate measures.
    pub(super) fn fix(
        remote: &str,
        branch: &str,
        fix: &str,
        head: &str,
        current: &str,
    ) -> Result<String> {
        git::output(
            here(),
            &[
                "fetch",
                "--quiet",
                remote,
                &format!("+refs/heads/{branch}/*:refs/remotes/{remote}/{branch}/*"),
            ],
        )
        .with_context(|| format!("cannot fetch the run's branches from {remote}"))?;
        let gated = if is_ancestor(current, head)? {
            head.to_owned()
        } else {
            let rebased = rebase(head, current).with_context(|| {
                format!(
                    "the fix {fix} does not rebase cleanly onto {current}, where the run stands. Rebase it yourself and gate it again"
                )
            })?;
            eprintln!(
                "xpd: the fix {fix} was made before {current}, where the run stands, so it was rebased onto it as {rebased}. Check that commit out with `git reset --hard {rebased}`"
            );
            rebased
        };
        push(
            remote,
            &[
                format!("{gated}:refs/heads/{branch}/fix/{fix}"),
                format!("{gated}:refs/heads/{branch}/head"),
            ],
        )?;
        Ok(gated)
    }

    fn is_ancestor(ancestor: &str, commit: &str) -> Result<bool> {
        let status = Command::new("git")
            .args(["merge-base", "--is-ancestor", ancestor, commit])
            .stdin(Stdio::null())
            .status()
            .context("cannot run git")?;
        match status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => bail!("git cannot compare {ancestor} with {commit}"),
        }
    }

    /// A worktree of the checkout's repository, removed when it is dropped.
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let path = self.0.to_string_lossy().into_owned();
            let _ = git::output(here(), &["worktree", "remove", "--force", &path]);
        }
    }

    /// `head` rebased onto `onto`, in a detached worktree, so the checkout stays as it is.
    fn rebase(head: &str, onto: &str) -> Result<String> {
        let name = format!("xpd-rebase-{}", std::process::id());
        let path = git::output(here(), &["rev-parse", "--git-path", &name])?;
        let path = PathBuf::from(path.trim());
        let shown = path.to_string_lossy().into_owned();
        git::output(
            here(),
            &["worktree", "add", "--quiet", "--detach", &shown, head],
        )?;
        let scratch = Scratch(path);
        if let Err(error) = git::output(&scratch.0, &["rebase", "--quiet", onto]) {
            let _ = git::output(&scratch.0, &["rebase", "--abort"]);
            return Err(error);
        }
        Ok(git::output(&scratch.0, &["rev-parse", "HEAD"])?
            .trim()
            .to_owned())
    }
}
