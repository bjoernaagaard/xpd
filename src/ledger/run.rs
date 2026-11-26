//! A refactor that a command of the ledger measures, as a run with stages that xpd records.
//!
//! The run is a journal in the git folder of the repository, so it survives the end of a
//! session and belongs to no agent tool. The agent judges, designs and edits. xpd keeps what
//! was decided, says which stage comes next, and gates each fix itself: it runs the checks of
//! the project and compares the findings by their ids, so no stage grades its own work.
//!
//! Without the `ledger-durable` feature the journal is the JSON file `xpd-<command>.json` in the
//! git folder of the worktree. With it, the run is a `duroxide` orchestration in
//! `<git-common-dir>/xpd-ledger.sqlite`, and the CLI measures as before and hands each
//! measurement to the orchestration as an activity result. A JSON journal that an older xpd left
//! is imported once, by the worktree that owns it, and renamed with the `.migrated` suffix.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::git;
use crate::output::Format;

use super::{Entry, Kind, Tree, at_commit, files, repository};

/// How many times a fix may fail its gate before the next stage is to drop it.
const ATTEMPTS: usize = 2;
/// How much of the output of a failed check the journal keeps.
const OUTPUT_KEPT: usize = 1500;
/// Maximum allowed slowdown factor on `--bench` before a gate rejects a fix as a regression.
#[cfg(not(feature = "ledger-durable"))]
const MAX_SLOWDOWN: f64 = 1.15;
/// Minimum speedup factor required for a fix whose goal is `faster` or `both`.
#[cfg(not(feature = "ledger-durable"))]
const MIN_SPEEDUP: f64 = 0.98;

/// A stage of a run.
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Action {
    /// Start a run: record the commit it starts from, the paths, line count, checks and optional
    /// benchmark of the project.
    Start {
        /// Files or directories to measure, inside one git repository.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// A command that exits 0 when the project is sound, such as its tests. Repeat the flag
        /// for more than one. Each runs now, and again at every gate.
        #[arg(long = "check", value_name = "COMMAND")]
        checks: Vec<String>,
        /// A command that runs a representative workload to time before and after each fix.
        /// Measured with one warmup run and `--bench-runs` timed runs.
        #[arg(long, value_name = "COMMAND")]
        bench: Option<String>,
        /// How many timed runs each benchmark measurement takes.
        #[arg(long, default_value_t = 5)]
        bench_runs: usize,
        /// Replace a run that is not finished.
        #[arg(long)]
        restart: bool,
    },
    /// Record the plan of a round: the fixes in order, and the findings that are right as they
    /// are.
    ///
    /// The file holds JSON such as
    /// `{"fixes": [{"id": "one-price-rule", "title": "...", "findings": ["d81b64d1"],
    /// "changes_interface": false}], "accepted": [{"finding": "cc6bc6e7", "reason": "..."}]}`.
    /// A fix with no static finding id may set `"goal": "shorter"`, `"faster"` or `"both"` so
    /// the gate checks line count and `--bench` timing instead.
    Plan {
        /// The JSON file of the plan, or `-` for stdin.
        file: PathBuf,
    },
    /// Record that the user said yes to a fix that changes what callers or users see.
    Approve { fix: String },
    /// Gate a fix that is committed: run the checks, time `--bench` when set, count lines and
    /// compare the findings with the commit before it. Exit 1 when the fix is not kept.
    Gate { fix: String },
    /// Record that a fix was given up, after its files were restored.
    Drop {
        fix: String,
        /// Why the fix was given up.
        #[arg(long)]
        reason: String,
    },
    /// Print the run and the stage that comes next.
    Status,
    /// End the run: compare findings, line count and benchmark timing with the start commit.
    /// Exit 1 while a fix is open or a new finding has no reason.
    Finish,
}

/// One check of the project.
#[derive(Debug, Serialize, Deserialize)]
struct Check {
    command: String,
    /// Whether it passed when the run started. A check that failed then gates nothing.
    passed: bool,
}

/// Measured wall-clock duration of one benchmark point, in milliseconds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct Timing {
    pub(super) median_ms: f64,
    pub(super) min_ms: f64,
    pub(super) max_ms: f64,
    pub(super) runs: usize,
}

/// A benchmark command tracked across the run.
#[derive(Debug, Serialize, Deserialize)]
struct Bench {
    command: String,
    runs: usize,
    baseline: Timing,
    current: Timing,
    /// The class of machine that timed `baseline` and `current`, from [`host_fingerprint`].
    /// A gate timed on another class of machine times both commits there, back to back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host_fingerprint: Option<String>,
}

/// What proves a fix that names no static finding id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Goal {
    /// Must reduce total code lines in the measured paths.
    Shorter,
    /// Must reduce median `--bench` time by at least 2%.
    Faster,
    /// Must reduce both code lines and median `--bench` time.
    Both,
}

/// How a fix ended.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
enum Outcome {
    Kept {
        commit: String,
        /// The ids of the findings that the fix removed.
        fixed: Vec<String>,
        /// The ids of the findings that the fix added.
        added: Vec<String>,
        #[serde(default)]
        lines_before: usize,
        #[serde(default)]
        lines_after: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timing_before: Option<Timing>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timing_after: Option<Timing>,
    },
    Dropped {
        reason: String,
    },
}

/// One fix of a plan.
#[derive(Debug, Serialize, Deserialize)]
struct Fix {
    id: String,
    title: String,
    /// The ids of the findings that the fix is to remove. May be empty when `goal` is set.
    #[serde(default)]
    findings: Vec<String>,
    /// Measurable target for a judgment-based fix with no static finding id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    goal: Option<Goal>,
    /// The fix changes what code outside the paths can call, or what a user sees.
    #[serde(default)]
    changes_interface: bool,
    #[serde(default)]
    approved: bool,
    /// Why each failed gate did not keep the fix.
    #[serde(default)]
    attempts: Vec<String>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
}

/// A finding that is right as it is.
#[derive(Debug, Serialize, Deserialize)]
struct Accepted {
    finding: String,
    reason: String,
}

/// What `plan` reads.
#[derive(Debug, Deserialize)]
struct Plan {
    #[serde(default)]
    fixes: Vec<Fix>,
    #[serde(default)]
    accepted: Vec<Accepted>,
}

/// What xpd records of a run.
#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    /// The commit that the run started from.
    base: String,
    /// The commit that the next fix starts from: the base, or the last fix that was kept.
    current: String,
    /// The paths that are measured, from the top of the repository.
    paths: Vec<PathBuf>,
    checks: Vec<Check>,
    /// Optional representative workload timed at start and at every gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bench: Option<Bench>,
    /// How many findings the base had.
    was: usize,
    /// Total lines of code in `paths` at the start commit.
    #[serde(default)]
    was_lines: usize,
    /// Total lines of code in `paths` at `current`.
    #[serde(default)]
    current_lines: usize,
    /// The fixes of each round, in the order of the plan.
    rounds: Vec<Vec<Fix>>,
    /// The findings that are right as they are, each id with its reason.
    accepted: BTreeMap<String, String>,
    finished: bool,
}

/// The stage that comes next, with what to do in it.
#[derive(Debug, Serialize)]
struct Next {
    stage: &'static str,
    text: String,
}

impl Journal {
    fn fixes(&self) -> impl Iterator<Item = &Fix> {
        self.rounds.iter().flatten()
    }

    /// The first fix that is neither kept nor dropped.
    fn open(&self) -> Option<&Fix> {
        self.fixes().find(|fix| fix.outcome.is_none())
    }

