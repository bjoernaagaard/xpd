//! `xpd doctor` holds a project to the profile, the target setup that ships with xpd in
//! `doctor/profile.toml`, and reports the other settings worth changing in Ruff, ty and pytest.
//!
//! The plan in `doctor/plan.rs` is the difference between the project and the profile. Plain
//! doctor prints it with the conditional advice of `doctor/decisions.jsonl`, `--check` exits 1
//! when the project drifts, and `--apply` writes the plan. Missing tools leave the conditional
//! advice unverified locally. Computing the plan runs no tool. `--check` then runs uv on the
//! lock files and the locked pytest with `--markers`, and every mode runs the pinned Ruff on the
//! settings when it is installed. Every line doctor prints has the credentials of each URL
//! redacted.

mod advice;
mod apply;
pub mod catalog;
mod edit;
mod evidence;
mod placement;
mod plan;
mod probe;
mod profile;
mod pytest;
mod ruff;
mod ty;
mod validate;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::output::{Format, redact};

pub use evidence::evidence_rule_ids;

/// What `xpd doctor` does with the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Print the plan and the conditional advice, and exit 0.
    Report,
    /// Print the plan, and exit 1 when the project drifts from the profile.
    Check,
    /// Write the plan.
    Apply,
}

/// The exit code of `--check` when the project drifts from the profile.
const DRIFT: u8 = 1;
/// The exit code of `--check` when doctor cannot tell which setup applies, or cannot hold the
/// layout to the profile.
const AMBIGUOUS: u8 = 2;

/// Runs doctor on the project in the working directory. Every line it prints, and the error it
/// returns, has the credentials of each URL redacted.
///
/// The URLs of the packaged catalog, such as the source link of each record with its anchor,
/// print whole, because they are xpd's own text. Every other URL, from the project or from a
/// tool's message, is redacted, even on the same host.
pub(crate) fn run(format: Format, mode: Mode, kind: Option<&str>) -> Result<ExitCode> {
    let mut out = String::new();
    let code = report(format, mode, kind, &mut out)
        .map_err(|error| anyhow!(redact(&format!("{error:#}"))));
    let own = crate::output::urls(catalog::DECISIONS);
    print!("{}", crate::output::redact_except(&out, &own));
    code
}

/// Runs doctor, and writes what it prints to `out`.
fn report(format: Format, mode: Mode, kind: Option<&str>, out: &mut String) -> Result<ExitCode> {
    let root = std::env::current_dir()?;
    let catalog = catalog::Catalog::packaged()?;
    let profile = profile::Profile::packaged(&catalog)?;
    let mut files: Vec<PathBuf> = crate::source::python_files(std::slice::from_ref(&root))?;
    files.sort();
    let request = plan::Request { kind, stamp: true };
    match mode {
        Mode::Apply => {
            let applied = apply::run(&root, &catalog, &profile, &files, kind)?;
            match format {
                Format::Json => writeln!(out, "{}", serde_json::to_string_pretty(&applied)?)?,
                Format::Text => out.push_str(&applied.to_text()),
            }
            Ok(ExitCode::SUCCESS)
        }
        Mode::Check => {
            let mut plan = plan::compute(&root, &catalog, &profile, &files, request)?;
            plan.check_tools(&root, validate::Fetch::Online);
            plan.check_locks(&root)?;
            plan.check_pytest(&root);
            match format {
                Format::Json => writeln!(out, "{}", serde_json::to_string_pretty(&plan)?)?,
                Format::Text => out.push_str(&plan.to_text()),
            }
            Ok(if plan.ambiguous() {
                ExitCode::from(AMBIGUOUS)
            } else if plan.drifts() {
                ExitCode::from(DRIFT)
            } else if !plan.unverified.is_empty() {
                ExitCode::from(AMBIGUOUS)
            } else {
                ExitCode::SUCCESS
            })
        }
        Mode::Report => {
            let mut plan = plan::compute(&root, &catalog, &profile, &files, request)?;
            plan.check_tools(&root, validate::Fetch::Offline);
            let report = advise(&root, &files, &catalog)?;
            match format {
                Format::Json => {
                    let output = Output {
                        profile: &plan,
                        report: &report,
                    };
                    writeln!(out, "{}", serde_json::to_string_pretty(&output)?)?;
                }
                Format::Text => write!(out, "{}\n{}", plan.to_text(), report.to_text())?,
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// The JSON of plain doctor: the plan beside the conditional advice.
#[derive(Serialize)]
struct Output<'a> {
    profile: &'a plan::Plan,
    #[serde(flatten)]
    report: &'a advice::Report,
}

/// The conditional advice of the catalog for the project at `root`.
fn advise(root: &Path, files: &[PathBuf], catalog: &catalog::Catalog) -> Result<advice::Report> {
    let evidence = evidence::find(files)?;
    let ruff = ruff::analyze(
        root,
        files,
        catalog,
        probe::Checker::find(catalog::Tool::Ruff, root),
    );
    let ty = ty::analyze(root, catalog, probe::Checker::find(catalog::Tool::Ty, root));
    Ok(advice::evaluate(
        root, files, catalog, &evidence, &ruff, &ty,
    ))
}

/// The differences between a project and the profile of one revision.
#[derive(Debug)]
pub(crate) struct Drift {
    /// The revision of the profile that this xpd holds.
    pub(crate) revision: i64,
    /// One line for each difference, without a line number, so that the same difference gives
    /// the same line after an unrelated edit.
    pub(crate) lines: Vec<String>,
    /// One line for each lock that uv could not check offline, which starts with the lock file.
    pub(crate) unverified: Vec<String>,
}

/// The differences between the project at `root`, whose Python files are `files`, and the
/// profile. `None` when the project records no profile. It reads files and runs only
/// `uv lock --check --offline`, which reads uv's cache and never the network, so the Stop hook
/// can call it on every turn. Every line has the credentials of each URL redacted.
pub(crate) fn drift(root: &Path, files: &[PathBuf]) -> Option<Result<Drift>> {
    let doc = match crate::toml::read(&root.join(plan::PYPROJECT)) {
        Ok(Some(doc)) => doc,
        Ok(None) => return None,
        Err(error) => return Some(Err(anyhow!(redact(&error)))),
    };
    crate::toml::get(&doc, &profile::TABLE)?;
    let drift = (|| {
        let catalog = catalog::Catalog::packaged()?;
        let profile = profile::Profile::packaged(&catalog)?;
        let request = plan::Request {
            kind: None,
            stamp: true,
        };
        let mut plan = plan::compute(root, &catalog, &profile, files, request)?;
        let unverified = plan.check_locks_offline(root);
        Ok(Drift {
            revision: plan.revision,
            lines: plan.drift().iter().map(|line| redact(line)).collect(),
            unverified: unverified.iter().map(|line| redact(line)).collect(),
        })
    })();
    Some(drift.map_err(|error: anyhow::Error| anyhow!(redact(&format!("{error:#}")))))
}
