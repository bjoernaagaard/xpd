//! Workspace probes, shell checks, benchmarks, and git queries for recorded ledger runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{Context, Result, ensure};

use crate::git;

use super::ChecklistOwners;
use super::feature_evidence::FeatureEvidence;
use super::run_journal::{JournalBench, JournalCheck, RunJournal, Timing};
use super::workflow_registry::{CheckDeterminismPolicy, WorkflowId};
use super::{git_repository, walk_code_files};

/// How much of the output of a failed check the journal keeps.
const OUTPUT_KEPT: usize = 1500;

/// Surveyed finding IDs, checklist item owners, and optional fold-level feature evidence.
pub(super) type SurveyedFindings = (BTreeSet<String>, ChecklistOwners, Option<FeatureEvidence>);

/// The findings and checklist owners of a run at a commit, or of the working tree for `None`.
pub(super) type SurveyProbe<'f> = &'f dyn Fn(&RunJournal, Option<&str>) -> Result<SurveyedFindings>;

/// Maps legacy finding IDs from xpd 0.55.0 to current `SipHash` finding IDs at a commit or tree.
pub(super) type Renamed<'r> =
    &'r dyn Fn(&RunJournal, Option<&str>) -> Result<BTreeMap<String, String>>;

/// Total lines across all code files under `paths`.
pub(super) fn count_lines(paths: &[PathBuf]) -> usize {
    walk_code_files(paths)
        .into_iter()
        .map(|(path, _)| fs::read_to_string(&path).map_or(0, |text| text.lines().count()))
        .sum()
}

/// The class of machine this process runs on: its CPU model, OS and architecture. Two
/// timings of a benchmark compare only when they come from the same class.
pub(super) fn host_fingerprint() -> String {
    format!(
        "{} / {} / {}",
        cpu_model().unwrap_or_else(|| "unknown CPU".to_owned()),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// The CPU model, from `/proc/cpuinfo` on Linux and `sysctl` on macOS.
fn cpu_model() -> Option<String> {
    if cfg!(target_os = "macos") {
        let output = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .stdin(Stdio::null())
            .output()
            .ok()?;
        let model = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (output.status.success() && !model.is_empty()).then_some(model)
    } else {
        model_name(&fs::read_to_string("/proc/cpuinfo").ok()?).map(str::to_owned)
    }
}

/// The value of the first `model name` line of `/proc/cpuinfo`.
fn model_name(cpuinfo: &str) -> Option<&str> {
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim())
    })
}

/// Rounds a millisecond measurement to two decimal places.
fn round_ms(ms: f64) -> f64 {
    (ms * 100.0).round() / 100.0
}

/// Times `command` with one warmup run followed by `runs` measured runs.
pub(super) fn measure_bench(command: &str, runs: usize) -> Result<Timing> {
    let runs = runs.max(1);
    let (passed, printed) = shell_check_passes(command);
    ensure!(
        passed,
        "benchmark `{command}` failed during warmup:\n{printed}"
    );
    let mut samples = Vec::with_capacity(runs);
    for run in 1..=runs {
        let started = Instant::now();
        let (passed, printed) = shell_check_passes(command);
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        ensure!(
            passed,
            "benchmark `{command}` failed on run {run}/{runs}:\n{printed}"
        );
        samples.push(elapsed_ms);
    }
    samples.sort_by(f64::total_cmp);
    let mid = samples.len() / 2;
    let median = if samples.len() % 2 == 1 {
        samples[mid]
    } else {
        f64::midpoint(samples[mid - 1], samples[mid])
    };
    Ok(Timing {
        median_ms: round_ms(median),
        min_ms: round_ms(samples[0]),
        max_ms: round_ms(*samples.last().expect("runs is at least 1")),
        runs,
    })
}

/// Tracked files changed between `current` and `head` in `top`.
pub(super) fn diff_changed_files(
    top: &Path,
    current: &str,
    head: &str,
) -> Result<BTreeSet<String>> {
    let diff = git::output(top, &["diff", "--name-only", current, head])?;
    Ok(diff
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect())
}

