//! `xpd hook`: what xpd answers when the client of a coding agent reports an event of a session.
//!
//! Claude Code and Codex send the same fields for `SessionStart`, `UserPromptSubmit` and `Stop`,
//! and both read the same answers, so those three events have one code path for both clients.
//! Both add what the hook prints on stdout for `SessionStart` and `UserPromptSubmit` to what the
//! agent reads, when the hook exits 0. That text states what xpd found and gives no order,
//! because Claude Code may show text that reads as a system command to the user instead.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::severity::Severity;
use anyhow::{Context, Result};
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::Filters;
use crate::embed;
use crate::fix_safety::Applicability;
use crate::git;
use crate::graph::Graph;
use crate::index;
use crate::report::Issue;
use crate::settings::Settings;
use crate::source;
use crate::style::Style;
use crate::syntax;
use crate::venv::Venv;
use crate::workspace::Workspace;

/// Claude Code shows stderr to the agent only when a hook exits 2.
const BLOCK: u8 = 2;
/// Any other non-zero hook exit shows stderr to the user without blocking the agent.
const FAILED: u8 = 1;
/// A session file that nothing wrote to for this long belongs to a session that is over.
const SESSION_KEPT: Duration = Duration::from_hours(7 * 24);
/// The most Python files of a project that the session brief and the prompt answer read, so
/// that both stay fast in a large project.
const MOST_FILES: usize = 1000;
/// The most files and folders of a project that are looked at to find its Python files.
const MOST_ENTRIES: usize = 20_000;
/// The most characters of a session brief or a prompt answer. Claude Code passes on 10,000
/// characters of a hook's output and Codex about 2,500 tokens.
const MOST_CHARACTERS: usize = 2000;
/// The most files that one line of a brief names.
const MOST_NAMED: usize = 3;
/// The rules with the most issues that a prompt answer names.
const TOP_RULES: usize = 5;
/// The most lines of drift from the profile that was there before the turn, which the Stop
/// check names in its notice.
const MOST_NOTICES: usize = 5;
/// A prompt with one of these phrases asks what to make or what to improve.
const OPEN_ASKS: [&str; 12] = [
    "what should i",
    "what should we",
    "what can i",
    "what could i",
    "what to make",
    "what to build",
    "what to improve",
    "what to do next",
    "what next",
    "improve",
    "optimi",
    "better",
];

/// An event of an agent session, as the client names it in `hook_event_name`.
enum HookEvent {
    SessionStart {
        session: String,
        cwd: PathBuf,
        source: Source,
    },
    UserPromptSubmit {
        cwd: PathBuf,
        prompt: String,
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
            "UserPromptSubmit" => Self::UserPromptSubmit {
                cwd: required("cwd")?.into(),
                prompt: required("prompt")?.to_owned(),
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
        HookEvent::UserPromptSubmit { cwd, prompt } => user_prompt(&cwd, &prompt),
        HookEvent::PostToolUse {
            client,
            session,
            files,
        } => post_tool_use(&client, session.as_deref(), &files),
        HookEvent::Stop { session, cwd, turn } => stop(&session, &cwd, turn),
    });
    answered.unwrap_or_else(|e| {
        eprintln!("xpd hook: {}", crate::output::redact(&format!("{e:#}")));
        ExitCode::from(FAILED)
    })
}

