//! `xpd hook`: what xpd answers when the client of a coding agent reports an event of a session.
//!
//! Claude Code and Codex send the same fields for `SessionStart` and `Stop`, and both read the
//! same answer on `Stop`, so those two events have one code path for both clients.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};
use ast_grep_config::Severity;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::{self, Filters};
use crate::cli::{self, FixMode, Scope};
use crate::embed;
use crate::graph::Graph;
use crate::rule::Applicability;
use crate::scan;

/// Claude Code shows stderr to the agent only when a hook exits 2.
const BLOCK: u8 = 2;
/// Any other non-zero hook exit shows stderr to the user without blocking the agent.
const FAILED: u8 = 1;
/// A session file that nothing wrote to for this long belongs to a session that is over.
const SESSION_KEPT: Duration = Duration::from_hours(7 * 24);

/// An event of an agent session, as the client names it in `hook_event_name`.
enum HookEvent {
    SessionStart {
        session: String,
        cwd: PathBuf,
        source: Source,
    },
    PostToolUse {
        client: Client,
        /// `None` when the event names no session, and then nothing is recorded.
        session: Option<String>,
        files: Vec<PathBuf>,
    },
    Stop {
        session: String,
        cwd: PathBuf,
        turn: Turn,
    },
}

/// What made the client send `SessionStart`.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// A session that starts, or one that the user resumes, clears or forks.
    Opened,
    /// A compaction, which happens inside a session.
    Compacted,
}

/// What the client says in `stop_hook_active` about the turn that wants to end.
#[derive(Clone, Copy, PartialEq)]
enum Turn {
    /// The agent stopped by itself.
    Ended,
    /// The client already continued this turn because a Stop hook held it.
    Continued,
}

/// The client that sent a `PostToolUse` event. The two name the files and read the issues in
/// different ways.
enum Client {
    /// A client of the Agent Plugins specification, which names the completed call's result
    /// `tool_result`. It reads the issues from stdout as `additional_context`.
    AgentPlugin {
        /// The agent's working directory. The hook itself starts in the plugin's folder.
        cwd: Option<PathBuf>,
    },
    /// A hook in the Claude Code settings. It reads the issues from stderr when the exit code
    /// is 2.
    ClaudeCode,
}

impl HookEvent {
    fn parse(event: &Value) -> Result<Self> {
        let text = |key: &str| event.get(key).and_then(Value::as_str);
        let name = text("hook_event_name").unwrap_or_default();
        let required = |key: &str| {
            text(key).with_context(|| format!("the {name} event has no {key}, which is a string"))
        };
        Ok(match name {
            "SessionStart" => Self::SessionStart {
                session: required("session_id")?.to_owned(),
                cwd: required("cwd")?.into(),
                source: match text("source") {
                    Some("compact") => Source::Compacted,
                    _ => Source::Opened,
                },
            },
            "Stop" => Self::Stop {
                session: required("session_id")?.to_owned(),
                cwd: required("cwd")?.into(),
                turn: match event.get("stop_hook_active").and_then(Value::as_bool) {
                    Some(true) => Turn::Continued,
                    _ => Turn::Ended,
                },
            },
            // Every other event is read as an edit, which is what the hook did before it read
            // the name. An event that names no file gets no answer.
            _ => {
                let (client, files) = edit(event);
                Self::PostToolUse {
                    client,
                    session: text("session_id").map(str::to_owned),
                    files,
                }
            }
        })
    }
}

/// The client that sent the `PostToolUse` event `event`, and the files that its tool call
/// changed.
fn edit(event: &Value) -> (Client, Vec<PathBuf>) {
    let path = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(PathBuf::from);
    if event.get("tool_result").is_none() {
        let input = event.get("tool_input");
        let named = input.and_then(|input| path(input, "file_path"));
        // Codex edits files with `apply_patch`, which names them inside the text of the patch.
        let diff = input
            .and_then(|input| input.get("command"))
            .and_then(Value::as_str)
            .filter(|_| event.get("tool_name").and_then(Value::as_str) == Some("apply_patch"))
            .unwrap_or_default();
        let cwd = path(event, "cwd").unwrap_or_default();
        let files = named.into_iter().chain(patched(diff, &cwd)).collect();
        return (Client::ClaudeCode, files);
    }
    // Only the file.modify profile names files. A call that reports an error may still have
    // changed some of them.
    let modified =
        event.pointer("/tool_profile/name").and_then(Value::as_str) == Some("file.modify");
    let operations = event
        .pointer("/tool_profile/input/operations")
        .and_then(Value::as_array)
        .filter(|_| modified);
    let files = operations
        .into_iter()
        .flatten()
        .filter_map(
            |operation| match operation.get("kind").and_then(Value::as_str) {
                Some("edit" | "write") => path(operation, "path"),
                Some("move") => path(operation, "to"),
                _ => None,
            },
        )
        .collect();
    let cwd = path(event, "cwd");
    (Client::AgentPlugin { cwd }, files)
}

