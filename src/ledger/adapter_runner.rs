//! Runs external Python or shell runtime adapters and parses their JSON output.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use serde_json::Value;

use crate::venv;

/// Executes `adapter` in `work_dir`, wrapping `.py` files with `runner`, and returns its JSON.
pub(crate) fn invoke_adapter(adapter: &str, work_dir: &Path, runner: &str) -> Result<Value> {
    let candidate = work_dir.join(adapter);
    let is_python_file = candidate.is_file()
        && candidate
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("py"));

    let output = if is_python_file {
        Command::new(venv::project_python_at(work_dir))
            .arg("-c")
            .arg(runner)
            .arg(&candidate)
            .current_dir(work_dir)
            .stdin(Stdio::null())
            .output()
    } else if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", adapter])
            .current_dir(work_dir)
            .stdin(Stdio::null())
            .output()
    } else {
        Command::new("sh")
            .args(["-c", adapter])
            .current_dir(work_dir)
            .stdin(Stdio::null())
            .output()
    };

    let output = match output {
        Ok(output) => output,
        Err(error) => bail!("could not start adapter `{adapter}`: {error}"),
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    let last_stderr = stderr.lines().rev().find(|line| !line.trim().is_empty());

    let Ok(parsed) = serde_json::from_slice::<Value>(&output.stdout) else {
        let code = output
            .status
            .code()
            .map_or_else(|| "a signal".to_owned(), |c| c.to_string());
        if let Some(last) = last_stderr {
            bail!(
                "adapter `{adapter}` exited with {code} and printed no JSON. Last line on stderr: {}",
                last.trim()
            );
        }
        bail!("adapter `{adapter}` exited with {code} and printed no JSON");
    };

    if let Some(error) = parsed.get("error").and_then(Value::as_str) {
        if let Some(last) = last_stderr {
            bail!("{error}. Last line on stderr: {}", last.trim());
        }
        bail!("{error}");
    }
    if !output.status.success() {
        bail!(
            "adapter `{adapter}` exited with {}",
            output.status.code().unwrap_or(1)
        );
    }
    Ok(parsed)
}
