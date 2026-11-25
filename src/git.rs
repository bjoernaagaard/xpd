//! Runs `git` and reads what it prints.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

/// What `git` prints for `args` in `folder`, preserving spawn, status, and stderr details.
pub(crate) fn output(folder: &Path, args: &[&str]) -> Result<String> {
    output_with(folder, args, &[])
}

/// What `git` prints for `args` in `folder`, with the variables of `env` set for the command.
pub(crate) fn output_with(folder: &Path, args: &[&str], env: &[(&str, &OsStr)]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(folder)
        .args(args)
        .envs(env.iter().copied())
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to run git {}", args.first().copied().unwrap_or("")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let subcommand = args.first().copied().unwrap_or("");
    if stderr.is_empty() {
        anyhow::bail!("git {subcommand} failed ({})", output.status);
    }
    anyhow::bail!("git {subcommand} failed ({}): {stderr}", output.status)
}
