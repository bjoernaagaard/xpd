//! A local Duroxide instance owns one animation from its first preview to its approved final.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand};
use duroxide::providers::{Provider, sqlite::SqliteProvider};
use duroxide::runtime::Runtime;
use duroxide::runtime::registry::{ActivityRegistry, OrchestrationRegistry};
use duroxide::{Client, EventKind, OrchestrationContext, OrchestrationStatus};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::animate::runner;

const ORCHESTRATION: &str = "xpd.animate.workflow";
const RENDER: &str = "xpd.animate.render";
const DECISIONS: &str = "xpd.animate.decisions";

#[derive(Clone, Debug, Args, Serialize, Deserialize)]
pub(crate) struct SceneFiles {
    /// Project-relative Manim scene file.
    file: PathBuf,
    /// Scene class to render.
    scene: String,
    /// Additional project-relative file to freeze with the scene.
    #[arg(long = "asset")]
    assets: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Workflow {
    /// Freeze a scene and render the first preview of a durable animation.
    Start {
        #[command(flatten)]
        source: SceneFiles,
        #[arg(long)]
        json: bool,
    },
    /// Freeze the edited scene and render a new preview, retaining the review note.
    Revise {
        run: Uuid,
        #[arg(long)]
        revision: usize,
        #[arg(long)]
        note: String,
        #[arg(long)]
        json: bool,
    },
    /// Approve a reviewed revision and render its frozen source at final quality.
    Approve {
        run: Uuid,
        #[arg(long)]
        revision: usize,
        #[arg(long)]
        json: bool,
    },
    /// Show the animation stage, revision history and artifacts.
    Status {
        run: Uuid,
        #[arg(long)]
        json: bool,
    },
    /// Recover an interrupted render or retry a failed render using its frozen inputs.
    Resume {
        run: Uuid,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SavedScene {
    revision: usize,
    snapshot: String,
    note: Option<String>,
    preview: Option<Value>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Previewing,
    PreviewFailed,
    Review,
    Finalizing,
    FinalFailed,
    Complete,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnimationRun {
    run: String,
    source: SceneFiles,
    revision: usize,
    stage: Stage,
    history: Vec<SavedScene>,
    approved_revision: Option<usize>,
    #[serde(rename = "final")]
    final_report: Option<Value>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Start {
    source: SceneFiles,
    snapshot: String,
    receipt: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Decision {
    Revise {
        revision: usize,
        snapshot: String,
        note: String,
    },
    Approve {
        revision: usize,
    },
    Resume,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    receipt: String,
    decision: Decision,
}

#[derive(Debug, Serialize, Deserialize)]
struct RenderInput {
    snapshot: String,
    profile: String,
    job_id: String,
}

fn print_artifacts(report: &Value) {
    for (key, label) in [
        ("video", "Video"),
        ("contact_sheet", "Contact sheet"),
        ("source", "Source"),
        ("manifest", "Manifest"),
        ("log", "Render log"),
    ] {
        if let Some(path) = report[key].as_str() {
            println!("  {label}: {path}");
        }
    }
}

impl AnimationRun {
    fn print(&self) {
        let stage = match self.stage {
            Stage::Previewing => "Rendering preview",
            Stage::PreviewFailed => "Preview failed",
            Stage::Review => "Awaiting review",
            Stage::Finalizing => "Rendering approved revision",
            Stage::FinalFailed => "Final render failed",
            Stage::Complete => "Complete",
        };
        println!(
            "Animation {} (revision {}, {stage})",
            self.run, self.revision
        );
        for revision in &self.history {
            println!("Preview for revision {}", revision.revision);
            if let Some(note) = &revision.note {
                println!("  Review note: {note}");
            }
            if let Some(error) = &revision.error {
                println!("  {error}");
            }
            if let Some(report) = &revision.preview {
                print_artifacts(report);
            }
        }
        if let Some(revision) = self.approved_revision {
            println!("Approved final for revision {revision}");
            if let Some(report) = &self.final_report {
                print_artifacts(report);
            }
            if let Some(error) = &self.error {
                println!("  {error}");
            }
        }
    }

    fn append(&mut self, snapshot: String, note: Option<String>) {
        self.revision += 1;
        self.history.clear();
        self.history.push(SavedScene {
            revision: self.revision,
            snapshot,
            note,
            preview: None,
            error: None,
        });
    }

    fn check_revision(&self, revision: usize) -> Result<()> {
        ensure!(
            self.revision == revision,
            "requested revision {revision}, current revision is {}",
            self.revision
        );
        Ok(())
    }

    async fn render(&mut self, ctx: &OrchestrationContext) -> Result<()> {
        let (profile, pending, failed) = if self.approved_revision.is_some() {
            ("final", Stage::Finalizing, Stage::FinalFailed)
        } else {
            ("preview", Stage::Previewing, Stage::PreviewFailed)
        };
        self.error = None;
        self.stage = pending;
        ctx.set_custom_status(serde_json::to_string(self)?);
        let revision = self
            .history
            .last_mut()
            .context("animation has no saved revision")?;
        let digest = Sha256::digest(format!("{}/{}/{profile}", self.run, revision.revision));
        let input = RenderInput {
            snapshot: revision.snapshot.clone(),
            profile: profile.into(),
            job_id: format!("{digest:x}")[..32].to_owned(),
        };
        match ctx
            .schedule_activity_typed::<_, Value>(RENDER, &input)
            .await
        {
            Ok(report) if self.approved_revision.is_some() => {
                self.final_report = Some(report);
                self.stage = Stage::Complete;
            }
            Ok(report) => {
                revision.preview = Some(report);
                revision.error = None;
                self.stage = Stage::Review;
            }
            Err(error) => {
                self.stage = failed;
                if failed == Stage::PreviewFailed {
                    revision.error = Some(error.clone());
                }
                self.error = Some(error);
            }
        }
        Ok(())
    }

    async fn apply(&mut self, ctx: &OrchestrationContext, decision: Decision) -> Result<()> {
        match decision {
            Decision::Revise {
                revision,
                snapshot,
                note,
            } => {
                self.check_revision(revision)?;
                ensure!(
                    matches!(self.stage, Stage::Review | Stage::PreviewFailed),
                    "only an unapproved preview can be revised"
                );
                self.append(snapshot, Some(note));
            }
            Decision::Approve { revision } => {
                self.check_revision(revision)?;
                ensure!(
                    self.stage == Stage::Review,
                    "approval requires a successful preview"
                );
                self.approved_revision = Some(revision);
            }
            Decision::Resume if self.stage == Stage::Review => return Ok(()),
            Decision::Resume => {}
        }
        self.render(ctx).await
    }
}

async fn animate(ctx: OrchestrationContext, start: Start) -> Result<AnimationRun, String> {
    let mut state = AnimationRun {
        run: ctx.instance_id(),
        source: start.source,
        revision: 0,
        stage: Stage::Previewing,
        history: Vec::new(),
        approved_revision: None,
        final_report: None,
        error: None,
    };
    state.append(start.snapshot, None);
    state.render(&ctx).await.map_err(|e| e.to_string())?;
    let mut receipt = start.receipt;
    let mut rejection = None::<String>;
    loop {
        ctx.set_custom_status(serde_json::to_string(&state).map_err(|e| e.to_string())?);
        ctx.set_kv_value_typed(&receipt, &rejection);
        if state.stage == Stage::Complete {
            return Ok(state);
        }
        let raw = ctx.dequeue_event(DECISIONS).await;
        let request: Request = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        ctx.clear_kv_value(receipt);
        receipt = request.receipt;
        rejection = state
            .apply(&ctx, request.decision)
            .await
            .err()
            .map(|e| e.to_string());
    }
}

fn decode_output<T: DeserializeOwned>(output: &Output) -> Result<T> {
    ensure!(
        output.status.success(),
        "animation renderer failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout).context("invalid animation renderer response")
}

#[derive(Deserialize)]
struct Frozen {
    snapshot: String,
}

impl SceneFiles {
    fn snapshot(&self) -> Result<String> {
        ensure!(
            self.file != Path::new("-"),
            "a revisable animation needs a project-relative scene file"
        );
        let mut command = runner();
        command.arg("snapshot");
        for asset in &self.assets {
            command.arg("--asset").arg(asset);
        }
        let output = command
            .arg("--")
            .arg(&self.file)
            .arg(&self.scene)
            .output()?;
        Ok(decode_output::<Frozen>(&output)?.snapshot)
    }
}

fn render_command(input: &RenderInput) -> Command {
    let mut command = runner();
    command.args([
        "render-snapshot",
        &input.snapshot,
        &input.job_id,
        &input.profile,
    ]);
    command
}

async fn with_history(provider: &SqliteProvider, mut state: AnimationRun) -> Result<AnimationRun> {
    let mut revisions = BTreeMap::new();
    for event in provider.read(&state.run).await? {
        if let EventKind::CustomStatusUpdated {
            status: Some(status),
        } = event.kind
        {
            let saved: AnimationRun = serde_json::from_str(&status)?;
            if saved.revision < state.revision {
                for scene in saved.history {
                    revisions.insert(scene.revision, scene);
                }
            }
        }
    }
    for scene in state.history {
        revisions.insert(scene.revision, scene);
    }
    state.history = revisions.into_values().collect();
    Ok(state)
}

async fn wait_for_receipt(
    client: &Client,
    run: &str,
    receipt: &str,
    command: &Workflow,
) -> Result<AnimationRun> {
    let timeout = Duration::from_secs(3600);
    tokio::select! {
        biased;
        reply = client.wait_for_kv_value_typed::<Option<String>>(run, receipt, timeout) => {
            if let Some(rejected) = reply? { bail!("{rejected}"); }
            read_state(client.get_orchestration_status(run).await?)?.context("missing animation state after receipt")
        },
        done = client.wait_for_orchestration(run, timeout) => {
            let completed = read_state(done?)?.context("missing completed animation")?;
            ensure!(!matches!(command, Workflow::Revise { .. }), "an approved animation cannot be revised");
            if let Workflow::Approve { revision, .. } = command { completed.check_revision(*revision)?; }
            Ok(completed)
        },
    }
}

async fn worker(provider: Arc<SqliteProvider>) -> Arc<Runtime> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let activities = ActivityRegistry::builder()
        .register_typed(RENDER, |_, input: RenderInput| async move {
            let output = tokio::process::Command::from(render_command(&input))
                .kill_on_drop(true)
                .output()
                .await
                .map_err(|e| e.to_string())?;
            decode_output::<Value>(&output).map_err(|e| e.to_string())
        })
        .build();
    let orchestrations = OrchestrationRegistry::builder()
        .register_typed(ORCHESTRATION, animate)
        .build();
    Runtime::start_with_store(provider, activities, orchestrations).await
}

fn read_state(status: OrchestrationStatus) -> Result<Option<AnimationRun>> {
    match status {
        OrchestrationStatus::Failed { details, .. } => {
            bail!("animation workflow failed: {details:?}")
        }
        OrchestrationStatus::Completed { output, .. }
        | OrchestrationStatus::Running {
            custom_status: Some(output),
            ..
        } => Ok(Some(serde_json::from_str(&output)?)),
        _ => Ok(None),
    }
}

fn store_path(run: &str, action: &Workflow) -> Result<PathBuf> {
    let create = matches!(action, Workflow::Start { .. });
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env::home_dir().map(|p| p.join(".local/state")))
        .context("cannot find the user state directory for animation runs")?;
    let project = env::current_dir()?.canonicalize()?;
    let namespace = Sha256::digest(project.as_os_str().as_encoded_bytes());
    let runs = state.join("xpd/animate").join(format!("{namespace:x}"));
    if create {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.recursive(true).create(&runs)?;
    }
    let path = runs.join(format!("{run}.sqlite"));
    ensure!(
        !path.is_symlink(),
        "invalid animation database {}",
        path.display()
    );
    ensure!(create || path.is_file(), "unknown animation run {run}");
    Ok(path)
}

impl Workflow {
    pub(crate) fn run(self) -> Result<ExitCode> {
        let (run, json) = match &self {
            Self::Start { json, .. } => (Uuid::new_v4(), *json),
            Self::Revise { run, json, .. }
            | Self::Approve { run, json, .. }
            | Self::Status { run, json }
            | Self::Resume { run, json } => (*run, *json),
        };
        let run = run.simple().to_string();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let observing = matches!(self, Self::Status { .. });
        let state = runtime.block_on(self.execute(&run))?;
        if json {
            println!("{}", serde_json::to_string(&state)?);
        } else {
            state.print();
        }
        Ok(if !observing && state.error.is_some() {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        })
    }

    async fn execute(self, run: &str) -> Result<AnimationRun> {
        let starting = matches!(self, Self::Start { .. });
        let frozen = match &self {
            Self::Start { source, .. } => Some(source.snapshot()?),
            _ => None,
        };
        let path = store_path(run, &self)?;
        let _writer = if matches!(self, Self::Status { .. }) {
            None
        } else {
            let file = fs::File::options()
                .create(true)
                .truncate(false)
                .write(true)
                .open(path.with_extension("lock"))?;
            file.try_lock()
                .with_context(|| format!("another command is changing animation {run}"))?;
            Some(file)
        };
        let url = format!(
            "sqlite:{}?mode={}",
            path.to_str()
                .context("the animation state path must be UTF-8")?
                .replace('%', "%25")
                .replace('?', "%3F"),
            if starting { "rwc" } else { "rw" }
        );
        let provider = Arc::new(SqliteProvider::new(&url, None).await?);
        let client = Client::new(Arc::<SqliteProvider>::clone(&provider));
        let state = read_state(client.get_orchestration_status(run).await?)?;
        if matches!(self, Self::Status { .. }) {
            return with_history(
                &provider,
                state.context("animation start is queued; run resume to advance it")?,
            )
            .await;
        }
        if let Some(state) = state.as_ref() {
            match &self {
                Self::Revise { revision, .. } | Self::Approve { revision, .. } => {
                    state.check_revision(*revision)?;
                }
                _ => {}
            }
        }
        if state.as_ref().is_some_and(|s| s.stage == Stage::Complete) {
            ensure!(
                !matches!(self, Self::Revise { .. }),
                "an approved animation cannot be revised"
            );
            return with_history(&provider, state.context("missing completed animation")?).await;
        }
        let receipt = Uuid::new_v4().simple().to_string();
        self.submit(&client, run, &receipt, frozen, state.as_ref())
            .await?;
        let runtime = worker(Arc::clone(&provider)).await;
        let result = wait_for_receipt(&client, run, &receipt, &self).await;
        runtime.shutdown(Some(1000)).await;
        let state = result.with_context(|| format!("animation run {run}"))?;
        with_history(&provider, state).await
    }

    async fn submit(
        &self,
        client: &Client,
        run: &str,
        receipt: &str,
        frozen: Option<String>,
        state: Option<&AnimationRun>,
    ) -> Result<()> {
        match self {
            Self::Start { source, .. } => {
                let snapshot = frozen.context("missing initial scene snapshot")?;
                client
                    .start_orchestration_typed(
                        run,
                        ORCHESTRATION,
                        &Start {
                            source: source.clone(),
                            snapshot,
                            receipt: receipt.to_owned(),
                        },
                    )
                    .await?;
                eprintln!("Animation run {run}");
            }
            action => {
                let decision = match action {
                    Self::Revise { revision, note, .. } => {
                        ensure!(!note.trim().is_empty(), "a revision needs a review note");
                        let state = state.context("the first preview is pending")?;
                        ensure!(
                            matches!(state.stage, Stage::Review | Stage::PreviewFailed),
                            "only an unapproved preview can be revised"
                        );
                        Decision::Revise {
                            revision: *revision,
                            snapshot: state.source.snapshot()?,
                            note: note.clone(),
                        }
                    }
                    Self::Approve { revision, .. } => {
                        ensure!(
                            state.as_ref().is_some_and(|s| s.stage == Stage::Review),
                            "approval requires a successful preview awaiting review"
                        );
                        Decision::Approve {
                            revision: *revision,
                        }
                    }
                    Self::Resume { .. } => Decision::Resume,
                    _ => unreachable!("start and status are handled separately"),
                };
                client
                    .enqueue_event_typed(
                        run,
                        DECISIONS,
                        &Request {
                            receipt: receipt.to_owned(),
                            decision,
                        },
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
