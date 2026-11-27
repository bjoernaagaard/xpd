use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::venv;

const PERF_FACTS_PY: &str = include_str!("../../py/perf_facts.py");

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Workload {
    name: String,
    median_ms: f64,
    peak_bytes: u64,
    pub(super) functions: Vec<FunctionCost>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct FunctionCost {
    pub(super) file: String,
    pub(super) name: String,
    line: usize,
    cpu_ms: f64,
    cumulative_cpu_ms: f64,
    pub(super) cpu_share: f64,
    calls: u64,
}

#[derive(Deserialize)]
struct Measurements {
    #[serde(default)]
    workloads: Vec<Workload>,
    error: Option<String>,
}

pub(super) fn measure(adapter: &str, work_dir: &Path) -> Result<Vec<Workload>> {
    let candidate = work_dir.join(adapter);
    let mut command = if candidate.is_file() && candidate.extension().is_some_and(|ext| ext == "py")
    {
        let mut command = Command::new(venv::project_python_at(work_dir));
        command
            .arg("-c")
            .arg(PERF_FACTS_PY)
            .arg(candidate.canonicalize()?);
        command
    } else {
        let (shell, option) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let mut command = Command::new(shell);
        command.args([option, adapter]);
        command
    };
    let output = command
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("could not start performance adapter `{adapter}`"))?;
    let data: Measurements = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "performance adapter `{adapter}` did not return measurements ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })?;
    ensure!(
        data.error.is_none(),
        "performance adapter `{adapter}` failed: {}",
        data.error.unwrap_or_default()
    );
    ensure!(
        output.status.success(),
        "performance adapter `{adapter}` exited with {}",
        output.status
    );
    ensure!(
        !data.workloads.is_empty(),
        "performance adapter `{adapter}` returned no workloads"
    );
    Ok(data.workloads)
}