    fn fix_mut(&mut self, id: &str) -> Result<&mut Fix> {
        let fix = (self.rounds.iter_mut().flatten())
            .find(|fix| fix.id == id)
            .with_context(|| format!("the plan has no fix named {id}"))?;
        ensure!(fix.outcome.is_none(), "the fix {id} is already settled");
        Ok(fix)
    }

    /// The stage that comes next. `command` is the command of the ledger, such as `untangle`.
    fn next(&self, command: &str) -> Next {
        if self.finished {
            return Next {
                stage: "finished",
                text: format!("The run is finished. `xpd {command} start` starts another."),
            };
        }
        let Some(fix) = self.open() else {
            return Next {
                stage: "plan",
                text: format!(
                    "Read the ledger with `xpd {command} --format json` and judge each finding \
                     against the code. Group the findings that have one cause under one fix, \
                     and accept a finding that is right as it is with a reason. Write the plan \
                     as JSON, {{\"fixes\": [{{\"id\": \"short-name\", \"title\": \"...\", \
                     \"findings\": [\"<id>\"], \"changes_interface\": false}}], \"accepted\": \
                     [{{\"finding\": \"<id>\", \"reason\": \"...\"}}]}}, with the fixes in the \
                     order to make them, and run `xpd {command} plan <file>`. When no finding \
                     is worth a fix, run `xpd {command} finish`."
                ),
            };
        };
        let id = &fix.id;
        if fix.changes_interface && !fix.approved {
            return Next {
                stage: "ask",
                text: format!(
                    "The fix {id}, \"{}\", changes what callers or users see. Ask the user. \
                     After a yes run `xpd {command} approve {id}`, and after a no run \
                     `xpd {command} drop {id} --reason \"the user said no\"`.",
                    fix.title
                ),
            };
        }
        if fix.attempts.len() >= ATTEMPTS {
            return Next {
                stage: "drop",
                text: format!(
                    "The fix {id} failed its gate {} times. Restore the files with \
                     `git reset --hard {}` and run `xpd {command} drop {id} --reason \"...\"`.",
                    fix.attempts.len(),
                    self.current
                ),
            };
        }
        Next {
            stage: "fix",
            text: if fix.findings.is_empty() {
                format!(
                    "Make the fix {id}, \"{}\", commit it, and run `xpd {command} gate {id}`. \
                     The gate keeps the fix when each check passes as it did at the start, \
                     the goal is met, and the count of findings is not higher than at {}.",
                    fix.title,
                    &self.current[..self.current.len().min(12)]
                )
            } else {
                format!(
                    "Make the fix {id}, \"{}\", which removes the findings {}. Keep the behavior \
                     the same, commit it, and run `xpd {command} gate {id}`. The gate keeps the \
                     fix when each check passes as it did at the start, a finding of the fix is \
                     gone, and the count of findings is not higher than at {}.",
                    fix.title,
                    fix.findings.join(", "),
                    &self.current[..self.current.len().min(12)]
                )
            },
        }
    }

    fn text(&self, command: &str) -> String {
        let mut text = String::new();
        let _ = writeln!(
            text,
            "xpd {command} run: started at {} with {} findings ({} lines), now at {} ({} lines)",
            &self.base[..self.base.len().min(12)],
            self.was,
            self.was_lines,
            &self.current[..self.current.len().min(12)],
            self.current_lines,
        );
        for check in &self.checks {
            let state = if check.passed {
                "passed"
            } else {
                "failed at the start, so it gates nothing"
            };
            let _ = writeln!(text, "  check: {} ({state})", check.command);
        }
        if let Some(bench) = &self.bench {
            let _ = writeln!(
                text,
                "  bench: {} ({:.2} ms -> {:.2} ms, {} runs)",
                bench.command, bench.baseline.median_ms, bench.current.median_ms, bench.runs
            );
        }
        for (round, fixes) in self.rounds.iter().enumerate() {
            let _ = writeln!(text, "  round {}", round + 1);
            for fix in fixes {
                let state = match &fix.outcome {
                    Some(Outcome::Kept {
                        fixed,
                        added,
                        lines_before,
                        lines_after,
                        timing_before,
                        timing_after,
                        ..
                    }) => {
                        let line_delta = format_line_delta(*lines_before, *lines_after);
                        let bench_delta = match (timing_before, timing_after) {
                            (Some(before), Some(after)) => {
                                format!(", {}", format_speedup(*before, *after))
                            }
                            _ => String::new(),
                        };
                        format!(
                            "kept, {} fixed, {} added{line_delta}{bench_delta}",
                            fixed.len(),
                            added.len()
                        )
                    }
                    Some(Outcome::Dropped { reason }) => format!("dropped: {reason}"),
                    None => format!("open, {} failed gates", fix.attempts.len()),
                };
                let _ = writeln!(text, "    {}  {}  ({state})", fix.id, fix.title);
            }
        }
        let _ = writeln!(
            text,
            "  {} findings accepted with a reason",
            self.accepted.len()
        );
        let _ = writeln!(text, "Next: {}", self.next(command).text);
        text
    }
}

/// Formats a line count change when the two counts differ.
fn format_line_delta(before: usize, after: usize) -> String {
    if before == 0 && after == 0 || before == after {
        return String::new();
    }
    if after > before {
        format!(", +{} lines", after - before)
    } else {
        format!(", -{} lines", before - after)
    }
}

/// Formats a benchmark comparison between two median timings.
fn format_speedup(before: Timing, after: Timing) -> String {
    if before.median_ms > 0.0 && after.median_ms > 0.0 {
        let ratio = before.median_ms / after.median_ms;
        format!(
            "{:.2} ms -> {:.2} ms ({ratio:.2}x)",
            before.median_ms, after.median_ms
        )
    } else {
        format!("{:.2} ms -> {:.2} ms", before.median_ms, after.median_ms)
    }
}

/// Total lines across all code files under `paths`.
pub(super) fn count_lines(paths: &[PathBuf]) -> usize {
    files(paths)
        .into_iter()
        .map(|(path, _)| fs::read_to_string(&path).map_or(0, |text| text.lines().count()))
        .sum()
}

/// Rounds a millisecond measurement to two decimal places.
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

fn round_ms(ms: f64) -> f64 {
    (ms * 100.0).round() / 100.0
}

/// Times `command` with one warmup run followed by `runs` measured runs.
pub(super) fn measure_bench(command: &str, runs: usize) -> Result<Timing> {
    let runs = runs.max(1);
    let (passed, printed) = passes(command);
    ensure!(
        passed,
        "benchmark `{command}` failed during warmup:\n{printed}"
    );
    let mut samples = Vec::with_capacity(runs);
    for run in 1..=runs {
        let started = Instant::now();
        let (passed, printed) = passes(command);
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

/// Why a gate cannot judge a fix that cites `findings`, when the commit before the fix lacks
/// any of them. Such an id cannot be gone after the fix either, so without this rule the gate
/// would count it as removed and keep a fix that removed nothing.
pub(super) fn cited_missing(findings: &[String], before: &BTreeSet<String>) -> Option<String> {
    let missing: Vec<&str> = (findings.iter())
        .filter(|id| !before.contains(*id))
        .map(String::as_str)
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "The commit before the fix has no finding {}, so the gate cannot tell whether the fix removed it. Drop the fix, and plan it again with the ids that the ledger prints now.",
            missing.join(", ")
        )
    })
}

/// What a gate found, with no file or command in it.
#[cfg(not(feature = "ledger-durable"))]
#[derive(Debug)]
struct Compared {
    fixed: Vec<String>,
    added: Vec<String>,
}