/// The files that the `apply_patch` text `patch` adds, updates or moves to. Their paths are
/// relative to `cwd`, the folder Codex ran the patch in. A deleted file has nothing to lint.
fn patched<'a>(patch: &'a str, cwd: &'a Path) -> impl Iterator<Item = PathBuf> + 'a {
    const HEADERS: [&str; 3] = ["*** Add File: ", "*** Update File: ", "*** Move to: "];
    patch
        .lines()
        .filter_map(|line| HEADERS.iter().find_map(|header| line.strip_prefix(header)))
        .map(move |file| cwd.join(file.trim()))
}

/// Answers the hook event on stdin the way the client that sent it reads the answer.
pub(crate) fn run() -> ExitCode {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("xpd hook: cannot read stdin: {e}");
        return ExitCode::from(FAILED);
    }
    let event: Value = match serde_json::from_str(&input) {
        Ok(event) => event,
        Err(e) => {
            eprintln!("xpd hook: stdin is not hook JSON: {e}");
            return ExitCode::from(FAILED);
        }
    };
    let answered = HookEvent::parse(&event).and_then(|event| match event {
        HookEvent::SessionStart {
            session,
            cwd,
            source,
        } => session_start(&session, &cwd, source),
        HookEvent::PostToolUse {
            client,
            session,
            files,
        } => post_tool_use(&client, session.as_deref(), &files),
        HookEvent::Stop { session, cwd, turn } => stop(&session, &cwd, turn),
    });
    answered.unwrap_or_else(|e| {
        eprintln!("xpd hook: {e:#}");
        ExitCode::from(FAILED)
    })
}