/// Runs each check that passed at the start and returns the ones that fail now.
pub(super) fn broken_checks<'a>(
    workflow: WorkflowId,
    checks: impl IntoIterator<Item = (&'a str, bool)>,
) -> Vec<String> {
    let repeat_first =
        workflow.spec().check_determinism == CheckDeterminismPolicy::RepeatFirstStdoutHash;
    let mut broken = Vec::new();
    let mut first = None;
    for (index, (command, passed)) in checks.into_iter().enumerate() {
        if passed || (repeat_first && index == 0) {
            match check_output(command) {
                Ok(output) => {
                    if passed && !output.status.success() {
                        broken.push(format!("{command}\n{}", check_text(&output)));
                    }
                    if repeat_first && index == 0 {
                        first = Some((command, output));
                    }
                }
                Err(error) => broken.push(format!("{command}\ncannot start: {error}")),
            }
        }
    }
    if broken.is_empty()
        && let Some((command, output)) = first
    {
        use sha2::{Digest as _, Sha256};

        match check_output(command) {
            Ok(repeat)
                if repeat.status == output.status
                    && Sha256::digest(&repeat.stdout) == Sha256::digest(&output.stdout) => {}
            Ok(_) => broken.push(format!(
                "determinism repeat disagreed for {command}: exit status or stdout changed"
            )),
            Err(error) => broken.push(format!(
                "determinism repeat could not run {command}: {error}"
            )),
        }
    }
    broken
}

/// The git repository top folder and legacy journal path of a recorded ledger run.
pub(super) struct RunWorkspace {
    pub(super) top: PathBuf,
    pub(super) journal: PathBuf,
}

impl RunWorkspace {
    /// The repository that holds the working folder. The process moves to its top, so that
    /// the paths of the journal and the checks start there.
    pub(super) fn find(workflow: WorkflowId) -> Result<Self> {
        let top = git_repository(Path::new("."))
            .context("this folder is in no git repository, which a run needs")?;
        std::env::set_current_dir(&top)
            .with_context(|| format!("cannot enter {}", top.display()))?;
        let journal = resolve_git_path(&top, &format!("xpd-{}.json", workflow.as_str()))?;
        Ok(Self { top, journal })
    }

    /// The path of `name` in the git folder of the worktree.
    pub(super) fn git_path(&self, name: &str) -> Result<PathBuf> {
        resolve_git_path(&self.top, name)
    }