/// Whether a fix is kept. `before` and `after` hold the ids of the findings at the commit
/// before the fix and after it, and `broken` the checks that passed at the start and fail now.
#[cfg(not(feature = "ledger-durable"))]
fn judge(
    fix: &Fix,
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    broken: &[String],
    lines: (usize, usize),
    timing: Option<(Timing, Timing)>,
) -> Result<Compared, String> {
    if !broken.is_empty() {
        return Err(format!(
            "These checks passed at the start and fail now: {}.",
            broken.join("; ")
        ));
    }
    if let Some(reason) = cited_missing(&fix.findings, before) {
        return Err(reason);
    }
    if !fix.findings.is_empty() && !fix.findings.iter().any(|id| !after.contains(id)) {
        return Err("None of the findings the fix was to remove is gone.".to_owned());
    }
    if after.len() > before.len() {
        return Err(format!(
            "The count of findings rose from {} to {}. The new ones: {}.",
            before.len(),
            after.len(),
            after
                .difference(before)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(goal) = fix.goal {
        let (lines_before, lines_after) = lines;
        if matches!(goal, Goal::Shorter | Goal::Both) && lines_after >= lines_before {
            return Err(format!(
                "The goal is shorter code, but lines went from {lines_before} to {lines_after}."
            ));
        }
        if matches!(goal, Goal::Faster | Goal::Both) {
            let Some((tb, ta)) = timing else {
                return Err("The goal is faster code, which needs `--bench` on `start`.".to_owned());
            };
            if tb.median_ms > 0.0 && ta.median_ms > tb.median_ms * MIN_SPEEDUP {
                return Err(format!(
                    "The goal is faster code, but benchmark median went from {:.2} ms to {:.2} ms.",
                    tb.median_ms, ta.median_ms
                ));
            }
        }
    }
    if let Some((tb, ta)) = timing
        && tb.median_ms > 0.0
        && ta.median_ms > tb.median_ms * MAX_SLOWDOWN
        && ta.median_ms - tb.median_ms > 5.0
        && ta.min_ms > tb.max_ms
    {
        return Err(format!(
            "Benchmark regressed from {:.2} ms to {:.2} ms ({:.2}x slower).",
            tb.median_ms,
            ta.median_ms,
            ta.median_ms / tb.median_ms
        ));
    }
    Ok(Compared {
        fixed: before.difference(after).cloned().collect(),
        added: after.difference(before).cloned().collect(),
    })
}

/// The repository of a run and where its journal is.
struct Place {
    top: PathBuf,
    journal: PathBuf,
}

impl Place {
    /// The repository that holds the working folder. The process moves to its top, so that
    /// the paths of the journal and the checks start there.
    fn find(command: &str) -> Result<Self> {
        let top = repository(Path::new("."))
            .context("this folder is in no git repository, which a run needs")?;
        std::env::set_current_dir(&top)
            .with_context(|| format!("cannot enter {}", top.display()))?;
        let journal = git_path(&top, &format!("xpd-{command}.json"))?;
        Ok(Self { top, journal })
    }

    /// The path of `name` in the git folder of the worktree.
    #[cfg(feature = "ledger-durable")]
    fn git_path(&self, name: &str) -> Result<PathBuf> {
        git_path(&self.top, name)
    }

    #[cfg(not(feature = "ledger-durable"))]
    fn read(&self, command: &str) -> Result<Journal> {
        ensure!(
            self.journal.is_file(),
            "no run is open. Start one with `xpd {command} start`"
        );
        let text = fs::read_to_string(&self.journal)
            .with_context(|| format!("cannot read {}", self.journal.display()))?;
        serde_json::from_str(&text)
            .with_context(|| format!("cannot read {}", self.journal.display()))
    }

    #[cfg(not(feature = "ledger-durable"))]
    fn write(&self, journal: &Journal) -> Result<()> {
        fs::write(&self.journal, serde_json::to_string_pretty(journal)?)
            .with_context(|| format!("cannot write {}", self.journal.display()))
    }

    fn head(&self) -> Result<String> {
        Ok(git::output(&self.top, &["rev-parse", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// Fails when a tracked file differs from the commit, since a gate measures the commit.
    fn clean(&self) -> Result<()> {
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
fn git_path(top: &Path, name: &str) -> Result<PathBuf> {
    let path = git::output(top, &["rev-parse", "--git-path", name])?;
    Ok(top.join(path.trim()))
}

/// Whether `command` exits 0 in the working folder, with the end of what it printed.
pub(super) fn passes(command: &str) -> (bool, String) {
    let mut shell = if cfg!(windows) {
        let mut shell = Command::new("cmd");
        shell.arg("/C");
        shell
    } else {
        let mut shell = Command::new("sh");
        shell.arg("-c");
        shell
    };
    match shell.arg(command).stdin(Stdio::null()).output() {
        Ok(output) => {
            let mut printed = String::from_utf8_lossy(&output.stdout).into_owned();
            printed.push_str(&String::from_utf8_lossy(&output.stderr));
            let from = printed.len().saturating_sub(OUTPUT_KEPT);
            let from = (from..printed.len())
                .find(|at| printed.is_char_boundary(*at))
                .unwrap_or(printed.len());
            (
                output.status.success(),
                printed[from..].trim_end().to_owned(),
            )
        }
        Err(error) => (false, format!("cannot start: {error}")),
    }
}

/// The ids of the findings of a run at a commit, or of the files of today for `None`.
type Found<'f> = &'f dyn Fn(&Journal, Option<&str>) -> Result<BTreeSet<String>>;

/// The findings at a commit, or in the working tree for `None`, as a map from the id that
/// xpd 0.55.0 gave each one to its id of today.
#[cfg(feature = "ledger-durable")]
type Renamed<'r> = &'r dyn Fn(&Journal, Option<&str>) -> Result<BTreeMap<String, String>>;

/// A new journal for the paths, which are given from the folder `here`.
fn start(
    place: &Place,
    here: &Path,
    paths: &[PathBuf],
    checks: Vec<String>,
    bench: Option<(String, usize)>,
    found: Found<'_>,
) -> Result<Journal> {
    place.clean()?;
    let paths = (paths.iter())
        .map(|path| {
            let absolute = fs::canonicalize(here.join(path))
                .with_context(|| format!("cannot read {}", path.display()))?;
            let below = absolute
                .strip_prefix(&place.top)
                .with_context(|| format!("{} is outside the repository", path.display()))?;
            Ok(if below.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                below.to_path_buf()
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let base = place.head()?;
    let lines = count_lines(&paths);
    let bench = bench
        .map(|(command, runs)| -> Result<Bench> {
            let timing = measure_bench(&command, runs)?;
            Ok(Bench {
                command,
                runs: timing.runs,
                baseline: timing,
                current: timing,
                host_fingerprint: Some(host_fingerprint()),
            })
        })
        .transpose()?;
    let mut journal = Journal {
        current: base.clone(),
        base,
        paths,
        checks: (checks.into_iter())
            .map(|command| Check {
                passed: passes(&command).0,
                command,
            })
            .collect(),
        bench,
        was: 0,
        was_lines: lines,
        current_lines: lines,
        rounds: Vec::new(),
        accepted: BTreeMap::new(),
        finished: false,
    };
    journal.was = found(&journal, None)?.len();
    Ok(journal)
}

/// Reads the plan file, or stdin for `-`. `file` is given from the folder `here`.
fn plan_text(here: &Path, file: &Path) -> Result<String> {
    if file == Path::new("-") {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        fs::read_to_string(here.join(file))
            .with_context(|| format!("cannot read {}", file.display()))
    }
}

/// Reads the plan in `text`, when the run is at a stage that takes a plan.
fn read_plan(journal: &Journal, text: &str, command: &str) -> Result<Plan> {
    ensure!(!journal.finished, "the run is finished");
    if let Some(open) = journal.open() {
        bail!(
            "the fix {} is still open. Gate it or drop it before the next plan",
            open.id
        );
    }
    serde_json::from_str(text)
        .with_context(|| format!("the plan is not the JSON that `xpd {command} plan --help` shows"))
}

/// Adds `plan` to the journal, after checking it against `known`, the ids of the ledger of today.
#[cfg(not(feature = "ledger-durable"))]
fn plan(journal: &mut Journal, plan: Plan, known: &BTreeSet<String>) -> Result<()> {
    let mut named: BTreeSet<&str> = journal.fixes().map(|fix| fix.id.as_str()).collect();
    for fix in &plan.fixes {
        ensure!(named.insert(&fix.id), "two fixes are named {}", fix.id);
        ensure!(
            !fix.findings.is_empty() || fix.goal.is_some(),
            "the fix {} names no finding and sets no goal",
            fix.id
        );
        if matches!(fix.goal, Some(Goal::Faster | Goal::Both)) {
            ensure!(
                journal.bench.is_some(),
                "the fix {} has a speed goal, which needs `--bench` on `start`",
                fix.id
            );
        }
        ensure!(
            fix.outcome.is_none() && fix.attempts.is_empty() && !fix.approved,
            "the fix {} holds fields that only xpd writes",
            fix.id
        );
    }
    let cited = (plan.fixes.iter().flat_map(|fix| &fix.findings))
        .chain(plan.accepted.iter().map(|accepted| &accepted.finding));
    for id in cited {
        ensure!(
            known.contains(id),
            "the ledger of today has no finding {id}"
        );
    }
    for accepted in plan.accepted {
        ensure!(
            !accepted.reason.trim().is_empty(),
            "the accepted finding {} has no reason",
            accepted.finding
        );
        journal.accepted.insert(accepted.finding, accepted.reason);
    }
    if !plan.fixes.is_empty() {
        journal.rounds.push(plan.fixes);
    }
    Ok(())
}

/// Fails unless `id` is the open fix, and the user said yes to it when it changes an interface.
fn check_gate(journal: &Journal, id: &str) -> Result<()> {
    let open = journal.open().map(|open| open.id.clone());
    ensure!(
        open.as_deref() == Some(id),
        "the fix that is open is {}",
        open.as_deref().unwrap_or("none")
    );
    let asked = journal
        .open()
        .is_some_and(|fix| fix.changes_interface && !fix.approved);
    ensure!(
        !asked,
        "the fix {id} changes what callers or users see, and the user has not said yes. Ask the user first"
    );
    Ok(())
}

/// What a gate measures of the commit that is checked out. The fields have the names of the
/// gate output of the orchestration.
#[derive(Debug, Serialize)]
struct Measured {
    commit: String,
    /// The checks that passed at the start and fail now, each with the end of its output.
    broken: Vec<String>,
    /// The ids of the findings at the commit the run stands at.
    before: BTreeSet<String>,
    /// The ids of the findings of today.
    after: BTreeSet<String>,
    lines_before: usize,
    lines_after: usize,
    timing: Option<(Timing, Timing)>,
}

/// Measures the commit that is checked out against the commit the run stands at.
fn measure_gate(journal: &Journal, place: &Place, found: Found<'_>) -> Result<Measured> {
    place.clean()?;
    let commit = place.head()?;
    ensure!(
        commit != journal.current,
        "no commit was made since {}",
        journal.current
    );
    let broken: Vec<String> = (journal.checks.iter())
        .filter(|check| check.passed)
        .filter_map(|check| {
            let (passed, printed) = passes(&check.command);
            (!passed).then(|| format!("{}\n{printed}", check.command))
        })
        .collect();
    let before = found(journal, Some(&journal.current))?;
    let after = found(journal, None)?;
    let lines_before = journal.current_lines;
    let lines_after = count_lines(&journal.paths);
    let timing = if broken.is_empty()
        && let Some(bench) = &journal.bench
    {
        let after_timing = measure_bench(&bench.command, bench.runs)?;
        Some((bench.current, after_timing))
    } else {
        None
    };
    Ok(Measured {
        commit,
        broken,
        before,
        after,
        lines_before,
        lines_after,
        timing,
    })
}

/// Gates the fix `id` at the commit that is checked out. Returns whether the fix is kept.
#[cfg(not(feature = "ledger-durable"))]
fn gate(journal: &mut Journal, place: &Place, id: &str, found: Found<'_>) -> Result<bool> {
    check_gate(journal, id)?;
    let measured = measure_gate(journal, place, found)?;
    let fix = journal.fix_mut(id)?;
    match judge(
        fix,
        &measured.before,
        &measured.after,
        &measured.broken,
        (measured.lines_before, measured.lines_after),
        measured.timing,
    ) {
        Ok(Compared { fixed, added }) => {
            fix.outcome = Some(Outcome::Kept {
                commit: measured.commit.clone(),
                fixed,
                added,
                lines_before: measured.lines_before,
                lines_after: measured.lines_after,
                timing_before: measured.timing.map(|(before, _)| before),
                timing_after: measured.timing.map(|(_, after)| after),
            });
            journal.current = measured.commit;
            journal.current_lines = measured.lines_after;
            if let (Some(bench), Some((_, after_timing))) = (&mut journal.bench, measured.timing) {
                bench.current = after_timing;
            }
            Ok(true)
        }
        Err(reason) => {
            eprintln!("xpd: the gate did not keep {id}. {reason}");
            fix.attempts.push(reason);
            Ok(false)
        }
    }
}

/// What `finish` measures. The fields have the names of the finish output of the
/// orchestration.
#[derive(Debug, Serialize)]
struct Surveyed {
    /// The ids of the findings at the commit the run started from.
    before: BTreeSet<String>,
    /// The ids of the findings of today.
    after: BTreeSet<String>,
    /// The lines of code of today in the paths of the run.
    current_lines: usize,
}

/// Measures the files of today against the commit the run started from.
fn measure_finish(journal: &Journal, place: &Place, found: Found<'_>) -> Result<Surveyed> {
    if let Some(open) = journal.open() {
        bail!("the fix {} is still open", open.id);
    }
    place.clean()?;
    Ok(Surveyed {
        before: found(journal, Some(&journal.base))?,
        after: found(journal, None)?,
        current_lines: count_lines(&journal.paths),
    })
}

/// The findings that are new since the start and have no reason.
fn unexplained<'s>(journal: &Journal, surveyed: &'s Surveyed) -> Vec<&'s str> {
    (surveyed.after.difference(&surveyed.before))
        .filter(|id| !journal.accepted.contains_key(*id))
        .map(String::as_str)
        .collect()
}

fn report_unexplained(unexplained: &[&str]) {
    eprintln!(
        "xpd: these findings are new since the start and have no reason: {}. Remove them, or accept them in a plan",
        unexplained.join(", ")
    );
}

/// Prints what a finished run changed, from the start commit to the files of today.
fn report_finish(journal: &Journal, surveyed: &Surveyed) {
    let Surveyed {
        before,
        after,
        current_lines,
    } = surveyed;
    let bench_summary = journal.bench.as_ref().map_or_else(String::new, |bench| {
        format!(", bench {}", format_speedup(bench.baseline, bench.current))
    });
    eprintln!(
        "xpd: {} findings at the start, {} now: {} fixed, {} added, {} left without a reason; lines {} -> {current_lines}{bench_summary}",
        before.len(),
        after.len(),
        before.difference(after).count(),
        after.difference(before).count(),
        after
            .iter()
            .filter(|id| !journal.accepted.contains_key(*id))
            .count(),
        journal.was_lines,
    );
}

/// Ends the run when no fix is open and each finding that is new since the start has a
/// reason. Returns whether the run ended.
#[cfg(not(feature = "ledger-durable"))]
fn finish(journal: &mut Journal, place: &Place, found: Found<'_>) -> Result<bool> {
    let surveyed = measure_finish(journal, place, found)?;
    let unexplained = unexplained(journal, &surveyed);
    if !unexplained.is_empty() {
        report_unexplained(&unexplained);
        return Ok(false);
    }
    journal.current_lines = surveyed.current_lines;
    report_finish(journal, &surveyed);
    journal.finished = true;
    Ok(true)
}

/// Runs one stage on the JSON journal of the worktree. Returns the run and whether the stage
/// kept what it measured.
#[cfg(not(feature = "ledger-durable"))]
fn act_journal(
    action: Action,
    place: &Place,
    here: &Path,
    command: &str,
    found: Found<'_>,
) -> Result<(Journal, bool)> {
    let mut kept = true;
    let journal = match action {
        Action::Start {
            paths,
            checks,
            bench,
            bench_runs,
            restart,
        } => {
            if let Ok(old) = place.read(command) {
                ensure!(
                    old.finished || restart,
                    "a run is open. Go on with `xpd {command} status`, or replace it with --restart"
                );
            }
            start(
                place,
                here,
                &paths,
                checks,
                bench.map(|cmd| (cmd, bench_runs)),
                found,
            )?
        }
        Action::Plan { file } => {
            let mut journal = place.read(command)?;
            let text = plan_text(here, &file)?;
            let read = read_plan(&journal, &text, command)?;
            let known = found(&journal, None)?;
            plan(&mut journal, read, &known)?;
            journal
        }
        Action::Approve { fix } => {
            let mut journal = place.read(command)?;
            journal.fix_mut(&fix)?.approved = true;
            journal
        }
        Action::Drop { fix, reason } => {
            let mut journal = place.read(command)?;
            ensure!(
                place.head()? == journal.current,
                "the commit is not {}, where the run stands. Restore it with `git reset --hard {}`",
                journal.current,
                journal.current
            );
            journal.fix_mut(&fix)?.outcome = Some(Outcome::Dropped { reason });
            journal
        }
        Action::Gate { fix } => {
            let mut journal = place.read(command)?;
            kept = gate(&mut journal, place, &fix, found)?;
            journal
        }
        Action::Status => place.read(command)?,
        Action::Finish => {
            let mut journal = place.read(command)?;
            kept = finish(&mut journal, place, found)?;
            journal
        }
    };
    place.write(&journal)?;
    Ok((journal, kept))
}

/// A run as a `duroxide` orchestration in the ledger store of the repository.
///
/// Each worktree has its own namespace in the shared store, and each start of a run takes the
/// next generation of that namespace, so the instance of a run is
/// `wt-<worktree>::<command>::<generation>` and the run that counts is the highest generation.
/// A lock file in the git folder of the worktree makes one process at a time its writer.
#[cfg(feature = "ledger-durable")]
mod durable_run {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::{self, File, TryLockError};
    use std::hash::Hasher as _;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use anyhow::{Context, Result, bail, ensure};
    use duroxide::runtime::registry::ActivityRegistry;
    use duroxide::{ActivityContext, OrchestrationStatus};
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use siphasher::sip::SipHasher13;

    use super::super::durable::{
        LazyTokio, LedgerOrchestratorClient, ProviderConfig, instance_id, open_store, run_command,
        start_command,
    };
    use super::super::orchestration::{
        ACT_FINISH_RUN, ACT_GATE_FIX, ACT_INIT_BASELINE, ACT_VALIDATE_PLAN, BaselineOutput,
        Command as RunCommand, CommandEnvelope, FinishOutput, GateOutput, JournalState, Receipt,
        ReceiptOutcome, RunStartInput, Snapshot, ValidatePlanOutput,
    };
    use super::{
        Action, Found, Journal, Outcome, Place, Renamed, check_gate, measure_finish, measure_gate,
        plan_text, read_plan, report_finish, report_unexplained, start, unexplained,
    };

    /// How long a command waits for the receipt of the run. Every measurement is taken before
    /// the command is queued, so a turn takes milliseconds, not the time of the checks.
    const WAIT: Duration = Duration::from_secs(120);
    /// The key of the hash that names the import of a JSON journal. It never changes, so a
    /// retried import finds the receipt of the first try.
    const IMPORT_KEY: [u8; 16] = *b"xpd-ledger-impor";

    /// The ledger store of one command in one worktree.
    struct Store {
        config: ProviderConfig,
        namespace: String,
        command: &'static str,
    }

    /// Where the highest generation of a worktree's run stands.
    enum Standing {
        Running(Option<Snapshot>),
        Completed(Option<Snapshot>),
        Failed(String),
    }

    /// The highest generation of a worktree's run.
    struct Latest {
        generation: u64,
        standing: Standing,
    }

    impl Store {
        fn open(place: &Place, command: &'static str) -> Result<Self> {
            let git_dir = git_folder(place, "--absolute-git-dir")?;
            let common = git_folder(place, "--git-common-dir")?;
            let namespace = if git_dir == common {
                "wt-main".to_owned()
            } else {
                let below = git_dir.strip_prefix(&common).unwrap_or(&git_dir);
                format!("wt-{}", below.display())
            };
            Ok(Self {
                config: ProviderConfig::for_repository(&common),
                namespace,
                command,
            })
        }

        fn instance(&self, generation: u64) -> String {
            instance_id(&self.namespace, self.command, generation)
        }

        /// Measurements for the run of `generation`, with none taken yet.
        fn measurements(&self, generation: u64) -> Measurements {
            Measurements {
                instance: self.instance(generation),
                ..Measurements::default()
            }
        }

        /// The lock file of this command's runs in the worktree, and its path.
        fn lock_file(&self, place: &Place) -> Result<(File, PathBuf)> {
            let path = place.git_path(&format!("xpd-{}.lock", self.command))?;
            let file = File::options()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&path)
                .with_context(|| format!("cannot open {}", path.display()))?;
            Ok((file, path))
        }

        /// Takes the lock for a command that changes the run. It fails at once while another
        /// process holds it, so two commands never change one run at the same time.
        fn lock_writer(&self, place: &Place) -> Result<File> {
            let (file, path) = self.lock_file(place)?;
            match file.try_lock() {
                Ok(()) => Ok(file),
                Err(TryLockError::WouldBlock) => bail!(
                    "another `xpd {}` command is changing the run of this worktree. Run this one when it is done",
                    self.command
                ),
                Err(TryLockError::Error(error)) => {
                    Err(error).with_context(|| format!("cannot lock {}", path.display()))
                }
            }
        }

        /// Takes the lock for `status` when it has a JSON journal to import, waiting its turn.
        fn lock_reader(&self, place: &Place) -> Result<File> {
            let (file, path) = self.lock_file(place)?;
            file.lock()
                .with_context(|| format!("cannot lock {}", path.display()))?;
            Ok(file)
        }

        /// The highest generation of the run, read through a client alone.
        fn latest(&self) -> Result<Option<Latest>> {
            let config = self.config.clone();
            let namespace = self.namespace.clone();
            let command = self.command;
            LazyTokio::new().block_on(async move {
                let store = open_store(&config).await?;
                let client = |generation| {
                    LedgerOrchestratorClient::new(
                        Arc::clone(&store),
                        instance_id(&namespace, command, generation),
                    )
                };
                let mut highest = None;
                for generation in 0_u64.. {
                    match client(generation).status().await? {
                        OrchestrationStatus::NotFound => break,
                        status => highest = Some((generation, status)),
                    }
                }
                let Some((generation, status)) = highest else {
                    return Ok(None);
                };
                let standing = match status {
                    OrchestrationStatus::Failed { details, .. } => {
                        Standing::Failed(format!("{details:?}"))
                    }
                    OrchestrationStatus::Completed { .. } => {
                        Standing::Completed(client(generation).snapshot().await?)
                    }
                    _ => Standing::Running(client(generation).snapshot().await?),
                };
                Ok::<_, anyhow::Error>(Some(Latest {
                    generation,
                    standing,
                }))
            })?
        }

        /// Whether the run of `generation` filed a receipt for `command_id`.
        fn has_receipt(&self, generation: u64, command_id: &str) -> Result<bool> {
            let config = self.config.clone();
            let instance = self.instance(generation);
            let command_id = command_id.to_owned();
            LazyTokio::new().block_on(async move {
                let store = open_store(&config).await?;
                let client = LedgerOrchestratorClient::new(store, instance);
                Ok::<_, anyhow::Error>(client.receipt(&command_id).await?.is_some())
            })?
        }

        /// Asks the run of `generation` to stop, as a restart does with the run it replaces.
        fn cancel(&self, generation: u64) -> Result<()> {
            let config = self.config.clone();
            let instance = self.instance(generation);
            LazyTokio::new().block_on(async move {
                let store = open_store(&config).await?;
                LedgerOrchestratorClient::new(store, instance)
                    .cancel("restart")
                    .await
            })?
        }

        /// Queues one command behind the commands before it, and waits for its receipt.
        fn send(
            &self,
            generation: u64,
            measurements: Measurements,
            kind: &str,
            command: RunCommand,
        ) -> Result<(Journal, Receipt)> {
            let envelope = CommandEnvelope {
                command_id: command_id(kind),
                command,
            };
            let (snapshot, receipt) = run_command(
                &self.config,
                &self.instance(generation),
                measurements.registry(),
                &envelope,
                WAIT,
            )?;
            Ok((journal_of(&snapshot.run)?, receipt))
        }
    }

    /// A git folder of the worktree, as an absolute path.
    fn git_folder(place: &Place, flag: &str) -> Result<PathBuf> {
        let folder = crate::git::output(&place.top, &["rev-parse", flag])?;
        let folder = place.top.join(folder.trim());
        fs::canonicalize(&folder).with_context(|| format!("cannot read {}", folder.display()))
    }

    /// The measurements this process took, which the activities of its own run return.
    ///
    /// The CLI measures before it queues a command, as the journal did, so an activity only
    /// hands back what was measured. An activity of another instance, which a runtime in this
    /// process can pick up from the shared store, gets an error and never this run's numbers.
    #[derive(Debug, Default)]
    struct Measurements {
        instance: String,
        baseline: Option<(String, BaselineOutput)>,
        known: Option<ValidatePlanOutput>,
        gate: Option<(String, GateOutput)>,
        finish: Option<FinishOutput>,
    }

    impl Measurements {
        fn serve(&self, ctx: &ActivityContext, input: &str) -> Result<String, String> {
            let name = ctx.activity_name();
            if ctx.instance_id() != self.instance {
                return Err(format!(
                    "this xpd process serves {}, not {}. Run the command again in the worktree of that run",
                    self.instance,
                    ctx.instance_id()
                ));
            }
            let encode = |output: &dyn erased::Encode| output.encode();
            let served = match name {
                ACT_INIT_BASELINE => self
                    .baseline
                    .as_ref()
                    .filter(|(id, _)| {
                        serde_json::from_str::<RunStartInput>(input)
                            .is_ok_and(|start| start.command_id == *id)
                    })
                    .map(|(_, output)| encode(output)),
                ACT_VALIDATE_PLAN => self.known.as_ref().map(|output| encode(output)),
                ACT_GATE_FIX => self
                    .gate
                    .as_ref()
                    .filter(|(fix, output)| {
                        serde_json::from_str::<serde_json::Value>(input).is_ok_and(|asked| {
                            asked["fix"] == fix.as_str() && asked["head"] == output.commit.as_str()
                        })
                    })
                    .map(|(_, output)| encode(output)),
                ACT_FINISH_RUN => self.finish.as_ref().map(|output| encode(output)),
                _ => None,
            };
            served.unwrap_or_else(|| {
                Err(format!(
                    "this xpd process took no measurement for {name}. Run the command again"
                ))
            })
        }

        /// The activities of a run, each a static entry that returns what was measured.
        fn registry(self) -> ActivityRegistry {
            let this = Arc::new(self);
            let mut builder = ActivityRegistry::builder();
            for name in [
                ACT_INIT_BASELINE,
                ACT_VALIDATE_PLAN,
                ACT_GATE_FIX,
                ACT_FINISH_RUN,
            ] {
                let this = Arc::clone(&this);
                builder = builder.register(name, move |ctx: ActivityContext, input: String| {
                    let served = this.serve(&ctx, &input);
                    async move { served }
                });
            }
            builder.build()
        }
    }

    /// Encoding of an activity output, behind one object type.
    mod erased {
        pub(super) trait Encode {
            fn encode(&self) -> Result<String, String>;
        }

        impl<T: serde::Serialize> Encode for T {
            fn encode(&self) -> Result<String, String> {
                serde_json::to_string(self).map_err(|error| error.to_string())
            }
        }
    }

    /// One value as another type of the same JSON shape.
    fn convert<T: DeserializeOwned>(value: &impl Serialize) -> Result<T> {
        serde_json::from_value(serde_json::to_value(value)?)
            .context("the run and the ledger store disagree on the shape of a run")
    }

    /// The journal of a mirrored run. It prints with the texts of this module.
    fn journal_of(state: &JournalState) -> Result<Journal> {
        convert(state)
    }

    /// A command id that no other command of any process takes.
    fn command_id(kind: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        format!("{kind}-{}-{nanos}", std::process::id())
    }

    /// The command id of the import of the JSON journal `text`. A retry of the same import
    /// gets the same id, so it finds the receipt of the first try.
    fn import_id(text: &str) -> String {
        let mut hasher = SipHasher13::new_with_key(&IMPORT_KEY);
        hasher.write(text.as_bytes());
        format!("import-{:016x}", hasher.finish())
    }

    /// Where a JSON journal goes once the store holds its run.
    fn migrated(journal: &Path) -> PathBuf {
        let mut name = journal.as_os_str().to_owned();
        name.push(".migrated");
        PathBuf::from(name)
    }

    fn remove_if_present(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error).with_context(|| format!("cannot remove {}", path.display()))
            }
            _ => Ok(()),
        }
    }

    fn applied(receipt: &Receipt) -> Result<()> {
        match receipt.outcome {
            ReceiptOutcome::Applied => Ok(()),
            ReceiptOutcome::Rejected => bail!(
                "{}",
                receipt
                    .detail
                    .as_deref()
                    .unwrap_or("the run turned the command away")
            ),
        }
    }

    /// The run that is open, as a journal, with its generation.
    fn open_run(latest: Option<&Latest>, command: &str) -> Result<(u64, Journal)> {
        let Some(latest) = latest else {
            bail!("no run is open. Start one with `xpd {command} start`");
        };
        let snapshot = match &latest.standing {
            Standing::Failed(details) => bail!(
                "the run failed in the ledger store: {details}. Replace it with `xpd {command} start --restart`"
            ),
            Standing::Running(snapshot) | Standing::Completed(snapshot) => snapshot
                .as_ref()
                .context("the run has not published its state yet. Run the command again")?,
        };
        Ok((latest.generation, journal_of(&snapshot.run)?))
    }

    /// The commits whose findings a journal cites: the start, each kept fix and the current
    /// commit. `None` stands for the working tree, which a plan is checked against.
    fn cited_commits(journal: &Journal) -> BTreeSet<Option<&str>> {
        let kept = journal.fixes().filter_map(|fix| match &fix.outcome {
            Some(Outcome::Kept { commit, .. }) => Some(commit.as_str()),
            _ => None,
        });
        ([journal.base.as_str(), journal.current.as_str()].into_iter())
            .chain(kept)
            .map(Some)
            .chain([None])
            .collect()
    }

    /// Rewrites each finding id that `journal` cites to the id of today. `renamed` maps the id
    /// that xpd 0.55.0 gave a finding to its id of today. An id that is already an id of today,
    /// because a newer xpd wrote the journal, stays. Fails, and names them, when some ids
    /// match neither.
    fn rewrite_ids(journal: &mut Journal, renamed: &BTreeMap<String, String>) -> Result<()> {
        let today: BTreeSet<&String> = renamed.values().collect();
        let mut unmatched = BTreeSet::new();
        let mut rewrite = |id: &mut String| {
            if let Some(new) = renamed.get(id.as_str()) {
                new.clone_into(id);
            } else if !today.contains(id) {
                unmatched.insert(id.clone());
            }
        };
        for fix in journal.rounds.iter_mut().flatten() {
            fix.findings.iter_mut().for_each(&mut rewrite);
            if let Some(Outcome::Kept { fixed, added, .. }) = &mut fix.outcome {
                fixed
                    .iter_mut()
                    .chain(added.iter_mut())
                    .for_each(&mut rewrite);
            }
        }
        journal.accepted = std::mem::take(&mut journal.accepted)
            .into_iter()
            .map(|(mut id, reason)| {
                rewrite(&mut id);
                (id, reason)
            })
            .collect();
        ensure!(
            unmatched.is_empty(),
            "the run cites findings that none of its commits has: {}",
            unmatched.into_iter().collect::<Vec<_>>().join(", ")
        );
        Ok(())
    }

    /// Moves the JSON journal of this worktree into the store, once.
    ///
    /// The lock of the worktree makes this process the only one that imports it. The import is
    /// started under a command id that the journal text fixes, so a retry after a crash finds
    /// the generation of the first try by its receipt. The file is renamed only when the
    /// store mirrors exactly the run that the file holds.
    ///
    /// An older xpd named findings with another hash, so before the import every id the run
    /// cites is rewritten to the id of today, found by surveying each commit of the run. The
    /// import is refused when an id matches no finding of those commits.
    fn migrate(store: &Store, place: &Place, renamed: Renamed<'_>) -> Result<()> {
        if !place.journal.is_file() {
            return Ok(());
        }
        let shown = place.journal.display();
        let text =
            fs::read_to_string(&place.journal).with_context(|| format!("cannot read {shown}"))?;
        let read: JournalState =
            serde_json::from_str(&text).with_context(|| format!("cannot read {shown}"))?;
        let mut journal = journal_of(&read)?;
        let mut ids = BTreeMap::new();
        for commit in cited_commits(&journal) {
            ids.extend(renamed(&journal, commit)?);
        }
        rewrite_ids(&mut journal, &ids).with_context(|| {
            format!(
                "{shown} stays. Finish its run with the xpd that wrote it, or replace it with `xpd {} start --restart`",
                store.command
            )
        })?;
        let state: JournalState = convert(&journal)?;
        let id = import_id(&text);
        let latest = store.latest()?;
        let generation = match &latest {
            None => 0,
            Some(latest) if store.has_receipt(latest.generation, &id)? => latest.generation,
            Some(Latest {
                standing: Standing::Running(_),
                ..
            }) => bail!(
                "{shown} holds a run, and the ledger store holds another open run of this worktree. Replace both with `xpd {} start --restart`",
                store.command
            ),
            Some(latest) => latest.generation + 1,
        };
        let input = RunStartInput {
            command_id: id,
            command: store.command.to_owned(),
            paths: state.paths.clone(),
            checks: (state.checks.iter())
                .map(|check| check.command.clone())
                .collect(),
            bench: (state.bench.as_ref()).map(|bench| (bench.command.clone(), bench.runs)),
            initial_state: Some(state.clone()),
        };
        let (snapshot, receipt) = start_command(
            &store.config,
            &store.instance(generation),
            store.measurements(generation).registry(),
            &input,
            WAIT,
        )?;
        ensure!(
            receipt.outcome == ReceiptOutcome::Applied && snapshot.run == state,
            "the ledger store did not take the run of {shown} as it is, so the file stays"
        );
        let target = migrated(&place.journal);
        fs::rename(&place.journal, &target).with_context(|| format!("cannot rename {shown}"))?;
        eprintln!(
            "xpd: the run of {shown} is in the ledger store now, and the file is {}",
            target.display()
        );
        Ok(())
    }

    /// Runs one stage on the run in the ledger store. Returns the run and whether the stage
    /// kept what it measured.
    pub(super) fn act(
        action: Action,
        place: &Place,
        here: &Path,
        command: &'static str,
        found: Found<'_>,
        renamed: Renamed<'_>,
    ) -> Result<(Journal, bool)> {
        quiet_runtime_logs();
        let store = Store::open(place, command)?;
        let _lock = match action {
            Action::Status if !place.journal.is_file() => None,
            Action::Status => Some(store.lock_reader(place)?),
            _ => Some(store.lock_writer(place)?),
        };
        if !matches!(action, Action::Start { restart: true, .. }) {
            migrate(&store, place, renamed)?;
        }
        let latest = store.latest()?;
        if let Action::Start {
            paths,
            checks,
            bench,
            bench_runs,
            restart,
        } = action
        {
            let bench = bench.map(|cmd| (cmd, bench_runs));
            let started = Started {
                paths: &paths,
                checks,
                bench,
                restart,
            };
            return start_run(&store, place, here, latest.as_ref(), started, found);
        }
        let (generation, journal) = open_run(latest.as_ref(), command)?;
        let at = At {
            store: &store,
            place,
            generation,
            journal,
        };
        match action {
            Action::Start { .. } => unreachable!("a start returned above"),
            Action::Status => Ok((at.journal, true)),
            Action::Plan { file } => plan_run(&at, &plan_text(here, &file)?, found),
            Action::Approve { fix } => approve_run(at, fix),
            Action::Drop { fix, reason } => drop_run(at, fix, reason),
            Action::Gate { fix } => gate_run(&at, &fix, found),
            Action::Finish => finish_run(&at, found),
        }
    }

    /// Keeps the log lines of the embedded runtime off stdout, where the run is printed.
    ///
    /// A booting duroxide runtime installs a log subscriber that writes to stdout, filtered by
    /// `RUST_LOG` or else at `warn`, so a retried SQLite lock would put a line into the JSON
    /// of the run. Nothing else in xpd reads `RUST_LOG`, and a rejected command reaches the
    /// user through its receipt, not a log line. `set_var` is unsound only while another
    /// thread reads or writes the environment, and this runs at the start of a ledger stage,
    /// which `main` reaches on one thread, before any Tokio runtime, SQLite connection or survey
    /// thread pool exists.
    fn quiet_runtime_logs() {
        // SAFETY: no other thread exists yet, as the doc comment above explains.
        unsafe { std::env::set_var("RUST_LOG", "off") };
    }

    /// What `start` was given.
    struct Started<'a> {
        paths: &'a [PathBuf],
        checks: Vec<String>,
        bench: Option<(String, usize)>,
        restart: bool,
    }

    /// The open run that a stage works on.
    struct At<'a> {
        store: &'a Store,
        place: &'a Place,
        generation: u64,
        journal: Journal,
    }

    impl At<'_> {
        /// Measurements for this run, with none taken yet.
        fn measurements(&self) -> Measurements {
            self.store.measurements(self.generation)
        }

        /// Queues `command` on this run and waits for its receipt.
        fn send(
            &self,
            measurements: Measurements,
            kind: &str,
            command: RunCommand,
        ) -> Result<(Journal, Receipt)> {
            self.store
                .send(self.generation, measurements, kind, command)
        }
    }

    /// Starts a run at the next generation, after cancelling an open one on `--restart`.
    fn start_run(
        store: &Store,
        place: &Place,
        here: &Path,
        latest: Option<&Latest>,
        started: Started<'_>,
        found: Found<'_>,
    ) -> Result<(Journal, bool)> {
        let command = store.command;
        let open = latest.and_then(|latest| {
            matches!(latest.standing, Standing::Running(_)).then_some(latest.generation)
        });
        ensure!(
            open.is_none() || started.restart,
            "a run is open. Go on with `xpd {command} status`, or replace it with --restart"
        );
        let journal = start(
            place,
            here,
            started.paths,
            started.checks,
            started.bench,
            found,
        )?;
        if let Some(open) = open {
            store.cancel(open)?;
        }
        let generation = latest.map_or(0, |latest| latest.generation + 1);
        let id = command_id("start");
        let input = RunStartInput {
            command_id: id.clone(),
            command: command.to_owned(),
            paths: convert(&journal.paths)?,
            checks: (journal.checks.iter())
                .map(|check| check.command.clone())
                .collect(),
            bench: (journal.bench.as_ref()).map(|bench| (bench.command.clone(), bench.runs)),
            initial_state: None,
        };
        let measurements = Measurements {
            baseline: Some((id, convert(&journal)?)),
            ..store.measurements(generation)
        };
        let (snapshot, receipt) = start_command(
            &store.config,
            &store.instance(generation),
            measurements.registry(),
            &input,
            WAIT,
        )?;
        applied(&receipt)?;
        if started.restart {
            remove_if_present(&place.journal)?;
        }
        remove_if_present(&migrated(&place.journal))?;
        Ok((journal_of(&snapshot.run)?, true))
    }

    fn plan_run(at: &At<'_>, text: &str, found: Found<'_>) -> Result<(Journal, bool)> {
        let plan = read_plan(&at.journal, text, at.store.command)?;
        let known = found(&at.journal, None)?;
        let measurements = Measurements {
            known: Some(ValidatePlanOutput { known }),
            ..at.measurements()
        };
        let planned = RunCommand::Plan {
            fixes: convert(&plan.fixes)?,
            accepted: convert(&plan.accepted)?,
        };
        let (journal, receipt) = at.send(measurements, "plan", planned)?;
        applied(&receipt)?;
        Ok((journal, true))
    }

    fn approve_run(mut at: At<'_>, fix: String) -> Result<(Journal, bool)> {
        at.journal.fix_mut(&fix)?;
        let (journal, receipt) =
            at.send(at.measurements(), "approve", RunCommand::Approve { fix })?;
        applied(&receipt)?;
        Ok((journal, true))
    }

    fn drop_run(mut at: At<'_>, fix: String, reason: String) -> Result<(Journal, bool)> {
        let head = at.place.head()?;
        let current = &at.journal.current;
        ensure!(
            head == *current,
            "the commit is not {current}, where the run stands. Restore it with `git reset --hard {current}`"
        );
        at.journal.fix_mut(&fix)?;
        let dropped = RunCommand::Drop { fix, reason, head };
        let (journal, receipt) = at.send(at.measurements(), "drop", dropped)?;
        applied(&receipt)?;
        Ok((journal, true))
    }

    /// Gates the fix `fix`. A gate that the run judged and did not keep returns `false`, as
    /// the journal did, and any other rejection is an error.
    fn gate_run(at: &At<'_>, fix: &str, found: Found<'_>) -> Result<(Journal, bool)> {
        check_gate(&at.journal, fix)?;
        let failed_before = attempts(&at.journal, fix);
        let measured = measure_gate(&at.journal, at.place, found)?;
        let head = measured.commit.clone();
        let measurements = Measurements {
            gate: Some((fix.to_owned(), convert(&measured)?)),
            ..at.measurements()
        };
        let gated = RunCommand::Gate {
            fix: fix.to_owned(),
            head,
        };
        let (journal, receipt) = at.send(measurements, "gate", gated)?;
        if receipt.outcome == ReceiptOutcome::Applied {
            return Ok((journal, true));
        }
        let reason = receipt.detail.unwrap_or_default();
        ensure!(attempts(&journal, fix) > failed_before, "{reason}");
        eprintln!("xpd: the gate did not keep {fix}. {reason}");
        Ok((journal, false))
    }

    fn finish_run(at: &At<'_>, found: Found<'_>) -> Result<(Journal, bool)> {
        ensure!(
            !at.journal.finished,
            "the run is finished. `xpd {} start` starts another.",
            at.store.command
        );
        let surveyed = measure_finish(&at.journal, at.place, found)?;
        let measurements = Measurements {
            finish: Some(convert(&surveyed)?),
            ..at.measurements()
        };
        let (journal, receipt) = at.send(measurements, "finish", RunCommand::Finish)?;
        if receipt.outcome == ReceiptOutcome::Applied {
            report_finish(&journal, &surveyed);
            remove_if_present(&migrated(&at.place.journal))?;
            return Ok((journal, true));
        }
        let unexplained = unexplained(&journal, &surveyed);
        ensure!(
            !unexplained.is_empty(),
            "{}",
            receipt.detail.unwrap_or_default()
        );
        report_unexplained(&unexplained);
        Ok((journal, false))
    }

    /// How many gates of the fix `id` failed.
    fn attempts(journal: &Journal, id: &str) -> usize {
        (journal.fixes())
            .find(|fix| fix.id == id)
            .map_or(0, |fix| fix.attempts.len())
    }
}

/// Runs one stage of a run of `K`'s command, and prints the run with the stage that comes
/// next. `survey` is the survey of that command, as [`super::Select::print`] takes it.
pub(super) fn act<K: Kind, R>(
    action: Action,
    format: Format,
    survey: impl Fn(&[PathBuf], Tree<'_>, Option<&Path>, &str) -> Result<(R, Vec<Entry<K>>)>,
) -> Result<ExitCode> {
    let command = K::COMMAND;
    let here = std::env::current_dir()?;
    let place = Place::find(command)?;
    let surveyed = |journal: &Journal, commit: Option<&str>| -> Result<Vec<Entry<K>>> {
        let top = Some(place.top.as_path());
        let entries = match commit {
            None => {
                let files = Tree {
                    top,
                    shown_from: None,
                };
                survey(&journal.paths, files, top, "HEAD")?.1
            }
            Some(commit) => at_commit(&journal.paths, &place.top, commit, |then, files, at| {
                Ok(survey(then, files, top, at)?.1)
            })?
            .unwrap_or_default(),
        };
        Ok(entries)
    };
    let found = |journal: &Journal, commit: Option<&str>| -> Result<BTreeSet<String>> {
        Ok((surveyed(journal, commit)?.into_iter())
            .map(|entry| entry.id)
            .collect())
    };
    #[cfg(not(feature = "ledger-durable"))]
    let (journal, kept) = act_journal(action, &place, &here, command, &found)?;
    #[cfg(feature = "ledger-durable")]
    let renamed = |journal: &Journal, commit: Option<&str>| -> Result<BTreeMap<String, String>> {
        Ok((surveyed(journal, commit)?.into_iter())
            .map(|entry| (entry.legacy_id, entry.id))
            .collect())
    };
    #[cfg(feature = "ledger-durable")]
    let (journal, kept) = durable_run::act(action, &place, &here, command, &found, &renamed)?;
    match format {
        Format::Json => println!(
            "{}",
            serde_json::json!({"run": journal, "next": journal.next(command)})
        ),
        Format::Text => print!("{}", journal.text(command)),
    }
    Ok(if kept {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