/// Records where the session began, so that the Stop check also sees a file that the agent
/// changed and committed. In a project with a Python file it then prints the session brief,
/// also after a compaction, which drops the brief the session began with.
fn session_start(session: &str, cwd: &Path, source: Source) -> Result<ExitCode> {
    let session = Session::open(Path::new(""), session)?;
    session.forget_old_sessions();
    let records = session.records()?;
    let began =
        (records.iter()).any(|record| matches!(record, Record::Start(_) | Record::Opened { .. }));
    // A resumed session begins again, because the files that changed while it was closed are
    // not its own. A compaction happens inside a session, so the first record stays.
    let root = project_root(cwd)?;
    if source == Source::Opened || !began {
        let at = now();
        let git = repository(cwd)
            .map(|top| GitBaseline::capture(top, at))
            .transpose()?;
        session.append(&Record::Opened { at, git })?;
        if let Some(baseline) = profile_drift(&root) {
            session.append(&Record::Profile {
                root: root.clone(),
                baseline,
            })?;
        }
    }
    let cwd = root;
    let files = project_files(&cwd, MOST_FILES);
    if !files.is_empty() {
        print_context(&brief(&cwd, &files)?);
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints `text` for the agent, cut to the most characters that an answer has.
fn print_context(text: &str) {
    let cut: String = text.chars().take(MOST_CHARACTERS).collect();
    println!("{cut}");
}

/// The Python files of the project in the working directory, in the order of their names and
/// at most `most` of them. Folders respect `.gitignore`, as in `xpd check`. Each file is named
/// from the working directory, as the agent names it.
fn project_files(root: &Path, most: usize) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(root)
        .sort_by_file_name(Ord::cmp)
        .build()
        .take(MOST_ENTRIES)
        .flatten()
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(ignore::DirEntry::into_path)
        .filter(|path| source::is_python(path))
        .map(|path| {
            path.strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or(path)
        })
        .take(most)
        .collect()
}

/// `count` with `noun`, which gets an s when the count is not one.
fn counted(count: usize, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{plural}")
}

/// How many files `files` are of the project. The scan stops at [`MOST_FILES`], and the text
/// says so when it did.
fn files_read(files: &[PathBuf]) -> String {
    if files.len() == MOST_FILES {
        format!("the first {MOST_FILES} Python files")
    } else {
        counted(files.len(), "Python file")
    }
}

/// What one Python file adds to the session brief.
struct Census {
    /// The modules of the packs whose rules run on the file.
    packs: Vec<String>,
    pipelines: usize,
    /// The nodes of a file that imports Hamilton. `xpd graph` reads every other module as
    /// nodes too, and those functions are no nodes of the project.
    nodes: usize,
}

/// What xpd knows about the project before the agent writes anything: the packs whose rules
/// run on `files`, the style setting, the pipelines and the Hamilton nodes that `xpd graph`
/// reads, and the three commands that fit between the edits.
fn brief(project: &Path, files: &[PathBuf]) -> Result<String> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let settings = Settings::find_in(project, project)?;
    // The packs apply by the imports of a file, whatever versions the project's venv holds.
    let workspace = Workspace::load(
        project.to_path_buf(),
        None,
        settings,
        None,
        &Filters::default(),
    )?;
    let read: Vec<(&PathBuf, Census)> = files
        .par_iter()
        .filter_map(|file| {
            let src = fs::read_to_string(project.join(file)).ok()?;
            let root = syntax::parse(&src);
            let packs: Vec<String> = (workspace.catalog.packs.iter())
                .filter(|pack| pack.applies(&root, &src, SupportLang::Python))
                .map(|pack| pack.module.clone())
                .collect();
            let (pipelines, nodes) = Graph::read(project, std::slice::from_ref(file)).counts();
            let nodes = if packs.iter().any(|pack| pack == "hamilton") {
                nodes
            } else {
                0
            };
            let census = Census {
                packs,
                pipelines,
                nodes,
            };
            Some((file, census))
        })
        .collect();
    let mut packs = BTreeMap::<&str, usize>::new();
    for pack in read.iter().flat_map(|(_, census)| &census.packs) {
        *packs.entry(pack).or_default() += 1;
    }
    let mut packs: Vec<_> = packs.into_iter().collect();
    packs.sort_by_key(|(name, count)| (std::cmp::Reverse(*count), *name));
    let packs: Vec<String> = (packs.iter())
        .map(|(name, count)| format!("{name} ({})", counted(*count, "file")))
        .collect();
    let style_line = match settings.style {
        Style::Strict => {
            "The style setting is strict, so a rule of the house style is a warning, and the \
             Stop check holds a turn that leaves a warning."
        }
        Style::Default => {
            "The style setting is default, so a rule of the house style is info, which holds no \
             turn."
        }
    };
    let pipelines = placed(&read, |census| census.pipelines);
    let nodes = placed(&read, |census| census.nodes);
    Ok(format!(
        "xpd read {} of this project.\n\
         Rule packs that apply: {}.\n\
         {style_line}\n\
         Pipelines of steps that `xpd graph` reads: {pipelines}.\n\
         Hamilton nodes that `xpd graph` reads: {nodes}.\n\
         Three xpd commands fit between the edits of a session.\n\
         - `xpd find \"<task>\"` ranks library symbols, rules and playbooks for a task, and it \
         comes before the choice of a library or a method.\n\
         - `xpd graph <file>` prints the dataflow of a file, and it comes after a step or a node \
         is added, moved or renamed.\n\
         - `xpd equiv <script.py> --since HEAD` compares the result of a script with the result \
         of its committed version, and a refactor of data logic is done when it prints \
         {{\"equivalent\": true}}. `xpd equiv old.py new.py` compares two scripts.",
        files_read(files),
        packs.join(", "),
    ))
}

/// The total of `count` over the files, with the files that have the most, as in
/// `3 in 2 files: a.py (2), b.py (1)`.
fn placed(read: &[(&PathBuf, Census)], count: impl Fn(&Census) -> usize) -> String {
    let mut found: Vec<(&Path, usize)> = (read.iter())
        .map(|(file, census)| (file.as_path(), count(census)))
        .filter(|(_, count)| *count > 0)
        .collect();
    if found.is_empty() {
        return "none".to_owned();
    }
    found.sort_by_key(|(file, count)| (std::cmp::Reverse(*count), *file));
    let total: usize = found.iter().map(|(_, count)| count).sum();
    let mut named: Vec<String> = (found.iter().take(MOST_NAMED))
        .map(|(file, count)| format!("{} ({count})", file.display()))
        .collect();
    if found.len() > MOST_NAMED {
        named.push(format!("and {} more", found.len() - MOST_NAMED));
    }
    format!(
        "{total} in {}: {}",
        counted(found.len(), "file"),
        named.join(", ")
    )
}

