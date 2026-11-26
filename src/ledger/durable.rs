//! The adapter between a ledger run and `duroxide`.
//!
//! Everything `duroxide` knows about lives behind this module and
//! [`super::orchestration`]. A caller opens one SQLite store in the common git
//! folder, boots one embedded runtime for a mutating command, and reads
//! `status` back through a client alone. No runtime, and no Tokio runtime,
//! exists until a caller asks for one: [`LazyTokio`] holds the runtime it
//! builds on first use and nothing else in this module builds one, so a hot
//! path that never names this module pays nothing for it.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use duroxide::providers::sqlite::SqliteProvider;
use duroxide::runtime::Runtime;
use duroxide::runtime::registry::{ActivityRegistry, OrchestrationRegistry};
use duroxide::{Client, OrchestrationStatus};

use super::orchestration::{
    COMMAND_QUEUE, CommandEnvelope, ORCHESTRATION_NAME, Receipt, RunStartInput, Snapshot,
    ledger_run, receipt_key,
};

/// The store a run lives in, chosen by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "wired into run::act in the next slice")]
pub(crate) enum ProviderConfig {
    /// Local default: `<git-common-dir>/xpd-ledger.sqlite`, shared by worktrees.
    Sqlite { db_path: PathBuf },
    /// Horizontal mode, compiled only with `ledger-distributed`.
    #[cfg(feature = "ledger-distributed")]
    Postgres {
        connection_url: String,
        schema: Option<String>,
    },
}

impl ProviderConfig {
    /// The local store inside `git_common_dir`.
    #[expect(dead_code, reason = "wired into run::act in the next slice")]
    pub(crate) fn sqlite_in(git_common_dir: &Path) -> Self {
        Self::Sqlite {
            db_path: git_common_dir.join("xpd-ledger.sqlite"),
        }
    }
}

/// The `sqlite:` URL of a store file, created when it is missing.
pub(crate) fn sqlite_url(db_path: &Path) -> String {
    format!("sqlite:{}?mode=rwc", db_path.display())
}

/// The instance of one run. A generation keeps a restarted run apart from the
/// run it replaced, and a namespace keeps one worktree's runs apart from
/// another's in the shared store.
#[expect(dead_code, reason = "wired into run::act in the next slice")]
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

    /// Whether the runtime has been built yet.
    #[expect(
        dead_code,
        reason = "the proof of lazy boot reads it before the next slice wires it"
    )]
    pub(crate) fn is_booted(&self) -> bool {
        self.runtime.is_some()
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

#[expect(dead_code, reason = "wired into run::act in the next slice")]
impl LedgerOrchestratorClient {
    pub(crate) fn new(store: Arc<SqliteProvider>, instance_id: String) -> Self {
        Self {
            client: Client::new(store),
            instance_id,
        }
    }

    pub(crate) fn instance_id(&self) -> &str {
        &self.instance_id
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

    /// Waits for the mirror to move past `last_seen_version`.
    pub(crate) async fn wait_for_status_change(
        &self,
        last_seen_version: u64,
        timeout: Duration,
    ) -> Result<OrchestrationStatus> {
        let status = self
            .client
            .wait_for_status_change(
                &self.instance_id,
                last_seen_version,
                Duration::from_millis(25),
                timeout,
            )
            .await
            .context("the run status did not change in time")?;
        Ok(status)
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
#[expect(dead_code, reason = "wired into run::act in the next slice")]
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
            anyhow::bail!("only the local SQLite store serves this call")
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
#[expect(dead_code, reason = "wired into run::act in the next slice")]
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
            anyhow::bail!("only the local SQLite store serves this call")
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

/// The snapshot a `status` call prints, read with a client alone.
///
/// The Tokio runtime here serves only the store read. No `duroxide` runtime
/// boots, no activity runs, and no command is queued.
#[expect(dead_code, reason = "wired into run::act in the next slice")]
pub(crate) fn read_status(config: &ProviderConfig, instance_id: &str) -> Result<Option<Snapshot>> {
    let db_path = match config {
        ProviderConfig::Sqlite { db_path } => db_path.clone(),
        #[cfg(feature = "ledger-distributed")]
        ProviderConfig::Postgres { .. } => {
            anyhow::bail!("only the local SQLite store serves this call")
        }
    };
    let instance_id = instance_id.to_owned();
    let mut lazy = LazyTokio::new();
    let snapshot = lazy.block_on(async move {
        let store = open_sqlite_store(&db_path).await?;
        let client = LedgerOrchestratorClient::new(store, instance_id);
        client.snapshot().await
    })??;
    Ok(snapshot)
}