/// Records where the session began, so that the Stop check also sees a file that the agent
/// changed and committed.
fn session_start(session: &str, cwd: &Path, source: Source) -> Result<ExitCode> {
    let session = Session::open(session)?;
    session.forget_old_sessions();
    let records = session.records()?;
    let began = (records.iter()).any(|record| matches!(record, Record::Start(_)));
    // A resumed session begins again, because the files that changed while it was closed are
    // not its own. A compaction happens inside a session, so the first record stays.
    if source == Source::Opened || !began {
        let commit = repository(cwd).and_then(|top| checked_out(&top));
        session.append(&Record::Start(commit))?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Lints the files that a tool call changed, and hands the issues to the client the way that
/// client reads them.
fn post_tool_use(client: &Client, session: Option<&str>, files: &[PathBuf]) -> Result<ExitCode> {
    // The project's venv and its own rules are found from the working directory, as in a
    // user's run.
    if let Client::AgentPlugin { cwd: Some(cwd) } = client {
        std::env::set_current_dir(cwd).context("cannot enter the agent's working directory")?;
    }
    // Only a project with its own rules can have rules for a file that is not Python.
    let project = std::env::current_dir().unwrap_or_default();
    let has_rules = matches!(catalog::LocalProject::find(&project), Ok(Some(_)));
    let files: Vec<PathBuf> = files
        .iter()
        .filter(|file| scan::is_python(file) || (has_rules && scan::names_language(file)))
        .cloned()
        .collect();
    if files.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    // The lint still runs when the record cannot be written, because the agent needs the
    // issues of this edit either way.
    if let Some(session) = session
        && let Err(e) = record_edits(session, &files)
    {
        eprintln!("xpd hook: cannot record the edit: {e:#}");
    }
    let lines: Vec<String> = (findings(&files)?.into_iter())
        .map(|finding| finding.line)
        .collect();
    if lines.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    Ok(match client {
        Client::AgentPlugin { .. } => {
            println!("{}", json!({ "additional_context": lines.join("\n") }));
            ExitCode::SUCCESS
        }
        Client::ClaudeCode => {
            eprintln!("{}", lines.join("\n"));
            ExitCode::from(BLOCK)
        }
    })
}

/// Writes the edited Python files to the session file. The Stop check reads them in a project
/// without git, where nothing else tells which files the session changed.
fn record_edits(session: &str, files: &[PathBuf]) -> Result<()> {
    let session = Session::open(session)?;
    for file in files.iter().filter(|file| scan::is_python(file)) {
        session.append(&Record::Edited(std::path::absolute(file)?))?;
    }
    Ok(())
}

/// Lints the Python files that the session changed, and holds the turn while one of them has
/// an error or a warning.
///
/// Claude Code and Codex both read `decision` and `reason` from JSON on stdout, and Codex takes
/// no plain text on this event. Both show `systemMessage` to the user and not to the agent.
fn stop(session: &str, cwd: &Path, turn: Turn) -> Result<ExitCode> {
    std::env::set_current_dir(cwd).context("cannot enter the agent's working directory")?;
    let session = Session::open(session)?;
    let records = session.records()?;
    let (held, advice): (Vec<Finding>, Vec<Finding>) = findings(&changed_files(&records)?)?
        .into_iter()
        .partition(|finding| finding.holds);
    let lines = |found: Vec<Finding>| -> Vec<String> {
        found.into_iter().map(|finding| finding.line).collect()
    };
    let (held, advice) = (lines(held), lines(advice));
    let last = (records.iter().rev())
        .find_map(|record| match record {
            Record::Held(lines) => Some(lines.as_slice()),
            Record::Start(_) | Record::Edited(_) => None,
        })
        .unwrap_or_default();
    // The agent saw this list when the hook held the turn, and it stopped again with the same
    // list. Holding the turn once more would repeat that until the client's own limit.
    let repeated = turn == Turn::Continued && !held.is_empty() && last == held;
    let holds = !held.is_empty() && !repeated;
    let record = if holds { held.as_slice() } else { &[] };
    if last != record {
        session.append(&Record::Held(record.to_vec()))?;
    }
    let mut text = Vec::new();
    if holds {
        text.push(
            "xpd holds this turn, because the Python files that this session changed have \
             errors or warnings. Fix each line below. When the code is right as it is, write \
             `# ast-grep-ignore: <pack>/<rule>` on the line above it, with the reason after the \
             rule."
                .to_owned(),
        );
        text.extend(held);
        if !advice.is_empty() {
            text.push("The lines below are advice, and they do not hold the turn.".to_owned());
        }
    } else if repeated {
        text.push(
            "xpd let the turn end with the errors and warnings that it already held the turn on."
                .to_owned(),
        );
        text.extend(held);
    }
    if !holds && !advice.is_empty() {
        text.push("xpd has advice on the Python files that this session changed.".to_owned());
    }
    text.extend(advice);
    let text = text.join("\n");
    if holds {
        println!("{}", json!({ "decision": "block", "reason": text }));
    } else if !text.is_empty() {
        println!("{}", json!({ "systemMessage": text }));
    }
    Ok(ExitCode::SUCCESS)
}

/// The Python files that the session changed. With git, they are the files that differ from
/// the commit the session began on and the untracked files, so an edit through the shell and
/// a committed edit both count. Without git, they are the files that `PostToolUse` recorded.
fn changed_files(records: &[Record]) -> Result<Vec<PathBuf>> {
    let here = fs::canonicalize(".")?;
    let mut files: Vec<PathBuf> = if let Some(top) = repository(&here) {
        let began = records.iter().rev().find_map(|record| match record {
            Record::Start(commit) => commit.as_deref(),
            Record::Edited(_) | Record::Held(_) => None,
        });
        // The recorded commit is not in this repository when the agent moved to another one,
        // and no commit is recorded when the hook was added to a running session. Then the
        // files that differ from the commit checked out now are the closest answer.
        let base = began
            .filter(|commit| {
                git(&top, &["cat-file", "-e", &format!("{commit}^{{tree}}")]).is_some()
            })
            .map(str::to_owned)
            .or_else(|| checked_out(&top))
            .context("git names no commit to compare the files with")?;
        let differing = git(
            &top,
            &["diff", "--name-only", "-z", "--diff-filter=d", &base, "--"],
        )
        .context("git diff failed")?;
        let untracked = git(&top, &["ls-files", "--others", "--exclude-standard", "-z"])
            .context("git ls-files failed")?;
        (differing.split('\0').chain(untracked.split('\0')))
            .filter(|name| !name.is_empty())
            .map(|name| top.join(name))
            .collect()
    } else {
        (records.iter())
            .filter_map(|record| match record {
                Record::Edited(file) => Some(file.clone()),
                Record::Start(_) | Record::Held(_) => None,
            })
            .collect()
    };
    files.retain(|file| scan::is_python(file));
    files.sort();
    files.dedup();
    // A file under the working directory is named from there, as the agent names it.
    Ok(files
        .into_iter()
        .map(|file| {
            file.strip_prefix(&here)
                .map(Path::to_path_buf)
                .unwrap_or(file)
        })
        .collect())
}

/// The top folder of the git repository that holds `folder`. `None` outside a repository, and
/// when git is not installed.
fn repository(folder: &Path) -> Option<PathBuf> {
    let top = git(folder, &["rev-parse", "--show-toplevel"])?;
    Some(PathBuf::from(top.trim_end_matches(['\r', '\n'])))
}

/// The commit that is checked out in the repository at `top`. A repository without a commit
/// gives the empty tree, which every file differs from.
fn checked_out(top: &Path) -> Option<String> {
    git(top, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])
        // With no input, git names the tree that holds no file.
        .or_else(|| git(top, &["hash-object", "-t", "tree", "--stdin"]))
        .map(|name| name.trim().to_owned())
}

/// What `git` prints for `args` in `folder`. `None` when git cannot start or the command
/// fails.
fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(folder)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    (output.status.success()).then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// One line of a session file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Record {
    /// The commit that was checked out when the session began. `None` without git.
    Start(Option<String>),
    /// A Python file that a tool call changed.
    Edited(PathBuf),
    /// The errors and warnings that the last Stop check held the turn on. Empty when it let
    /// the turn end.
    Held(Vec<String>),
}