/// Offers the offline animation guide, the chart review, the profile of a data file and the
/// ledger of a codebase refactor even before a project has Python files. In Python projects, also routes a refactor, other
/// playbooks and requests for open issues. A chart prompt gets no playbook, because the chart
/// line names its own.
fn user_prompt(cwd: &Path, prompt: &str) -> Result<ExitCode> {
    if animation_prompt(prompt) {
        print_context(
            "For this animation, start with the offline guide: run \
             `xpd animate guide` or call the xpd MCP tool `animate_guide`. \
             Use the conversation and relevant project code to identify what the animation \
             should show. Render a preview and check its intermediate frames before the final.",
        );
        return Ok(ExitCode::SUCCESS);
    }
    let cwd = project_root(cwd)?;
    let mut text = Vec::new();
    let chart = chart_prompt(prompt);
    if chart {
        text.push(
            "Chart or table task: run `xpd chart-review <paths>` on the files \
             that draw charts or presentation tables in Python or React/TypeScript (Plotly, Dash, \
             AG Grid, Matplotlib, Seaborn, pandas). It lists mechanical Tufte findings and a \
             per-figure checklist across all 57 principles from Tufte's book. Add `--adapter \
             <adapter.py or command>` to measure rendered figures, and `xpd chart-review start` \
             to record a gated review run. The `xpd-chart-review` skill holds the steps, and \
             `xpd api xpd/recipe/plotly-chart` prints the plotly-chart playbook."
                .to_owned(),
        );
    }
    if data_prep_prompt(prompt) {
        text.push(
            "Data preparation task: run `xpd data-prep <paths>` with `--adapter` to measure columns. Follow the `xpd-data-prep` skill to record and gate fixes.".to_owned(),
        );
    }
    text.extend(data_files(&cwd, prompt).iter().map(|file| {
        format!(
            "Data file task: `xpd profile {file}` prints the type, the \
             missing values and the number of distinct values of each column of {file}, with \
             findings on its shape. It comes before the choice of a method and before code that \
             reads the file."
        )
    }));
    if untangle_prompt(prompt) {
        text.push(
            "Codebase refactor task: `xpd untangle` reads the files and \
             the git history and lists the places where the code gets wrong which pieces change \
             together: copies, files that change in the same commits, import cycles, unused \
             definitions and special cases, each with its evidence. The count of each kind \
             measures the refactor, and `xpd untangle --since <commit>` prints the count at \
             that commit beside the count of today. `xpd untangle start` begins a run that xpd \
             records, with a plan, a gate for each fix and `xpd untangle status` for the stage \
             that comes next. The `xpd-untangle` skill holds the steps."
                .to_owned(),
        );
    }
    if harden_prompt(prompt) {
        text.push(
            "Reliability task: run `xpd harden start` to record checks and find silent failures, fixed waits and unreliable tests. The `xpd-harden` skill holds the steps.".to_owned(),
        );
    } else if optimize_prompt(prompt) {
        text.push(
            "Performance task: run `xpd optimize` to find repeated work and oversized intermediates. Use `xpd optimize start <paths> --check \"<tests>\" --bench \"<workload>\"` for a recorded run. The `xpd-optimize` skill holds the steps."
                .to_owned(),
        );
    }
    if !chart && !project_files(&cwd, 1).is_empty() {
        if refactor_prompt(prompt) {
            text.push(
                "Refactor task: for a script of data logic that assigns `result`, edit the \
                 script in place, and `xpd equiv <script.py> --since HEAD` runs the committed \
                 script and the edited one and compares the two results. The refactor is done \
                 when it prints {\"equivalent\": true}. For other data logic, `xpd equiv old.py \
                 new.py` compares two scripts that each assign `result`."
                    .to_owned(),
            );
        }
        let library = index::load(None)?.without_download(&cwd);
        if let Some(playbook) = index::playbook_for(&library, prompt) {
            text.push(playbook_lines(&playbook));
        }
        let lowered = prompt.to_lowercase();
        if OPEN_ASKS.iter().any(|ask| lowered.contains(ask)) {
            text.push(open_issues(&cwd, &project_files(&cwd, MOST_FILES))?);
        }
    }
    if !text.is_empty() {
        print_context(&text.join("\n"));
    }
    Ok(ExitCode::SUCCESS)
}

/// The data files that `prompt` names and the project holds, in the order of the prompt and at
/// most `MOST_NAMED` of them. They are the formats of `xpd profile` without JSON, because a
/// JSON file in a prompt is most often a settings file.
fn data_files(root: &Path, prompt: &str) -> Vec<String> {
    const FORMATS: [&str; 4] = ["csv", "parquet", "feather", "xlsx"];
    let mut files: Vec<String> = Vec::new();
    let named = prompt
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && !"/~._-".contains(c)))
        .map(|word| word.trim_end_matches('.'));
    for word in named {
        let format = Path::new(word)
            .extension()
            .and_then(|format| format.to_str());
        let is_data = format.is_some_and(|format| {
            FORMATS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(format))
        });
        if is_data && !files.iter().any(|file| file == word) && root.join(word).is_file() {
            files.push(word.to_owned());
        }
    }
    files.truncate(MOST_NAMED);
    files
}

/// Whether `prompt` asks to change code and keep its result, in English or Danish. Port,
/// convert and migrate route only beside the name of a frame library, so "open the port" stays out.
fn refactor_prompt(prompt: &str) -> bool {
    const STRONG: [&str; 20] = [
        "refactor",
        "refactors",
        "refactored",
        "refactoring",
        "rewrite",
        "rewrites",
        "rewriting",
        "rewritten",
        "vectorize",
        "vectorise",
        "vectorized",
        "vectorised",
        "optimize",
        "optimise",
        "simplify",
        "refaktorer",
        "refaktorere",
        "refaktorering",
        "omskriv",
        "omskrive",
    ];
    const WEAK: [&str; 7] = [
        "port",
        "convert",
        "translate",
        "migrate",
        "move",
        "konverter",
        "flyt",
    ];
    const LIBRARIES: [&str; 5] = ["pandas", "polars", "numpy", "duckdb", "pyarrow"];
    let lowered = prompt.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let has = |list: &[&str]| words.iter().any(|word| list.contains(word));
    has(&STRONG) || (has(&WEAK) && has(&LIBRARIES)) || lowered.contains("speed up")
}

