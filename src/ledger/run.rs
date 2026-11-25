//! A refactor that a command of the ledger measures, as a run with stages that xpd records.
//!
//! The run is a journal in the git folder of the repository, so it survives the end of a
//! session and belongs to no agent tool. The agent judges, designs and edits. xpd keeps what
//! was decided, says which stage comes next, and gates each fix itself: it runs the checks of
//! the project and compares the findings by their ids, so no stage grades its own work.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::git;
use crate::output::Format;

use super::{Entry, Kind, Tree, at_commit, repository};

/// How many times a fix may fail its gate before the next stage is to drop it.
const ATTEMPTS: usize = 2;
/// How much of the output of a failed check the journal keeps.
const OUTPUT_KEPT: usize = 1500;

/// A stage of a run.
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Action {
    /// Start a run: record the commit it starts from, the paths and the checks of the project.
    Start {
        /// Files or directories to measure, inside one git repository.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// A command that exits 0 when the project is sound, such as its tests. Repeat the flag
        /// for more than one. Each runs now, and again at every gate.
        #[arg(long = "check", value_name = "COMMAND")]
        checks: Vec<String>,
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
    Plan {
        /// The JSON file of the plan, or `-` for stdin.
        file: PathBuf,
    },
    /// Record that the user said yes to a fix that changes what callers or users see.
    Approve { fix: String },
    /// Gate a fix that is committed: run the checks and compare the findings with the commit
    /// before it. Exit 1 when the fix is not kept.
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
    /// End the run: compare with the commit it started from. Exit 1 while a fix is open or a
    /// new finding has no reason.
    Finish,
}

/// One check of the project.
#[derive(Debug, Serialize, Deserialize)]
struct Check {
    command: String,
    /// Whether it passed when the run started. A check that failed then gates nothing.
    passed: bool,
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
    /// The ids of the findings that the fix is to remove.
    findings: Vec<String>,
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
#[derive(Debug, Deserialize)]
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
    /// How many findings the base had.
    was: usize,
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
            text: format!(
                "Make the fix {id}, \"{}\", which removes the findings {}. Keep the behavior \
                 the same, commit it, and run `xpd {command} gate {id}`. The gate keeps the \
                 fix when each check passes as it did at the start, a finding of the fix is \
                 gone, and the count of findings is not higher than at {}.",
                fix.title,
                fix.findings.join(", "),
                &self.current[..self.current.len().min(12)]
            ),
        }
    }

    fn text(&self, command: &str) -> String {
        let mut text = String::new();
        let _ = writeln!(
            text,
            "xpd {command} run: started at {} with {} findings, now at {}",
            &self.base[..self.base.len().min(12)],
            self.was,
            &self.current[..self.current.len().min(12)]
        );
        for check in &self.checks {
            let state = if check.passed {
                "passed"
            } else {
                "failed at the start, so it gates nothing"
            };
            let _ = writeln!(text, "  check: {} ({state})", check.command);
        }
        for (round, fixes) in self.rounds.iter().enumerate() {
            let _ = writeln!(text, "  round {}", round + 1);
            for fix in fixes {
                let state = match &fix.outcome {
                    Some(Outcome::Kept { fixed, added, .. }) => {
                        format!("kept, {} fixed, {} added", fixed.len(), added.len())
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

/// What a gate found, with no file or command in it.
#[derive(Debug)]
struct Compared {
    fixed: Vec<String>,
    added: Vec<String>,
}

/// Whether a fix is kept. `before` and `after` hold the ids of the findings at the commit
/// before the fix and after it, and `broken` the checks that passed at the start and fail now.
fn judge(
    fix: &Fix,
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    broken: &[String],
) -> Result<Compared, String> {
    if !broken.is_empty() {
        return Err(format!(
            "These checks passed at the start and fail now: {}.",
            broken.join("; ")
        ));
    }
    if !fix.findings.iter().any(|id| !after.contains(id)) {
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
        let name = format!("xpd-{command}.json");
        let journal = git::output(&top, &["rev-parse", "--git-path", &name])?;
        Ok(Self {
            journal: top.join(journal.trim()),
            top,
        })
    }

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

/// Whether `command` exits 0 in the working folder, with the end of what it printed.
fn passes(command: &str) -> (bool, String) {
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

/// A new journal for the paths, which are given from the folder `here`.
fn start(
    place: &Place,
    here: &Path,
    paths: &[PathBuf],
    checks: Vec<String>,
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
        was: 0,
        rounds: Vec::new(),
        accepted: BTreeMap::new(),
        finished: false,
    };
    journal.was = found(&journal, None)?.len();
    Ok(journal)
}

/// Adds the plan in `text` to the journal, after checking it against the ledger of today.
fn plan(journal: &mut Journal, text: &str, command: &str, found: Found<'_>) -> Result<()> {
    ensure!(!journal.finished, "the run is finished");
    if let Some(open) = journal.open() {
        bail!(
            "the fix {} is still open. Gate it or drop it before the next plan",
            open.id
        );
    }
    let plan: Plan = serde_json::from_str(text).with_context(|| {
        format!("the plan is not the JSON that `xpd {command} plan --help` shows")
    })?;
    let known = found(journal, None)?;
    let mut named: BTreeSet<&str> = journal.fixes().map(|fix| fix.id.as_str()).collect();
    for fix in &plan.fixes {
        ensure!(named.insert(&fix.id), "two fixes are named {}", fix.id);
        ensure!(
            !fix.findings.is_empty(),
            "the fix {} names no finding",
            fix.id
        );
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

/// Gates the fix `id` at the commit that is checked out. Returns whether the fix is kept.
fn gate(journal: &mut Journal, place: &Place, id: &str, found: Found<'_>) -> Result<bool> {
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
    let fix = journal.fix_mut(id)?;
    match judge(fix, &before, &after, &broken) {
        Ok(Compared { fixed, added }) => {
            fix.outcome = Some(Outcome::Kept {
                commit: commit.clone(),
                fixed,
                added,
            });
            journal.current = commit;
            Ok(true)
        }
        Err(reason) => {
            eprintln!("xpd: the gate did not keep {id}. {reason}");
            fix.attempts.push(reason);
            Ok(false)
        }
    }
}

/// Ends the run when no fix is open and each finding that is new since the start has a
/// reason. Returns whether the run ended.
fn finish(journal: &mut Journal, place: &Place, found: Found<'_>) -> Result<bool> {
    if let Some(open) = journal.open() {
        bail!("the fix {} is still open", open.id);
    }
    place.clean()?;
    let before = found(journal, Some(&journal.base))?;
    let after = found(journal, None)?;
    let explained = |id: &&String| journal.accepted.contains_key(*id);
    let unexplained: Vec<&str> = (after.difference(&before))
        .filter(|id| !explained(id))
        .map(String::as_str)
        .collect();
    if !unexplained.is_empty() {
        eprintln!(
            "xpd: these findings are new since the start and have no reason: {}. Remove them, or accept them in a plan",
            unexplained.join(", ")
        );
        return Ok(false);
    }
    eprintln!(
        "xpd: {} findings at the start, {} now: {} fixed, {} added, {} left without a reason",
        before.len(),
        after.len(),
        before.difference(&after).count(),
        after.difference(&before).count(),
        after.iter().filter(|id| !explained(id)).count()
    );
    journal.finished = true;
    Ok(true)
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
    let found = |journal: &Journal, commit: Option<&str>| -> Result<BTreeSet<String>> {
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
        Ok(entries.into_iter().map(|entry| entry.id).collect())
    };
    let mut kept = true;
    let journal = match action {
        Action::Start {
            paths,
            checks,
            restart,
        } => {
            if let Ok(old) = place.read(command) {
                ensure!(
                    old.finished || restart,
                    "a run is open. Go on with `xpd {command} status`, or replace it with --restart"
                );
            }
            start(&place, &here, &paths, checks, &found)?
        }
        Action::Plan { file } => {
            let mut journal = place.read(command)?;
            let text = if file == Path::new("-") {
                let mut text = String::new();
                std::io::stdin().read_to_string(&mut text)?;
                text
            } else {
                fs::read_to_string(here.join(&file))
                    .with_context(|| format!("cannot read {}", file.display()))?
            };
            plan(&mut journal, &text, command, &found)?;
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
            kept = gate(&mut journal, &place, &fix, &found)?;
            journal
        }
        Action::Status => place.read(command)?,
        Action::Finish => {
            let mut journal = place.read(command)?;
            kept = finish(&mut journal, &place, &found)?;
            journal
        }
    };
    place.write(&journal)?;
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