    /// Full commit hash at `HEAD`.
    pub(super) fn head(&self) -> Result<String> {
        Ok(git::output(&self.top, &["rev-parse", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// Fails when a tracked file differs from the commit, since a gate measures the commit.
    pub(super) fn clean(&self) -> Result<()> {
        let changed = git::output(
            &self.top,
            &["status", "--porcelain", "--untracked-files=no"],
        )?;
        ensure!(
            changed.trim().is_empty(),
            "tracked files differ from the commit. Commit the fix first:\n{changed}"
        );
        Ok(())
    }
}

/// The path of `name` in the git folder of the worktree at `top`, which differs between linked
/// worktrees of one repository.
pub(super) fn resolve_git_path(top: &Path, name: &str) -> Result<PathBuf> {
    let path = git::output(top, &["rev-parse", "--git-path", name])?;
    Ok(top.join(path.trim()))
}

/// Whether `command` exits 0 in the working folder, with the end of what it printed.
pub(super) fn shell_check_passes(command: &str) -> (bool, String) {
    match check_output(command) {
        Ok(output) => (output.status.success(), check_text(&output)),
        Err(error) => (false, format!("cannot start: {error}")),
    }
}

fn check_output(command: &str) -> std::io::Result<std::process::Output> {
    let mut shell = if cfg!(windows) {
        let mut shell = Command::new("cmd");
        shell.arg("/C");
        shell
    } else {
        let mut shell = Command::new("sh");
        shell.arg("-c");
        shell
    };
    shell.arg(command).stdin(Stdio::null()).output()
}

fn check_text(output: &std::process::Output) -> String {
    let mut printed = String::from_utf8_lossy(&output.stdout).into_owned();
    printed.push_str(&String::from_utf8_lossy(&output.stderr));
    let from = printed.len().saturating_sub(OUTPUT_KEPT);
    let from = (from..printed.len())
        .find(|at| printed.is_char_boundary(*at))
        .unwrap_or(printed.len());
    printed[from..].trim_end().to_owned()
}

/// Normalizes an adapter argument: when it names an existing file under `here`, stores its
/// repository-relative path so checkouts and worktrees find the same file.
fn normalize_adapter(
    workspace: &RunWorkspace,
    here: &Path,
    adapter: Option<String>,
) -> Result<Option<String>> {
    let Some(adapter) = adapter else {
        return Ok(None);
    };
    let candidate = here.join(&adapter);
    if candidate.is_file() {
        let absolute = fs::canonicalize(&candidate)
            .with_context(|| format!("cannot read adapter {adapter}"))?;
        if let Ok(below) = absolute.strip_prefix(&workspace.top) {
            return Ok(Some(below.to_string_lossy().replace('\\', "/")));
        }
    }
    Ok(Some(adapter))
}

/// Validates a `start --adapter` argument against the workflow's adapter policy.
pub(super) fn validate_start_adapter(
    workflow: WorkflowId,
    workspace: &RunWorkspace,
    here: &Path,
    adapter: Option<&str>,
) -> Result<()> {
    if workflow.spec().adapter.requires_committed_python()
        && let Some(spec) = adapter
    {
        let candidate = here.join(spec);
        let is_python = candidate
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("py"));
        let single_word = shlex::split(spec).is_some_and(|words| words.len() == 1);
        if is_python && (candidate.is_file() || single_word) {
            let file = candidate
                .canonicalize()
                .with_context(|| format!("cannot read Python adapter {spec}"))?;
            let relative = file
                .strip_prefix(&workspace.top)
                .context("recorded Python adapter must be in the repository")?;
            let relative = relative
                .to_str()
                .context("Python adapter path must be UTF-8")?;
            git::output(
                &workspace.top,
                &["cat-file", "-e", &format!("HEAD:{relative}")],
            )
            .with_context(|| format!("commit the Python adapter {relative} before starting"))?;
        }
    }
    Ok(())
}

/// Measures the starting commit and constructs the initial journal for a new run.
pub(super) fn build_initial_journal(
    workspace: &RunWorkspace,
    here: &Path,
    paths: &[PathBuf],
    checks: Vec<String>,
    bench: Option<(String, usize)>,
    adapter: (Option<String>, Option<String>),
    found: SurveyProbe<'_>,
) -> Result<RunJournal> {
    workspace.clean()?;
    let paths = (paths.iter())
        .map(|path| {
            let absolute = fs::canonicalize(here.join(path))
                .with_context(|| format!("cannot read {}", path.display()))?;
            let below = absolute
                .strip_prefix(&workspace.top)
                .with_context(|| format!("{} is outside the repository", path.display()))?;
            Ok(if below.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                below.to_path_buf()
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let (adapter, target) = adapter;
    let adapter = normalize_adapter(workspace, here, adapter)?;
    let adapter = crate::data_prep::record_target(adapter, target)?;
    let base = workspace.head()?;
    let lines = count_lines(&paths);
    let bench = bench
        .map(|(command, runs)| -> Result<JournalBench> {
            let timing = measure_bench(&command, runs)?;
            Ok(JournalBench {
                command,
                runs: timing.runs,
                baseline: timing,
                current: timing,
                host_fingerprint: Some(host_fingerprint()),
            })
        })
        .transpose()?;
    let mut journal = RunJournal {
        current: base.clone(),
        base,
        paths,
        checks: (checks.into_iter())
            .map(|command| JournalCheck {
                passed: shell_check_passes(&command).0,
                command,
            })
            .collect(),
        bench,
        adapter,
        feature_evidence: None,
        was: 0,
        was_lines: lines,
        current_lines: lines,
        rounds: Vec::new(),
        accepted: BTreeMap::new(),
        answered: BTreeMap::new(),
        finished: false,
    };
    let (findings, _, evidence) = found(&journal, None)?;
    journal.was = findings.len();
    journal.feature_evidence = evidence;
    Ok(journal)
}

/// Ensures feature-review runs remain on the last kept commit before planning or finishing.
pub(super) fn require_locked_feature_head(
    journal: &RunJournal,
    workspace: &RunWorkspace,
) -> Result<()> {
    if journal.feature_evidence.is_some() {
        workspace.clean()?;
        ensure!(
            workspace.head()? == journal.current,
            "feature review must measure the last kept commit {}",
            journal.current
        );
    }
    Ok(())
}