/// Whether `prompt` asks to change the structure of a codebase, in English or Danish. A word
/// about structure routes alone. A word for a refactor routes only beside a word for the whole
/// codebase, so "refactor this function" stays with the route for data logic.
fn untangle_prompt(prompt: &str) -> bool {
    const STRUCTURE: [&str; 14] = [
        "untangle",
        "untangling",
        "restructure",
        "restructuring",
        "decouple",
        "decoupling",
        "architecture",
        "spaghetti",
        "duplication",
        "omstrukturer",
        "omstrukturere",
        "omstrukturering",
        "arkitektur",
        "arkitekturen",
    ];
    const REFACTOR: [&str; 9] = [
        "refactor",
        "refactoring",
        "clean",
        "cleanup",
        "simplify",
        "reorganize",
        "reorganise",
        "refaktorer",
        "oprydning",
    ];
    const WHOLE: [&str; 8] = [
        "codebase",
        "repo",
        "repository",
        "project",
        "modules",
        "kodebase",
        "kodebasen",
        "projektet",
    ];
    let lowered = prompt.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let has = |list: &[&str]| words.iter().any(|word| list.contains(word));
    has(&STRUCTURE)
        || (has(&REFACTOR) && has(&WHOLE))
        || lowered.contains("dead code")
        || lowered.contains("technical debt")
        || lowered.contains("tech debt")
}

fn harden_prompt(prompt: &str) -> bool {
    let prompt = prompt.to_lowercase();
    let words: HashSet<_> = prompt
        .split(|character: char| !character.is_alphanumeric())
        .collect();
    let has = |choices: &[&str]| choices.iter().any(|choice| words.contains(choice));
    has(&[
        "flaky",
        "swallowed",
        "swallow",
        "swallows",
        "deterministic",
        "determinism",
        "retry",
        "retries",
        "rerun",
        "nondeterministic",
        "tilfældig",
        "tilfældigt",
        "deterministisk",
    ]) || prompt.contains("fejler nogle gange")
        || prompt.contains("bare except")
        || (has(&[
            "sleep", "sleeps", "random", "seed", "error", "errors", "silent", "silently", "skipped",
        ]) && (has(&["test", "tests", "except", "failure"]) || prompt.contains("run again")))
}

/// Whether `prompt` asks to make code simpler, shorter or faster, in English or Danish.
fn optimize_prompt(prompt: &str) -> bool {
    const WORDS: [&str; 22] = [
        "optimize",
        "optimise",
        "optimizing",
        "optimising",
        "optimization",
        "optimisation",
        "simplify",
        "simplifying",
        "simpler",
        "shorter",
        "faster",
        "speedup",
        "performance",
        "optimer",
        "optimere",
        "optimering",
        "forenkl",
        "forenkle",
        "forenkling",
        "simplere",
        "kortere",
        "hurtigere",
    ];
    let lowered = prompt.to_lowercase();
    let mut words = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty());
    words.any(|word| WORDS.contains(&word))
        || lowered.contains("speed up")
        || lowered.contains("less code")
}

/// Whether `prompt` asks to make, change or review a chart or presentation table. A strong word,
/// such as chart, figure, supertable, aggrid or the Danish graf, routes alone. A weak word, such
/// as plot, table or a library name, routes only beside a second one, so "plot twist" or "sql
/// table" stays out. Graph after a word such as dataflow, and figure in "figure out", have another
/// sense.
fn chart_prompt(prompt: &str) -> bool {
    const STRONG: [&str; 33] = [
        "chart",
        "charts",
        "chartjunk",
        "graph",
        "graphs",
        "figure",
        "figures",
        "visualization",
        "visualizations",
        "visualisation",
        "visualisations",
        "visualize",
        "visualise",
        "heatmap",
        "heatmaps",
        "pie",
        "pies",
        "histogram",
        "histograms",
        "scatterplot",
        "scatterplots",
        "tufte",
        "supertable",
        "supertables",
        "aggrid",
        "datatable",
        "graf",
        "grafer",
        "grafen",
        "graferne",
        "figur",
        "figuren",
        "figurer",
    ];
    const WEAK: [&str; 22] = [
        "plot",
        "plots",
        "plotting",
        "plotly",
        "matplotlib",
        "pyplot",
        "seaborn",
        "bokeh",
        "altair",
        "ggplot",
        "ggplot2",
        "echarts",
        "recharts",
        "dash",
        "axis",
        "axes",
        "legend",
        "table",
        "tables",
        "grid",
        "columndefs",
        "dashboard",
    ];
    const OTHER_GRAPHS: [&str; 11] = [
        "dataflow",
        "call",
        "dependency",
        "knowledge",
        "task",
        "commit",
        "hamilton",
        "xpd",
        "step",
        "computation",
        "import",
    ];
    let lowered = prompt.to_lowercase();
    if lowered.contains("ag-grid") || lowered.contains("ag grid") || lowered.contains("go.table") {
        return true;
    }
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let strong = words.iter().enumerate().any(|(at, word)| {
        let before = at.checked_sub(1).and_then(|at| words.get(at));
        let after = words.get(at + 1);
        match *word {
            "graph" | "graphs" => before.is_none_or(|before| !OTHER_GRAPHS.contains(before)),
            "figure" => after != Some(&"out"),
            word => STRONG.contains(&word),
        }
    });
    let weak: HashSet<&str> = words
        .iter()
        .copied()
        .filter(|word| WEAK.contains(word))
        .collect();
    strong || weak.len() >= 2
}

fn data_prep_prompt(prompt: &str) -> bool {
    let lowered = prompt.to_lowercase();
    let words: HashSet<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .collect();
    let preparation = [
        "prepare",
        "clean",
        "cleaning",
        "impute",
        "imputer",
        "scaler",
        "scale",
        "scaling",
        "leakage",
        "outlier",
        "outliers",
        "preprocessing",
        "forbered",
        "rens",
        "manglende",
        "deduplicate",
        "skewed",
        "sentinel",
    ];
    let data = [
        "data",
        "dataset",
        "dataframe",
        "column",
        "columns",
        "target",
        "csv",
        "parquet",
        "pandas",
        "polars",
        "datasættet",
        "kolonnerne",
        "features",
    ];
    let split = lowered.contains("train test split")
        || lowered.contains("train-test split")
        || lowered.contains("test set");
    (preparation.iter().any(|word| words.contains(word))
        || lowered.contains("missing values")
        || lowered.contains("-999"))
        && data.iter().any(|word| words.contains(word))
        || split
            && ["imputer", "scaler", "leakage"]
                .iter()
                .any(|word| words.contains(word))
        || lowered.contains("join keys") && words.contains("deduplicate")
}

