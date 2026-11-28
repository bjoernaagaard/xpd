//! Local Duroxide orchestration for `xpd gates`.
//!
//! Each run lives in `$XDG_STATE_HOME/xpd/gates/<project-sha256>/<run>.sqlite`. Read-only
//! activities (`profile`, `check`, `graph`, `refit`) fan out concurrently in Wave 1 via
//! `ctx.join`, and the two `equiv` activities (`reproducible`, `served`) run in Wave 2 so
//! `train.py` never races against `refit` or itself on saved model files. Content hashes of
//! each activity's inputs let `retry` skip unchanged activities and let `resume` finish an
//! interrupted round without rerunning completed activities.

use std::env;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use duroxide::providers::sqlite::SqliteProvider;
use duroxide::runtime::registry::{ActivityRegistry, OrchestrationRegistry};
use duroxide::runtime::{Runtime, RuntimeOptions};
use duroxide::{Client, OrchestrationContext, OrchestrationStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::catalog::Filters;
use crate::check::FixMode;
use crate::cli::{EQUIV_PY, PROFILE_PY, REFIT_PY};
use crate::fix_safety::Applicability;
use crate::graph::Graph;
use crate::output::Format;
use crate::venv;
use crate::workspace::Workspace;

use super::{
    CheckIssue, CheckReport, Contract, DiscoveryProof, Equivalence, Gate, GateReport, GatesCommand,
    GraphReport, Profile, Proof, Refit, format_gates_text, inside_file, judge, read_contract,
    read_discovery, validate_seed_rules,
};

const ORCHESTRATION: &str = "xpd.gates.workflow";
const DECISIONS: &str = "xpd.gates.decisions";
const ACT_PROFILE: &str = "xpd.gates.profile";
const ACT_CHECK: &str = "xpd.gates.check";
const ACT_GRAPH: &str = "xpd.gates.graph";
const ACT_REFIT: &str = "xpd.gates.refit";
const ACT_EQUIV: &str = "xpd.gates.equiv";

const EXIT_ERROR: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateScripts {
    contract: String,
    train: String,
    predict: String,
    features: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InputHashes {
    profile: String,
    check: String,
    graph: String,
    refit: String,
    reproducible: String,
    served: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GateStage {
    Running,
    Failed,
    Error,
    Passed,
}

impl GateStage {
    fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Failed => "failed",
            Self::Error => "error",
            Self::Passed => "passed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cached<T> {
    hash: String,
    value: T,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct CachedEvidence {
    profile: Option<Cached<Profile>>,
    check: Option<Cached<CheckReport>>,
    graph: Option<Cached<GraphReport>>,
    refit: Option<Cached<Refit>>,
    reproducible: Option<Cached<Equivalence>>,
    served: Option<Cached<Equivalence>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoundRecord {
    round: usize,
    passed: bool,
    failed: Vec<String>,
    executed: Vec<String>,
    reused: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateRunState {
    run: String,
    scripts: GateScripts,
    round: usize,
    stage: GateStage,
    contract: Contract,
    hashes: InputHashes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discovery: Option<DiscoveryProof>,
    cached: CachedEvidence,
    executed: Vec<String>,
    reused: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest: Option<GateReport>,
    history: Vec<RoundRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// JSON output of `xpd gates`, carrying `GateReport` fields at the top level for MCP compatibility.
#[derive(Debug, Serialize)]
struct GateRunOutput<'a> {
    run: &'a str,
    round: usize,
    stage: GateStage,
    passed: bool,
    failed: &'a [String],
    gates: &'a [Gate],
    executed: &'a [String],
    reused: &'a [String],
    history: &'a [RoundRecord],
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

impl GateRunState {
    fn output(&self) -> GateRunOutput<'_> {
        let (passed, failed, gates) = match &self.latest {
            Some(report) => (
                report.passed,
                report.failed.as_slice(),
                report.gates.as_slice(),
            ),
            None => (false, &[][..], &[][..]),
        };
        GateRunOutput {
            run: &self.run,
            round: self.round,
            stage: self.stage,
            passed,
            failed,
            gates,
            executed: &self.executed,
            reused: &self.reused,
            history: &self.history,
            error: self.error.as_deref(),
        }
    }

    fn print(&self, format: Format) -> Result<()> {
        match format {
            Format::Json => println!("{}", serde_json::to_string(&self.output())?),
            Format::Text => print!(
                "{}",
                format_gates_text(
                    &self.run,
                    self.round,
                    self.stage.label(),
                    &self.executed,
                    &self.reused,
                    self.latest.as_ref(),
                    self.error.as_deref(),
                )
            ),
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StartInput {
    scripts: GateScripts,
    contract: Contract,
    hashes: InputHashes,
    discovery: Option<DiscoveryProof>,
    receipt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum CachePolicy {
    Reuse,
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreMode {
    Create,
    Existing,
}

#[derive(Debug, Serialize, Deserialize)]
struct RetryPayload {
    contract: Contract,
    hashes: InputHashes,
    discovery: Option<DiscoveryProof>,
    cache: CachePolicy,
}

#[derive(Debug, Serialize, Deserialize)]
enum Decision {
    Retry(Box<RetryPayload>),
    Resume,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    receipt: String,
    decision: Decision,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileInput {
    file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckInput {
    paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphInput {
    file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefitInput {
    contract: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EquivInput {
    old: String,
    new: String,
}

#[derive(Clone, Copy)]
enum WaveOneSlot {
    Profile,
    Check,
    Graph,
    Refit,
}

impl WaveOneSlot {
    fn name(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Check => "check",
            Self::Graph => "graph",
            Self::Refit => "refit",
        }
    }
}

fn cache_hit<T>(slot: Option<&Cached<T>>, hash: &str, cache: CachePolicy) -> bool {
    cache == CachePolicy::Reuse && slot.is_some_and(|entry| entry.hash == hash)
}

async fn run_wave_one(
    ctx: &OrchestrationContext,
    state: &mut GateRunState,
    cache: CachePolicy,
) -> Result<Option<String>, String> {
    let mut slots = Vec::new();
    let mut futures = Vec::new();

    if cache_hit(state.cached.profile.as_ref(), &state.hashes.profile, cache) {
        state.reused.push("profile".to_owned());
    } else {
        let payload = serde_json::to_string(&ProfileInput {
            file: state.contract.data.file.clone(),
        })
        .map_err(|e| e.to_string())?;
        slots.push(WaveOneSlot::Profile);
        futures.push(ctx.schedule_activity(ACT_PROFILE, payload));
    }

    if cache_hit(state.cached.check.as_ref(), &state.hashes.check, cache) {
        state.reused.push("check".to_owned());
    } else {
        let payload = serde_json::to_string(&CheckInput {
            paths: vec![
                state.scripts.train.clone(),
                state.scripts.predict.clone(),
                state.scripts.features.clone(),
            ],
        })
        .map_err(|e| e.to_string())?;
        slots.push(WaveOneSlot::Check);
        futures.push(ctx.schedule_activity(ACT_CHECK, payload));
    }

    if cache_hit(state.cached.graph.as_ref(), &state.hashes.graph, cache) {
        state.reused.push("graph".to_owned());
    } else {
        let payload = serde_json::to_string(&GraphInput {
            file: state.scripts.features.clone(),
        })
        .map_err(|e| e.to_string())?;
        slots.push(WaveOneSlot::Graph);
        futures.push(ctx.schedule_activity(ACT_GRAPH, payload));
    }

    if cache_hit(state.cached.refit.as_ref(), &state.hashes.refit, cache) {
        state.reused.push("refit".to_owned());
    } else {
        let payload = serde_json::to_string(&RefitInput {
            contract: state.scripts.contract.clone(),
        })
        .map_err(|e| e.to_string())?;
        slots.push(WaveOneSlot::Refit);
        futures.push(ctx.schedule_activity(ACT_REFIT, payload));
    }

    if futures.is_empty() {
        return Ok(None);
    }

    let results = ctx.join(futures).await;
    let mut first_error: Option<String> = None;
    for (slot, result) in slots.into_iter().zip(results) {
        match result {
            Ok(raw) => {
                state.executed.push(slot.name().to_owned());
                store_wave_one_result(state, slot, &raw)?;
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(format!("{}: {error}", slot.name()));
                }
            }
        }
    }
    Ok(first_error)
}

fn store_wave_one_result(
    state: &mut GateRunState,
    slot: WaveOneSlot,
    raw: &str,
) -> Result<(), String> {
    match slot {
        WaveOneSlot::Profile => {
            let value: Profile = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            state.cached.profile = Some(Cached {
                hash: state.hashes.profile.clone(),
                value,
            });
        }
        WaveOneSlot::Check => {
            let value: CheckReport = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            state.cached.check = Some(Cached {
                hash: state.hashes.check.clone(),
                value,
            });
        }
        WaveOneSlot::Graph => {
            let value: GraphReport = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            state.cached.graph = Some(Cached {
                hash: state.hashes.graph.clone(),
                value,
            });
        }
        WaveOneSlot::Refit => {
            let value: Refit = serde_json::from_str(raw).map_err(|e| e.to_string())?;
            state.cached.refit = Some(Cached {
                hash: state.hashes.refit.clone(),
                value,
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum WaveTwoSlot {
    Reproducible,
    Served,
}

async fn run_equiv_slot(
    ctx: &OrchestrationContext,
    state: &mut GateRunState,
    slot: WaveTwoSlot,
    cache: CachePolicy,
) -> Result<Option<String>, String> {
    let (label, hash, input, hit) = match slot {
        WaveTwoSlot::Reproducible => (
            "reproducible",
            state.hashes.reproducible.clone(),
            EquivInput {
                old: state.scripts.train.clone(),
                new: state.scripts.train.clone(),
            },
            cache_hit(
                state.cached.reproducible.as_ref(),
                &state.hashes.reproducible,
                cache,
            ),
        ),
        WaveTwoSlot::Served => (
            "served",
            state.hashes.served.clone(),
            EquivInput {
                old: state.scripts.train.clone(),
                new: state.scripts.predict.clone(),
            },
            cache_hit(state.cached.served.as_ref(), &state.hashes.served, cache),
        ),
    };
    if hit {
        state.reused.push(label.to_owned());
        return Ok(None);
    }
    let payload = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    match ctx.schedule_activity(ACT_EQUIV, payload).await {
        Ok(raw) => {
            let value: Equivalence = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            state.executed.push(label.to_owned());
            let cached = Some(Cached { hash, value });
            match slot {
                WaveTwoSlot::Reproducible => state.cached.reproducible = cached,
                WaveTwoSlot::Served => state.cached.served = cached,
            }
            Ok(None)
        }
        Err(error) => Ok(Some(format!("{label}: {error}"))),
    }
}

async fn run_round(
    ctx: &OrchestrationContext,
    state: &mut GateRunState,
    cache: CachePolicy,
) -> Result<(), String> {
    state.stage = GateStage::Running;
    state.error = None;
    state.executed.clear();
    state.reused.clear();
    ctx.set_custom_status(serde_json::to_string(state).map_err(|e| e.to_string())?);

    if let Some(error) = run_wave_one(ctx, state, cache).await? {
        record_error(state, error);
        return Ok(());
    }

    for slot in [WaveTwoSlot::Reproducible, WaveTwoSlot::Served] {
        if let Some(error) = run_equiv_slot(ctx, state, slot, cache).await? {
            record_error(state, error);
            return Ok(());
        }
    }

    finish_round(state)
}

fn finish_round(state: &mut GateRunState) -> Result<(), String> {
    let profile = &state
        .cached
        .profile
        .as_ref()
        .ok_or_else(|| "missing profile evidence".to_owned())?
        .value;
    let lint = &state
        .cached
        .check
        .as_ref()
        .ok_or_else(|| "missing check evidence".to_owned())?
        .value;
    let dataflow = &state
        .cached
        .graph
        .as_ref()
        .ok_or_else(|| "missing graph evidence".to_owned())?
        .value;
    let refit = &state
        .cached
        .refit
        .as_ref()
        .ok_or_else(|| "missing refit evidence".to_owned())?
        .value;
    let reproducible = &state
        .cached
        .reproducible
        .as_ref()
        .ok_or_else(|| "missing reproducible equivalence evidence".to_owned())?
        .value;
    let served = &state
        .cached
        .served
        .as_ref()
        .ok_or_else(|| "missing served equivalence evidence".to_owned())?
        .value;

    let report = judge(
        profile,
        &state.contract,
        &Proof {
            discovery: state.discovery.as_ref(),
            lint,
            dataflow,
            reproducible,
            refit,
            served,
        },
    );
    state.stage = if report.passed {
        GateStage::Passed
    } else {
        GateStage::Failed
    };
    state.history.push(RoundRecord {
        round: state.round,
        passed: report.passed,
        failed: report.failed.clone(),
        executed: state.executed.clone(),
        reused: state.reused.clone(),
        error: None,
    });
    state.latest = Some(report);
    Ok(())
}

fn record_error(state: &mut GateRunState, error: String) {
    state.stage = GateStage::Error;
    state.error = Some(error.clone());
    state.history.push(RoundRecord {
        round: state.round,
        passed: false,
        failed: Vec::new(),
        executed: state.executed.clone(),
        reused: state.reused.clone(),
        error: Some(error),
    });
}

async fn orchestrate(ctx: OrchestrationContext, start: StartInput) -> Result<GateRunState, String> {
    let mut state = GateRunState {
        run: ctx.instance_id(),
        scripts: start.scripts,
        round: 1,
        stage: GateStage::Running,
        contract: start.contract,
        hashes: start.hashes,
        discovery: start.discovery,
        cached: CachedEvidence::default(),
        executed: Vec::new(),
        reused: Vec::new(),
        latest: None,
        history: Vec::new(),
        error: None,
    };
    run_round(&ctx, &mut state, CachePolicy::Reuse).await?;
    let mut receipt = start.receipt;
    loop {
        ctx.set_custom_status(serde_json::to_string(&state).map_err(|e| e.to_string())?);
        ctx.set_kv_value_typed(&receipt, &None::<String>);
        if state.stage == GateStage::Passed {
            return Ok(state);
        }
        let raw = ctx.dequeue_event(DECISIONS).await;
        let request: Request = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        ctx.clear_kv_value(receipt);
        receipt = request.receipt;
        match request.decision {
            Decision::Retry(retry) => {
                state.round += 1;
                state.contract = retry.contract;
                state.hashes = retry.hashes;
                state.discovery = retry.discovery;
                run_round(&ctx, &mut state, retry.cache).await?;
            }
            Decision::Resume => {
                if state.stage == GateStage::Error || state.stage == GateStage::Running {
                    run_round(&ctx, &mut state, CachePolicy::Reuse).await?;
                }
            }
        }
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("cannot read {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8_192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn combine_hashes(parts: &[(&str, &str)]) -> String {
    let mut hasher = Sha256::new();
    for (label, digest) in parts {
        hasher.update(label.as_bytes());
        hasher.update(b"=");
        hasher.update(digest.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn compute_hashes(root: &Path, scripts: &GateScripts, contract: &Contract) -> Result<InputHashes> {
    let contract_h = hash_file(&root.join(&scripts.contract))?;
    let data_h = hash_file(&root.join(&contract.data.file))?;
    let model_h = hash_file(&root.join(&contract.model.file))?;
    let train_h = hash_file(&root.join(&scripts.train))?;
    let predict_h = hash_file(&root.join(&scripts.predict))?;
    let features_h = hash_file(&root.join(&scripts.features))?;

    let mut code_parts = Vec::with_capacity(contract.model.code.len());
    for code in &contract.model.code {
        code_parts.push((code.as_str(), hash_file(&root.join(code))?));
    }
    let code_refs: Vec<(&str, &str)> = code_parts
        .iter()
        .map(|(path, digest)| (*path, digest.as_str()))
        .collect();
    let code_h = combine_hashes(&code_refs);

    Ok(InputHashes {
        profile: combine_hashes(&[("data", &data_h)]),
        check: combine_hashes(&[
            ("train", &train_h),
            ("predict", &predict_h),
            ("features", &features_h),
        ]),
        graph: combine_hashes(&[("features", &features_h)]),
        refit: combine_hashes(&[
            ("contract", &contract_h),
            ("data", &data_h),
            ("model", &model_h),
            ("code", &code_h),
            ("features", &features_h),
        ]),
        reproducible: combine_hashes(&[
            ("train", &train_h),
            ("features", &features_h),
            ("data", &data_h),
            ("code", &code_h),
        ]),
        served: combine_hashes(&[
            ("train", &train_h),
            ("predict", &predict_h),
            ("features", &features_h),
            ("data", &data_h),
            ("model", &model_h),
            ("code", &code_h),
        ]),
    })
}

async fn run_python_json(
    root: &Path,
    script: &str,
    args: &[&str],
    allowed_codes: &[i32],
) -> Result<String, String> {
    let python = venv::project_python();
    let output = tokio::process::Command::new(&python)
        .arg("-c")
        .arg(script)
        .args(args)
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|error| format!("cannot start Python ({}): {error}", python.display()))?;
    let code = output.status.code().unwrap_or(-1);
    if allowed_codes.contains(&code) {
        return String::from_utf8(output.stdout)
            .map_err(|error| format!("command output was not UTF-8: {error}"));
    }
    let from_json = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    let reason = from_json.unwrap_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if stderr.is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        } else {
            stderr
        }
    });
    Err(format!("exited with {code}: {reason}"))
}

fn run_check_in_process(root: &Path, paths: &[String]) -> Result<CheckReport, String> {
    let workspace = Workspace::open(root.to_path_buf(), None, &Filters::default())
        .map_err(|error| format!("{error:#}"))?;
    validate_seed_rules(&workspace.catalog).map_err(|error| format!("{error:#}"))?;
    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let report = workspace
        .check(&path_bufs, FixMode::Apply(None))
        .map_err(|error| format!("{error:#}"))?;
    if report.skipped > 0 {
        return Err(format!("could not read {} file(s)", report.skipped));
    }
    let value = report.to_json(Applicability::Safe);
    let issues = value["issues"]
        .as_array()
        .ok_or_else(|| "check produced no issues array".to_owned())?
        .iter()
        .map(|item| {
            Ok(CheckIssue {
                package: item["package"].as_str().unwrap_or_default().to_owned(),
                rule: item["rule"].as_str().unwrap_or_default().to_owned(),
                file: item["file"].as_str().unwrap_or_default().to_owned(),
                line: item["line"].as_u64().and_then(|n| usize::try_from(n).ok()),
                severity: item["severity"].as_str().unwrap_or_default().to_owned(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CheckReport { issues })
}

fn run_graph_in_process(root: &Path, file: &str) -> Result<GraphReport, String> {
    let graph = Graph::read(root, &[PathBuf::from(file)]);
    if graph.skipped() > 0 {
        return Err(format!("could not read {file}"));
    }
    serde_json::from_value(graph.to_json()).map_err(|error| error.to_string())
}

async fn start_worker(provider: Arc<SqliteProvider>, root: Arc<Path>) -> Arc<Runtime> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .try_init();

    let r_profile = Arc::clone(&root);
    let r_check = Arc::clone(&root);
    let r_graph = Arc::clone(&root);
    let r_refit = Arc::clone(&root);
    let r_equiv = Arc::clone(&root);

    let activities = ActivityRegistry::builder()
        .register(ACT_PROFILE, move |_, raw: String| {
            let root = Arc::clone(&r_profile);
            async move {
                let input: ProfileInput = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                run_python_json(&root, PROFILE_PY, &[&input.file], &[0]).await
            }
        })
        .register(ACT_CHECK, move |_, raw: String| {
            let root = Arc::clone(&r_check);
            async move {
                let input: CheckInput = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                let report =
                    tokio::task::spawn_blocking(move || run_check_in_process(&root, &input.paths))
                        .await
                        .map_err(|e| e.to_string())??;
                serde_json::to_string(&report).map_err(|e| e.to_string())
            }
        })
        .register(ACT_GRAPH, move |_, raw: String| {
            let root = Arc::clone(&r_graph);
            async move {
                let input: GraphInput = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                let report =
                    tokio::task::spawn_blocking(move || run_graph_in_process(&root, &input.file))
                        .await
                        .map_err(|e| e.to_string())??;
                serde_json::to_string(&report).map_err(|e| e.to_string())
            }
        })
        .register(ACT_REFIT, move |_, raw: String| {
            let root = Arc::clone(&r_refit);
            async move {
                let input: RefitInput = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                run_python_json(&root, REFIT_PY, &[&input.contract], &[0, 1]).await
            }
        })
        .register(ACT_EQUIV, move |_, raw: String| {
            let root = Arc::clone(&r_equiv);
            async move {
                let input: EquivInput = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                run_python_json(&root, EQUIV_PY, &[&input.old, &input.new], &[0, 1]).await
            }
        })
        .build();

    let orchestrations = OrchestrationRegistry::builder()
        .register_typed(ORCHESTRATION, orchestrate)
        .build();

    let options = RuntimeOptions {
        worker_concurrency: 4,
        ..RuntimeOptions::default()
    };
    Runtime::start_with_options(provider, activities, orchestrations, options).await
}

fn read_state(status: OrchestrationStatus) -> Result<Option<GateRunState>> {
    match status {
        OrchestrationStatus::Failed { details, .. } => {
            bail!("gates workflow failed: {details:?}")
        }
        OrchestrationStatus::Completed { output, .. }
        | OrchestrationStatus::Running {
            custom_status: Some(output),
            ..
        } => Ok(Some(serde_json::from_str(&output)?)),
        _ => Ok(None),
    }
}

fn store_path(root: &Path, run: &str, mode: StoreMode) -> Result<PathBuf> {
    let create = mode == StoreMode::Create;
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::home_dir().map(|path| path.join(".local/state")))
        .context("cannot find the user state directory for gates runs")?;
    let namespace = Sha256::digest(root.as_os_str().as_encoded_bytes());
    let runs = state.join("xpd/gates").join(format!("{namespace:x}"));
    if create {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.recursive(true).create(&runs)?;
    }
    let path = runs.join(format!("{run}.sqlite"));
    ensure!(
        !path.is_symlink(),
        "invalid gates database {}",
        path.display()
    );
    ensure!(create || path.is_file(), "unknown gates run {run}");
    Ok(path)
}

async fn wait_for_receipt(client: &Client, run: &str, receipt: &str) -> Result<GateRunState> {
    let timeout = Duration::from_secs(3600);
    tokio::select! {
        biased;
        reply = client.wait_for_kv_value_typed::<Option<String>>(run, receipt, timeout) => {
            if let Some(rejected) = reply? {
                bail!("{rejected}");
            }
            read_state(client.get_orchestration_status(run).await?)?
                .context("missing gates state after receipt")
        }
        done = client.wait_for_orchestration(run, timeout) => {
            read_state(done?)?.context("missing completed gates state")
        }
    }
}

pub(crate) fn run(command: GatesCommand, default_format: Format) -> Result<ExitCode> {
    let root = env::current_dir()?.canonicalize()?;
    let format = match &command {
        GatesCommand::Start { format, .. }
        | GatesCommand::Retry { format, .. }
        | GatesCommand::Status { format, .. }
        | GatesCommand::Resume { format, .. } => format.unwrap_or(default_format),
    };
    let observing = matches!(command, GatesCommand::Status { .. });
    let run_id = match &command {
        GatesCommand::Start { .. } => Uuid::new_v4(),
        GatesCommand::Retry { run, .. }
        | GatesCommand::Status { run, .. }
        | GatesCommand::Resume { run, .. } => *run,
    }
    .simple()
    .to_string();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("cannot build the Tokio runtime for gates")?;
    let state = runtime.block_on(execute(command, root, &run_id))?;
    state.print(format)?;
    if observing {
        return Ok(ExitCode::SUCCESS);
    }
    Ok(match state.stage {
        GateStage::Passed => ExitCode::SUCCESS,
        GateStage::Failed => ExitCode::FAILURE,
        GateStage::Error | GateStage::Running => ExitCode::from(EXIT_ERROR),
    })
}

async fn submit_command(
    command: GatesCommand,
    client: &Client,
    root: &Path,
    run_id: &str,
    receipt: &str,
    existing: Option<&GateRunState>,
) -> Result<()> {
    match command {
        GatesCommand::Start {
            contract,
            train,
            predict,
            features,
            discovery,
            ..
        } => {
            let scripts = GateScripts {
                contract: inside_file(root, &contract)?,
                train: inside_file(root, &train)?,
                predict: inside_file(root, &predict)?,
                features: inside_file(root, &features)?,
            };
            let parsed = read_contract(root, &scripts.contract)?;
            let hashes = compute_hashes(root, &scripts, &parsed)?;
            let discovery = read_discovery(discovery.as_deref())?;
            client
                .start_orchestration_typed(
                    run_id,
                    ORCHESTRATION,
                    &StartInput {
                        scripts,
                        contract: parsed,
                        hashes,
                        discovery,
                        receipt: receipt.to_owned(),
                    },
                )
                .await?;
        }
        GatesCommand::Retry {
            discovery,
            no_cache,
            ..
        } => {
            let state = existing
                .context("gates start has not completed yet; run `xpd gates resume` first")?;
            let parsed = read_contract(root, &state.scripts.contract)?;
            let hashes = compute_hashes(root, &state.scripts, &parsed)?;
            let discovery = match discovery {
                Some(path) => read_discovery(Some(&path))?,
                None => state.discovery.clone(),
            };
            let cache = if no_cache {
                CachePolicy::Refresh
            } else {
                CachePolicy::Reuse
            };
            client
                .enqueue_event_typed(
                    run_id,
                    DECISIONS,
                    &Request {
                        receipt: receipt.to_owned(),
                        decision: Decision::Retry(Box::new(RetryPayload {
                            contract: parsed,
                            hashes,
                            discovery,
                            cache,
                        })),
                    },
                )
                .await?;
        }
        GatesCommand::Resume { .. } => {
            client
                .enqueue_event_typed(
                    run_id,
                    DECISIONS,
                    &Request {
                        receipt: receipt.to_owned(),
                        decision: Decision::Resume,
                    },
                )
                .await?;
        }
        GatesCommand::Status { .. } => unreachable!("status returned before submit"),
    }
    Ok(())
}

async fn execute(command: GatesCommand, root: PathBuf, run_id: &str) -> Result<GateRunState> {
    let mode = if matches!(command, GatesCommand::Start { .. }) {
        StoreMode::Create
    } else {
        StoreMode::Existing
    };
    let path = store_path(&root, run_id, mode)?;
    let _writer = if matches!(command, GatesCommand::Status { .. }) {
        None
    } else {
        let file = fs::File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path.with_extension("lock"))?;
        file.try_lock()
            .with_context(|| format!("another command is changing gates run {run_id}"))?;
        Some(file)
    };
    let url = format!(
        "sqlite:{}?mode={}",
        path.to_str()
            .context("the gates state path must be UTF-8")?
            .replace('%', "%25")
            .replace('?', "%3F"),
        if mode == StoreMode::Create {
            "rwc"
        } else {
            "rw"
        }
    );
    let provider = Arc::new(SqliteProvider::new(&url, None).await?);
    let client = Client::new(Arc::<SqliteProvider>::clone(&provider));
    let existing = read_state(client.get_orchestration_status(run_id).await?)?;

    if matches!(command, GatesCommand::Status { .. }) {
        return existing.context("gates start is queued; run `xpd gates resume` to advance it");
    }

    if let Some(state) = existing.as_ref()
        && state.stage == GateStage::Passed
    {
        return Ok(state.clone());
    }

    let receipt = Uuid::new_v4().simple().to_string();
    submit_command(command, &client, &root, run_id, &receipt, existing.as_ref()).await?;

    let worker = start_worker(Arc::clone(&provider), Arc::from(root.as_path())).await;
    let result = wait_for_receipt(&client, run_id, &receipt).await;
    worker.shutdown(Some(1_000)).await;
    result.with_context(|| format!("gates run {run_id}"))
}