/// The file that holds what xpd knows about one agent session, in the `sessions` folder of
/// xpd's cache folder. Each event adds lines and none rewrites the file, so two hooks of one
/// session that run at the same time do not lose each other's lines.
struct Session {
    file: PathBuf,
}

impl Session {
    /// The file of the session that the client names `id`. The id becomes the file name, so
    /// an id that could name another folder is refused.
    fn open(id: &str) -> Result<Self> {
        let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
        anyhow::ensure!(
            !id.is_empty() && !id.starts_with('.') && id.chars().all(plain),
            "the session id {id:?} cannot name a session file"
        );
        Ok(Self {
            file: embed::cache_folder()?.join("sessions").join(id),
        })
    }

    /// The lines of the file, oldest first. A session that wrote nothing yet has none. A line
    /// that this version cannot read is left out, because a later version may have written it.
    fn records(&self) -> Result<Vec<Record>> {
        let text = match fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).context(scan::cannot_read(&self.file)),
        };
        Ok(text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }

    fn append(&self, record: &Record) -> Result<()> {
        let cannot_write = || format!("cannot write {}", self.file.display());
        if let Some(folder) = self.file.parent() {
            fs::create_dir_all(folder).with_context(cannot_write)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .with_context(cannot_write)?;
        // One write for the whole line keeps the lines of two hooks apart.
        let line = format!("{}\n", serde_json::to_string(record)?);
        file.write_all(line.as_bytes()).with_context(cannot_write)
    }

    /// Deletes the session files beside this one that nothing wrote to for seven days. A file
    /// that cannot be read or deleted stays for a later session to delete.
    fn forget_old_sessions(&self) {
        let entries = self
            .file
            .parent()
            .and_then(|folder| fs::read_dir(folder).ok());
        for entry in entries.into_iter().flatten().flatten() {
            let age = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok());
            if age.is_some_and(|age| age > SESSION_KEPT) {
                // Another session that starts now may have deleted the file already.
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

/// One line of the hook's answer.
struct Finding {
    line: String,
    /// Whether it is an error or a warning, which the Stop check holds the turn on.
    holds: bool,
}

/// The issues of the files in `files`. A Python file with a pipeline of steps also gets the
/// errors and warnings of `xpd graph`.
fn findings(files: &[PathBuf]) -> Result<Vec<Finding>> {
    // One call can edit a file and then move or delete it, so a named file may be gone.
    let files: Vec<PathBuf> = (files.iter())
        .filter(|file| file.is_file())
        .cloned()
        .collect();
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let report = cli::lint(
        &files,
        FixMode::Apply(None),
        Scope::Files,
        &Filters::default(),
        None,
    )?;
    let issues = report.issues.iter().map(|issue| Finding {
        line: issue.line(Applicability::Safe),
        holds: catalog::rank(&issue.severity) >= catalog::rank(&Severity::Warning),
    });
    let python = files.iter().filter(|file| scan::is_python(file));
    let steps = python.flat_map(|file| {
        fs::read_to_string(file)
            .map(|src| Graph::step_lines(file, &src))
            .unwrap_or_default()
    });
    // `step_lines` gives errors and warnings only.
    let steps = steps.map(|line| Finding { line, holds: true });
    Ok(issues.chain(steps).collect())
}