fn animation_prompt(prompt: &str) -> bool {
    let lowered = prompt.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let normalized = words.join(" ");
    let contains = |phrases: &[&str]| phrases.iter().any(|phrase| normalized.contains(phrase));
    if contains(&["do not animate", "don t animate", "without animation"]) {
        return false;
    }

    let motion = words
        .iter()
        .any(|word| matches!(*word, "animate" | "animated" | "animation"));
    if (motion || words.contains(&"render")) && words.contains(&"manim") {
        return true;
    }
    if !motion {
        return false;
    }
    if words.contains(&"mp4") || contains(&["explainer video", "animated video"]) {
        return true;
    }
    if contains(&[
        "on hover",
        "on click",
        "css transition",
        "framer motion",
        "when it opens",
        "when it closes",
    ]) {
        return false;
    }
    contains(&[
        "animate this concept",
        "animate the concept",
        "animate this behavior",
        "animate this workflow",
        "animate the algorithm",
        "animate how ",
        "sequence diagram",
        "animation of how ",
        "animated explainer",
        "animation explaining",
        "animation that explains",
    ])
}

/// What the agent needs to follow `playbook`: its name, where to read its code, its commands
/// in order and the rules it satisfies.
fn playbook_lines(playbook: &index::Entry) -> String {
    let recipe = playbook.recipe.as_ref();
    let title = recipe
        .map(|recipe| recipe.title.as_str())
        .unwrap_or_default();
    let commands: Vec<String> = (recipe.iter())
        .flat_map(|recipe| &recipe.commands)
        .enumerate()
        .map(|(nth, command)| format!("{}. {command}", nth + 1))
        .collect();
    format!(
        "xpd has a playbook for this prompt: {name}, \"{title}\".\n\
         `xpd api {name}` prints its code in the house style.\n\
         Its commands, in order:\n\
         {}\n\
         The rules that its code satisfies: {}.",
        commands.join("\n"),
        playbook.rules.join(", "),
        name = playbook.name,
    )
}

/// The open issues of `files` under the style in force, counted by pack and severity, with
/// the rules that have the most.
fn open_issues(project: &Path, files: &[PathBuf]) -> Result<String> {
    let workspace = hook_workspace(project)?;
    let style = workspace.settings.style;
    let checked = format!(
        "xpd checked {} of this project under the {} style",
        files_read(files),
        style.as_str()
    );
    let report = workspace.scan_files(&workspace.collect_files(files)?, None)?;
    if report.issues.is_empty() {
        return Ok(format!("{checked} and found no open issue."));
    }
    let mut packs = BTreeMap::<&str, Vec<&Issue>>::new();
    let mut rules = BTreeMap::<String, usize>::new();
    for issue in &report.issues {
        packs.entry(issue.id.package()).or_default().push(issue);
        *rules.entry(issue.id.to_string()).or_default() += 1;
    }
    let mut lines = vec![format!(
        "{checked} and found {}.",
        counted(report.issues.len(), "open issue")
    )];
    let mut packs: Vec<_> = packs.into_iter().collect();
    packs.sort_by_key(|(name, issues)| (std::cmp::Reverse(issues.len()), *name));
    for (pack, issues) in packs {
        let severities: Vec<String> = [Severity::Error, Severity::Warning, Severity::Info]
            .iter()
            .filter_map(|level| {
                let count = (issues.iter())
                    .filter(|issue| issue.severity == *level)
                    .count();
                let name = level.as_str();
                (count > 0).then(|| match level {
                    Severity::Info => format!("{count} {name}"),
                    _ => counted(count, name),
                })
            })
            .collect();
        lines.push(format!("{pack}: {}", severities.join(", ")));
    }
    let mut rules: Vec<_> = rules.into_iter().collect();
    rules.sort_by_key(|(name, count)| (std::cmp::Reverse(*count), name.clone()));
    let top: Vec<String> = (rules.iter().take(TOP_RULES))
        .map(|(name, count)| format!("{name} ({count})"))
        .collect();
    lines.push(format!(
        "The rules with the most issues: {}.",
        top.join(", ")
    ));
    lines.push(
        "`xpd check . --format json` lists each issue with its file and line, and \
         `xpd api <pack>/<rule>` explains a rule."
            .to_owned(),
    );
    Ok(lines.join("\n"))
}

/// Lints the files that a tool call changed, and hands the issues to the client the way that
/// client reads them.
fn post_tool_use(client: &Client, session: Option<&str>, files: &[PathBuf]) -> Result<ExitCode> {
    let project = match client {
        Client::AgentPlugin { cwd: Some(cwd) } => project_root(cwd)?,
        _ => std::env::current_dir()?,
    };
    let files = Workspace::edited_files(&project, files);
    if files.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    // The lint still runs when the record cannot be written, because the agent needs the
    // issues of this edit either way.
    if let Some(session) = session
        && let Err(e) = record_edits(&project, session, &files)
    {
        eprintln!("xpd hook: cannot record the edit: {e:#}");
    }
    let lines: Vec<String> = (findings(&project, &files)?.into_iter())
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
fn record_edits(project: &Path, session: &str, files: &[PathBuf]) -> Result<()> {
    let session = Session::open(project, session)?;
    for file in files.iter().filter(|file| source::is_python(file)) {
        session.append(&Record::Edited(std::path::absolute(project.join(file))?))?;
    }
    Ok(())
}

/// Lints the Python files that the session changed, and holds the turn while one of them has
/// an error or a warning.
///
/// Claude Code and Codex both read `decision` and `reason` from JSON on stdout, and Codex takes
/// no plain text on this event. Both show `systemMessage` to the user and not to the agent.
fn stop(session: &str, cwd: &Path, turn: Turn) -> Result<ExitCode> {
    let cwd = project_root(cwd)?;
    let session = Session::open(&cwd, session)?;
    let records = session.records()?;
    let (held, advice): (Vec<Finding>, Vec<Finding>) =
        findings(&cwd, &changed_files(&cwd, &records)?)?
            .into_iter()
            .partition(|finding| finding.holds);
    let lines = |found: Vec<Finding>| -> Vec<String> {
        found.into_iter().map(|finding| finding.line).collect()
    };
    let (findings, advice) = (lines(held), lines(advice));
    let profile = profile_turn(&cwd, &records);
    let mut held = findings.clone();
    held.extend(profile.added.iter().cloned());
    let last = (records.iter().rev())
        .find_map(|record| match record {
            Record::Held(lines) => Some(lines.as_slice()),
            Record::Opened { .. }
            | Record::Start(_)
            | Record::Began(_)
            | Record::Edited(_)
            | Record::Profile { .. } => None,
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
        if !findings.is_empty() {
            text.push(
                "xpd holds this turn, because the Python files that this session changed have \
                 errors or warnings. Fix each line below. When the code is right as it is, write \
                 `# ast-grep-ignore: <pack>/<rule>` on the line above it, with the reason after \
                 the rule."
                    .to_owned(),
            );
            text.extend(findings);
        }
        if !profile.added.is_empty() {
            text.push(
                "xpd holds this turn, because this session moved the project away from the \
                 profile that its [tool.xpd.profile] table records. Undo each change below, \
                 run `xpd doctor --apply`, or record an exception with its reason in \
                 [tool.xpd.profile], and run `xpd doctor --check` to confirm."
                    .to_owned(),
            );
            text.extend(profile.added);
        }
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
    text.extend(profile.notices);
    let text = text.join("\n");
    if holds {
        println!("{}", json!({ "decision": "block", "reason": text }));
    } else if !text.is_empty() {
        println!("{}", json!({ "systemMessage": text }));
    }
    Ok(ExitCode::SUCCESS)
}

/// The Python files that the session changed. With git, they are the files that differ from
/// the commit the session began on and the untracked files, when their contents differ from the
/// opening baseline, so an edit through the shell and a committed edit both count. A file that
/// only a pull, a merge or a checkout changed is left out. Without git, they are the files that
/// `PostToolUse` recorded.
fn changed_files(here: &Path, records: &[Record]) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = if let Some(top) = repository(here) {
        let start = records
            .iter()
            .rposition(|record| matches!(record, Record::Start(_) | Record::Opened { .. }));
        let records = &records[start.unwrap_or(0)..];
        let baseline = match records.first() {
            Some(Record::Opened { git, .. }) => git.as_ref().filter(|baseline| baseline.top == top),
            _ => None,
        };
        let began = match records.first() {
            Some(Record::Start(commit)) => commit.as_deref(),
            _ => baseline.map(|baseline| baseline.commit.as_str()),
        };
        let since = records.iter().rev().find_map(|record| match record {
            Record::Began(at) | Record::Opened { at, .. } => Some(*at),
            _ => None,
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
        let (differing, untracked) = git_changes(&top, &base)?;
        let own = since.and_then(|since| own_files(&top, since, baseline));
        let differing = differing
            .split('\0')
            .filter(|name| own.as_ref().is_none_or(|own| own.contains(*name)));
        let candidates = differing
            .chain(untracked.split('\0'))
            .filter(|name| !name.is_empty());
        if let Some(baseline) = baseline {
            let dirty = &baseline.dirty;
            let current = blob_ids(
                &top,
                candidates.clone().filter(|name| dirty.contains_key(*name)),
            )?;
            candidates
                .filter(|name| {
                    current
                        .get(*name)
                        .is_none_or(|id| dirty.get(*name) != Some(id))
                })
                .map(|name| top.join(name))
                .collect()
        } else {
            candidates
                .map(|name| top.join(name))
                .filter(|file| since.is_none_or(|since| written(file) >= since))
                .collect()
        }
    } else {
        (records.iter())
            .filter_map(|record| match record {
                Record::Edited(file) => Some(file.clone()),
                Record::Opened { .. }
                | Record::Start(_)
                | Record::Began(_)
                | Record::Held(_)
                | Record::Profile { .. } => None,
            })
            .collect()
    };
    files.retain(|file| source::is_python(file));
    files.sort();
    files.dedup();
    // A file under the working directory is named from there, as the agent names it.
    Ok(files
        .into_iter()
        .map(|file| {
            file.strip_prefix(here)
                .map(Path::to_path_buf)
                .unwrap_or(file)
        })
        .collect())
}

/// Tracked differences from `base` and untracked paths, both NUL-delimited.
fn git_changes(top: &Path, base: &str) -> Result<(String, String)> {
    let tracked = git::output(
        top,
        &["diff", "--name-only", "-z", "--diff-filter=d", base, "--"],
    )?;
    let untracked = git::output(top, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    Ok((tracked, untracked))
}

/// Content identities of existing Python files. Hashing without `-w` leaves Git's
/// object database alone. Bound the argument list for large dirty worktrees.
fn blob_ids<'a>(
    top: &Path,
    names: impl Iterator<Item = &'a str>,
) -> Result<BTreeMap<String, String>> {
    let names: Vec<_> = names
        .filter(|name| source::is_python(Path::new(name)) && top.join(name).is_file())
        .collect();
    let mut ids = BTreeMap::new();
    for batch in names.chunks(64) {
        let mut args = vec!["hash-object", "--no-filters", "--"];
        args.extend_from_slice(batch);
        let hashes = git::output(top, &args)?;
        ids.extend(
            batch
                .iter()
                .zip(hashes.lines())
                .map(|(&name, id)| (name.to_owned(), id.to_owned())),
        );
    }
    Ok(ids)
}

/// Commits made in this checkout, newest first. Git applies `--since` to the reflog
/// timestamps, which can differ from the dates of the commits themselves.
fn local_commits(top: &Path, since: u64) -> Option<Vec<String>> {
    let log = git(
        top,
        &[
            "reflog",
            "show",
            &format!("--since=@{since}"),
            "--format=%H%x09%gs",
            "HEAD",
        ],
    )?;
    Some(
        log.lines()
            .filter_map(|line| {
                let (commit, action) = line.split_once('\t')?;
                ["commit", "cherry-pick", "revert"]
                    .iter()
                    .any(|verb| action.starts_with(verb))
                    .then(|| commit.to_owned())
            })
            .collect(),
    )
}

/// The files in the repository at `top` that the session itself changed since it began at
/// `since`, in seconds since 1970: the files that differ from the commit checked out now, and
/// the files of each commit that was made in this checkout since then. git logs how each
/// commit was checked out, so a commit that a pull, a merge, a rebase or a checkout brought is
/// told from one that `git commit`, `git cherry-pick` or `git revert` made here. `None` when
/// git keeps no such log, and then every file that differs counts.
fn own_files(top: &Path, since: u64, baseline: Option<&GitBaseline>) -> Option<HashSet<String>> {
    let commits = local_commits(top, since)?;
    let prior = baseline.map_or(&[][..], |baseline| baseline.prior_commits.as_slice());
    let made_here = commits.strip_suffix(prior).unwrap_or(&commits);
    let mut own: HashSet<String> = HashSet::new();
    let mut add = |names: String| {
        own.extend(
            (names.split('\0'))
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
        );
    };
    add(git(top, &["diff", "--name-only", "-z", "HEAD", "--"])?);
    for commit in made_here {
        let args = [
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-z",
            commit,
        ];
        add(git(top, &args)?);
    }
    Some(own)
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
    git::output(folder, args).ok()
}

/// The time now, in seconds since 1970.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// When `file` was last written, in seconds since 1970. A file whose time cannot be read
/// counts as written now, so the Stop check still lints it.
fn written(file: &Path) -> u64 {
    fs::metadata(file)
        .and_then(|data| data.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or_else(now, |since| since.as_secs())
}

/// Git evidence at the opening of this session in this worktree.
#[derive(Serialize, Deserialize)]
struct GitBaseline {
    top: PathBuf,
    commit: String,
    dirty: BTreeMap<String, String>,
    prior_commits: Vec<String>,
}

impl GitBaseline {
    fn capture(top: PathBuf, at: u64) -> Result<Self> {
        let commit = checked_out(&top).context("git names no commit to compare the files with")?;
        let (tracked, untracked) = git_changes(&top, &commit)?;
        let dirty = blob_ids(&top, tracked.split('\0').chain(untracked.split('\0')))?;
        let prior_commits = local_commits(&top, at).unwrap_or_default();
        Ok(Self {
            top,
            commit,
            dirty,
            prior_commits,
        })
    }
}

/// One line of a session file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Record {
    /// One baseline per opening. Older session files have separate Start and Began lines.
    Opened { at: u64, git: Option<GitBaseline> },
    /// The commit that was checked out when the session began. `None` without git.
    Start(Option<String>),
    /// When the session began, in seconds since 1970.
    Began(u64),
    /// A Python file that a tool call changed.
    Edited(PathBuf),
    /// The errors and warnings that the last Stop check held the turn on, and the drift from
    /// the profile that it held the turn on. Empty when it let the turn end.
    Held(Vec<String>),
    /// The drift from `xpd doctor`'s profile when the session opened, in the project at `root`,
    /// which records the profile.
    Profile { root: PathBuf, baseline: Baseline },
}

/// What a project's drift from the profile was at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Baseline {
    /// One line for each difference, as `xpd doctor --check` names it, from the profile of
    /// `revision`, and one line for each lock file that uv could not check offline.
    Drift {
        revision: i64,
        lines: Vec<String>,
        #[serde(default)]
        unverified: Vec<String>,
    },
    /// Why xpd could not compare the project with the profile.
    Unreadable(String),
}

/// The drift of the project at `root` from the profile it records. `None` when it records none.
fn profile_drift(root: &Path) -> Option<Baseline> {
    let files: Vec<PathBuf> = project_files(root, MOST_FILES)
        .into_iter()
        .map(|file| root.join(file))
        .collect();
    crate::doctor::drift(root, &files).map(|drift| match drift {
        Ok(drift) => Baseline::Drift {
            revision: drift.revision,
            lines: drift.lines,
            unverified: drift.unverified,
        },
        Err(error) => Baseline::Unreadable(format!("{error:#}")),
    })
}

/// What the Stop check says about the profile: the drift this session added, which holds the
/// turn, and notices about the rest, which do not.
#[derive(Debug, Default, PartialEq, Eq)]
struct ProfileTurn {
    added: Vec<String>,
    notices: Vec<String>,
}

/// Compares the project's drift from the profile now with the drift when the session opened.
fn profile_turn(root: &Path, records: &[Record]) -> ProfileTurn {
    let opened = records
        .iter()
        .rposition(|record| matches!(record, Record::Start(_) | Record::Opened { .. }));
    let opening = records[opened.unwrap_or(0)..]
        .iter()
        .find_map(|record| match record {
            Record::Profile {
                root: recorded,
                baseline,
            } if recorded == root => Some(baseline),
            _ => None,
        });
    compare_drift(opening, profile_drift(root))
}

/// The drift that is new since `opening`, and a notice for each lock file that uv cannot check
/// now. Without an opening baseline that xpd could read, the drift now is a notice, because xpd
/// cannot tell whether the session made it.
fn compare_drift(opening: Option<&Baseline>, now: Option<Baseline>) -> ProfileTurn {
    let unverified = match &now {
        Some(Baseline::Drift { unverified, .. }) => unverified.clone(),
        _ => Vec::new(),
    };
    let mut turn = compare_lines(opening, now);
    turn.notices.extend(unverified.into_iter().map(|line| {
        format!("xpd cannot tell whether this session left a lock file out of date: {line}")
    }));
    turn
}

/// The difference lines that are new since `opening`. A line about a lock file that uv could not
/// check when the session opened is a notice, because xpd cannot tell whether the session made it.
fn compare_lines(opening: Option<&Baseline>, now: Option<Baseline>) -> ProfileTurn {
    let notice = |lines: Vec<String>| {
        if lines.is_empty() {
            return Vec::new();
        }
        let mut notices = vec![format!(
            "The project has {} from its profile that this session did not make. `xpd doctor` shows them, and `xpd doctor --apply` fixes them.",
            counted(lines.len(), "difference")
        )];
        notices.extend(lines.into_iter().take(MOST_NOTICES));
        notices
    };
    match (opening, now) {
        (_, None) | (None, Some(Baseline::Unreadable(_))) => match opening {
            Some(_) => ProfileTurn {
                added: vec![
                    "pyproject.toml no longer records the profile in [tool.xpd.profile]."
                        .to_owned(),
                ],
                notices: Vec::new(),
            },
            None => ProfileTurn::default(),
        },
        (Some(_), Some(Baseline::Unreadable(error))) => ProfileTurn {
            added: Vec::new(),
            notices: vec![format!(
                "xpd cannot compare the project with its profile: {error}"
            )],
        },
        (
            Some(Baseline::Drift {
                revision: opened, ..
            }),
            Some(Baseline::Drift {
                revision: current,
                lines: now,
                ..
            }),
        ) if *opened != current => {
            let mut notices = vec![format!(
                "The profile moved from revision {opened} to revision {current} during this session, so xpd does not hold the turn for its differences."
            )];
            notices.extend(notice(now));
            ProfileTurn {
                added: Vec::new(),
                notices,
            }
        }
        (
            Some(Baseline::Drift {
                lines: before,
                unverified: unchecked,
                ..
            }),
            Some(Baseline::Drift { lines: now, .. }),
        ) => {
            let file = |line: &str| line.split_once(": ").map(|(file, _)| file.to_owned());
            let unchecked: Vec<String> = unchecked.iter().filter_map(|line| file(line)).collect();
            let (kept, added): (Vec<String>, Vec<String>) = now.into_iter().partition(|line| {
                before.contains(line) || file(line).is_some_and(|file| unchecked.contains(&file))
            });
            ProfileTurn {
                added,
                notices: notice(kept),
            }
        }
        (None | Some(Baseline::Unreadable(_)), Some(Baseline::Drift { lines: now, .. })) => {
            ProfileTurn {
                added: Vec::new(),
                notices: notice(now),
            }
        }
    }
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
    fn open(project: &Path, id: &str) -> Result<Self> {
        let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
        anyhow::ensure!(
            !id.is_empty() && !id.starts_with('.') && id.chars().all(plain),
            "the session id {id:?} cannot name a session file"
        );
        Ok(Self {
            file: project
                .join(embed::cache_folder()?)
                .join("sessions")
                .join(id),
        })
    }

    /// The lines of the file, oldest first. A session that wrote nothing yet has none. A line
    /// that this version cannot read is left out, because a later version may have written it.
    fn records(&self) -> Result<Vec<Record>> {
        let text = match fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).context(source::cannot_read(&self.file)),
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
        let mut file = crate::private::options()
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

/// The issues of the files in `files`. A Python file with a pipeline of steps, or one that
/// imports Hamilton, also gets the errors and warnings that `xpd graph` finds in that file.
fn findings(project: &Path, files: &[PathBuf]) -> Result<Vec<Finding>> {
    // One call can edit a file and then move or delete it, so a named file may be gone.
    let files: Vec<PathBuf> = (files.iter())
        .filter(|file| project.join(file).is_file())
        .cloned()
        .collect();
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let workspace = hook_workspace(project)?;
    let report = workspace.scan_files(&workspace.collect_files(&files)?, None)?;
    let issues = report.issues.iter().map(|issue| Finding {
        line: issue.line(Applicability::Safe),
        holds: issue.severity >= Severity::Warning,
    });
    let python = files.iter().filter(|file| source::is_python(file));
    let steps = python.flat_map(|file| {
        fs::read_to_string(project.join(file))
            .map(|src| Graph::edit_lines(file, &src))
            .unwrap_or_default()
    });
    let steps = steps.map(|line| Finding { line, holds: true });
    Ok(issues.chain(steps).collect())
}

fn project_root(cwd: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        !cwd.as_os_str().is_empty(),
        "cannot enter the agent's working directory: empty path"
    );
    fs::canonicalize(cwd.join(".")).context("cannot enter the agent's working directory")
}

fn hook_workspace(project: &Path) -> Result<Workspace> {
    let settings = Settings::find_in(project, project)?;
    let venv = Venv::find_from(project).map(|venv| venv.in_directory(project));
    Workspace::load(
        project.to_path_buf(),
        None,
        settings,
        venv,
        &Filters::default(),
    )
}
