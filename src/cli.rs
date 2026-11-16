//! The command line: its arguments, and what each command prints and exits with.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;

use crate::catalog;
use crate::graph::Graph;
use crate::index::{self, Kind};
use crate::lsp;
use crate::mcp;
use crate::pack;
use crate::package_check;
use crate::report::Report;
use crate::rule::Applicability;
use crate::scan;
use crate::skills;
use crate::venv::{self, Venv};

/// The agent skill that `xpd skill` prints.
const SKILL: &str = include_str!("../skills/xpd/SKILL.md");
/// The scripts that `profile`, `refit` and `equiv` hand to the project's Python.
const PROFILE_PY: &str = include_str!("../py/profile.py");
const REFIT_PY: &str = include_str!("../py/refit.py");
const EQUIV_PY: &str = include_str!("../py/equiv.py");

/// Exit code when xpd itself fails, as opposed to finding issues.
const CHECK_FAILED: u8 = 2;
/// Claude Code shows stderr to the agent only when a hook exits 2.
const HOOK_BLOCK: u8 = 2;
/// Any other non-zero hook exit shows stderr to the user without blocking the agent.
const HOOK_FAILED: u8 = 1;
/// The shell's code for a command that could not be started.
const PYTHON_NOT_STARTED: u8 = 127;

/// xpd: fast linter for Python library habits, for humans and coding agents.
#[derive(Debug, Parser)]
#[command(name = "xpd", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

/// How `check`, `graph` and `rules` print what they found.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Debug, clap::Args)]
struct IndexArgs {
    source: PathBuf,
    #[arg(long)]
    package: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Documentation and gallery root to harvest with the symbol index.
    #[arg(long, requires_all = ["source_url", "license"])]
    examples: Option<PathBuf>,
    /// Repository URL ending in /blob/<pinned revision> for example attribution.
    #[arg(long, requires = "examples")]
    source_url: Option<String>,
    #[arg(long, requires = "examples")]
    license: Option<PathBuf>,
    #[arg(long, requires = "examples")]
    notice: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Find libraries and rules for a task, without importing Python.
    Find {
        query: Option<String>,
        #[arg(long)]
        package: Option<String>,
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        /// Maximum results per kind (defaults to 5 symbols, 3 rules and 3 recipes).
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Additional index generated from local source.
        #[arg(long)]
        index: Option<PathBuf>,
        /// Run real discovery questions and fail if an expected entry misses the top five.
        #[arg(long, conflicts_with = "query")]
        check: Option<PathBuf>,
        /// Embed the index records that have no vector yet and write all vectors to this file,
        /// which is indexes/vectors.bin in the repository. Remove the file and build again to
        /// embed every record.
        #[arg(long, hide = true, conflicts_with_all = ["query", "check", "index"])]
        write_vectors: Option<PathBuf>,
    },
    /// Show a public symbol's parameters, summary, own members and guarding rules.
    Api {
        name: String,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        #[arg(long)]
        index: Option<PathBuf>,
    },
    /// Parse library source into a versioned index. Never imports or runs library code.
    Index(IndexArgs),
    /// Lint Python files. Exit 0 when clean, 1 on any error-severity issue, 2 if xpd can't run.
    ///
    /// A library's rules run only on files that import it, and the Python rules run on every file.
    /// `[*]` marks an issue `--fix` fixes.
    #[command(after_long_help = "Examples:
  xpd check src/
  xpd check analysis.py --format json
  xpd check . --fix
  xpd check . --fix --unsafe-fixes")]
    Check {
        /// Files or directories (directories respect .gitignore).
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
        /// Apply safe fixes, and report what is left.
        #[arg(long)]
        fix: bool,
        /// Also count unsafe fixes, which may change behavior: apply them with --fix, or mark
        /// them as fixable without it.
        #[arg(long)]
        unsafe_fixes: bool,
    },
    /// `PostToolUse` hook: reads the event JSON on stdin and lints the .py files it changed.
    ///
    /// A file with a pipeline of steps also gets the errors and warnings of `xpd graph`.
    ///
    /// For a client of the Agent Plugins 1.1.0 draft, it prints the issues as
    /// `additional_context` and exits 0.
    /// As a hook in the Claude Code settings, it prints them on stderr and exits 2. It exits 1
    /// if xpd can't run.
    Hook,
    /// Profile a data file (csv, parquet, feather, json, xlsx). Needs pandas.
    Profile { file: PathBuf },
    /// Check that two scripts produce an equal `result` (`DataFrame` or Series). Needs pandas.
    Equiv { old: PathBuf, new: PathBuf },
    /// Refit a saved Pipeline on raw training rows and compare its outputs and scores.
    Refit { contract: PathBuf },
    /// Print the dataflow of step code or Hamilton modules: what reads what, and what does not
    /// connect.
    ///
    /// A file with a function that passes a frame through other functions, as
    /// `frame.pipe(a).pipe(b)`, is read as step code. The other files are read as the Hamilton
    /// modules of one Driver. Nothing is imported or run. Exit 1 when the code would raise or
    /// Python drops a function, 2 if xpd can't run.
    #[command(after_long_help = "Examples:
  xpd graph features.py
  xpd graph loaders.py features.py --format json")]
    Graph {
        /// Files or directories (directories respect .gitignore).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// List every rule and package check, with its severity, fix and documentation.
    ///
    /// The JSON has one object per rule, with `package`, `rule`, `severity`, `message`, `fix`,
    /// `since`, `url` and `doc`, the rule's section of the rules page as Markdown.
    #[command(after_long_help = "Examples:
  xpd rules
  xpd rules --format json | jq '.[] | select(.package == \"numpy\")'")]
    Rules {
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// Print the agent skill (SKILL.md) for this xpd version.
    Skill {
        /// Print every skill that ships with xpd, with the path and text of each of its files,
        /// as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Start the MCP server
    ///
    /// The arguments go to the server, so `xpd mcp -h` lists its flags.
    #[command(disable_help_flag = true)]
    Mcp {
        /// Arguments for the server, such as --transport http.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Start the language server, which an editor runs to show the issues as you type.
    ///
    /// It speaks the Language Server Protocol on stdin and stdout, and lints the text the editor
    /// holds, saved or not. Exit 0 after `shutdown` and `exit`, 1 when the client goes away
    /// without them.
    Lsp,
}

/// Runs the command the arguments name.
pub(crate) fn run() -> ExitCode {
    use std::io::IsTerminal;

    let default_format = if io::stdout().is_terminal() {
        Format::Text
    } else {
        Format::Json
    };
    match Cli::parse().cmd {
        Cmd::Check {
            paths,
            format,
            fix,
            unsafe_fixes,
        } => {
            let format = format.unwrap_or(default_format);
            let allowed = if unsafe_fixes {
                Applicability::Unsafe
            } else {
                Applicability::Safe
            };
            check(&paths, format, fix.then_some(allowed), allowed)
        }
        Cmd::Find {
            query,
            package,
            kind,
            limit,
            format,
            index,
            check,
            write_vectors,
        } => discovery((|| {
            if let Some(path) = write_vectors {
                index::write_vectors(&path)
            } else if let Some(path) = check {
                check_ranking(index.as_deref(), &path)
            } else {
                let entries = index::load(index.as_deref())?;
                let query = query.context("provide a task query or --check")?;
                let found = index::find(&entries, &query, package.as_deref(), kind, limit);
                print_matches(found, kind, format)
            }
        })()),
        Cmd::Api {
            name,
            format,
            index,
        } => discovery((|| {
            let entries = index::load(index.as_deref())?;
            let entry =
                index::api(&entries, &name)?.with_context(|| format!("no entry named {name}"))?;
            print_entries(&[&entry], format, Detail::Full)
        })()),
        Cmd::Index(args) => discovery(refresh_index(args)),
        Cmd::Hook => hook(),
        Cmd::Graph { paths, format } => graph(&paths, format.unwrap_or(default_format)),
        Cmd::Profile { file } => run_py(PROFILE_PY, &[file]),
        Cmd::Refit { contract } => run_py(REFIT_PY, &[contract]),
        Cmd::Equiv { old, new } => run_py(EQUIV_PY, &[old, new]),
        Cmd::Rules { format } => rules(format.unwrap_or(default_format)),
        Cmd::Skill { json: false } => {
            print!("{SKILL}");
            ExitCode::SUCCESS
        }
        Cmd::Skill { json: true } => {
            println!("{}", skills::to_json());
            ExitCode::SUCCESS
        }
        Cmd::Mcp { args } => mcp::run(&args),
        Cmd::Lsp => lsp::run(),
    }
}

/// Prints what `find` found, each entry with the first paragraph of its summary.
fn print_matches(mut found: index::Matches, kind: Option<Kind>, format: Format) -> Result<()> {
    for group in [&mut found.symbols, &mut found.rules, &mut found.recipes] {
        for entry in group {
            entry.keywords.clear();
            entry.members.clear();
            entry.summary = entry
                .summary
                .split("\n\n")
                .next()
                .unwrap_or_default()
                .to_owned();
        }
    }
    match format {
        Format::Json => println!("{}", serde_json::to_string(&found)?),
        Format::Text => {
            for (heading, selected, group) in [
                ("Symbols", Kind::Symbol, &found.symbols),
                ("Rules", Kind::Rule, &found.rules),
                ("Recipes", Kind::Recipe, &found.recipes),
            ] {
                if kind.is_none_or(|k| k == selected) {
                    println!("{heading}");
                    print_entries(&group.iter().collect::<Vec<_>>(), format, Detail::Summary)?;
                }
            }
        }
    }
    Ok(())
}

/// Prints every rule of every pack, whatever the project's venv holds.
fn refresh_index(args: IndexArgs) -> Result<()> {
    let IndexArgs {
        source,
        package,
        version,
        output,
        examples,
        source_url: url,
        license,
        notice,
    } = args;
    let version = version.map_or_else(|| index::source_version(&source, &package), Ok)?;
    let output = output.unwrap_or_else(|| PathBuf::from(format!("indexes/{package}.json")));
    let mut index = index::generate(&source, &package, &version)?;
    if let (Some(root), Some(url), Some(license)) =
        (examples.as_deref(), url.as_deref(), license.as_deref())
    {
        let stats = crate::recipe::harvest(
            &crate::recipe::Source {
                root,
                url,
                license,
                notice: notice.as_deref(),
            },
            &package,
            &mut index.entries,
        )?;
        println!(
            "{package} recipes: {} candidates, {} clean, {} after fixes, {} dropped, {} capped, {} shipped",
            stats.candidates, stats.clean, stats.fixed, stats.dropped, stats.capped, stats.retained
        );
        index.recipes = Some(stats);
    } else if let Some(previous) = std::fs::read(&output)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<index::Index>(&bytes).ok())
        .filter(|previous| previous.package == package && previous.version == version)
    {
        // A symbol rebuild keeps the recipes that an earlier run harvested for this version.
        index.entries.extend(
            previous
                .entries
                .into_iter()
                .filter(|entry| entry.kind == index::Kind::Recipe),
        );
        index.recipes = previous.recipes;
    }
    std::fs::write(&output, serde_json::to_vec(&index)?)?;
    println!(
        "{package} {version}: {} entries written to {}",
        index.entries.len(),
        output.display()
    );
    Ok(())
}

fn rules(format: Format) -> ExitCode {
    let packs = match pack::load(None) {
        Ok(packs) => packs,
        Err(e) => {
            eprintln!("xpd: invalid embedded rules: {e:#}");
            return ExitCode::from(CHECK_FAILED);
        }
    };
    let entries = catalog::entries(&packs);
    match format {
        Format::Json => {
            let rules: Vec<Value> = entries.iter().map(catalog::Entry::to_json).collect();
            println!("{}", Value::Array(rules));
        }
        Format::Text => {
            for entry in &entries {
                println!("{}", entry.line());
            }
        }
    }
    ExitCode::SUCCESS
}

/// Lints `paths` with the rules for the library versions in the project's venv, after writing
/// fixes up to `fix` when it is set.
fn lint(paths: &[PathBuf], fix: Option<Applicability>, scope: Scope) -> Result<Report> {
    let venv = Venv::find();
    let installed = venv.as_ref().map(Venv::distributions);
    let packs = pack::load(installed.as_ref()).context("invalid embedded rules")?;
    let files = scan::collect_files(paths)?;
    let scanned = scan::scan(&files, &packs, fix)?;
    let mut issues = scanned.issues;
    if let (Scope::Project, Some(venv), Some(installed)) = (scope, &venv, &installed) {
        // Without a working directory there is no project folder to look in.
        let project = std::env::current_dir().unwrap_or_default();
        issues.extend(package_check::missing(&packs, venv, installed, &project));
    }
    Ok(Report {
        issues,
        fixed: scanned.fixed,
    })
}

/// How much a lint run looks at.
#[derive(Debug, Clone, Copy)]
enum Scope {
    /// The given files only. The hook runs after every edit, so it stops here.
    Files,
    /// The files, and the packages the project's venv lacks.
    Project,
}

/// `allowed` is the fix level that `[*]` and the summary count as fixable.
fn check(
    paths: &[PathBuf],
    format: Format,
    fix: Option<Applicability>,
    allowed: Applicability,
) -> ExitCode {
    let report = match lint(paths, fix, Scope::Project) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xpd: {e:#}");
            return ExitCode::from(CHECK_FAILED);
        }
    };
    match format {
        Format::Json => println!("{}", report.to_json(allowed)),
        Format::Text => {
            for i in &report.issues {
                println!("{}", i.line(allowed));
            }
            eprintln!("xpd: {}", report.summary(allowed));
        }
    }
    if report.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Prints the dataflow of the files under `paths`.
fn graph(paths: &[PathBuf], format: Format) -> ExitCode {
    let graph = match scan::collect_files(paths).and_then(|files| Graph::read(&files)) {
        Ok(graph) => graph,
        Err(e) => {
            eprintln!("xpd: {e:#}");
            return ExitCode::from(CHECK_FAILED);
        }
    };
    match format {
        Format::Json => println!("{}", graph.to_json()),
        Format::Text => {
            println!("{}", graph.to_text());
            eprintln!("xpd: {}", graph.summary());
        }
    }
    if graph.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A `PostToolUse` event, from one of the two clients that `xpd hook` answers.
enum HookEvent {
    /// A client of the Agent Plugins specification, which names the completed call's result
    /// `tool_result`. It reads the issues from stdout as `additional_context`.
    AgentPlugin {
        /// The agent's working directory. The hook itself starts in the plugin's folder.
        cwd: Option<PathBuf>,
        files: Vec<PathBuf>,
    },
    /// A hook in the Claude Code settings. It reads the issues from stderr when the exit code
    /// is 2.
    ClaudeCode { files: Vec<PathBuf> },
}

impl HookEvent {
    fn parse(event: &Value) -> Self {
        let path =
            |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(PathBuf::from);
        if event.get("tool_result").is_none() {
            let files = event
                .get("tool_input")
                .and_then(|input| path(input, "file_path"));
            return Self::ClaudeCode {
                files: files.into_iter().collect(),
            };
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
        Self::AgentPlugin {
            cwd: path(event, "cwd"),
            files,
        }
    }
}

/// Lints the Python files that the hook event on stdin changed, and hands the issues to the
/// client the way that client reads them.
fn hook() -> ExitCode {
    use std::io::Read;

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("xpd hook: cannot read stdin: {e}");
        return ExitCode::from(HOOK_FAILED);
    }
    let event = match serde_json::from_str(&input) {
        Ok(event) => HookEvent::parse(&event),
        Err(e) => {
            eprintln!("xpd hook: stdin is not hook JSON: {e}");
            return ExitCode::from(HOOK_FAILED);
        }
    };
    let (cwd, files) = match &event {
        HookEvent::AgentPlugin { cwd, files } => (cwd.as_ref(), files),
        HookEvent::ClaudeCode { files } => (None, files),
    };
    let files: Vec<&PathBuf> = files.iter().filter(|file| scan::is_python(file)).collect();
    if files.is_empty() {
        return ExitCode::SUCCESS;
    }
    // The project's venv is found from the working directory, as in a user's run.
    if let Some(Err(e)) = cwd.map(std::env::set_current_dir) {
        eprintln!("xpd hook: cannot enter the agent's working directory: {e}");
        return ExitCode::from(HOOK_FAILED);
    }
    let lines = match hook_lines(&files) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("xpd hook: {e:#}");
            return ExitCode::from(HOOK_FAILED);
        }
    };
    if lines.is_empty() {
        return ExitCode::SUCCESS;
    }
    match event {
        HookEvent::AgentPlugin { .. } => {
            println!(
                "{}",
                serde_json::json!({ "additional_context": lines.join("\n") })
            );
            ExitCode::SUCCESS
        }
        HookEvent::ClaudeCode { .. } => {
            eprintln!("{}", lines.join("\n"));
            ExitCode::from(HOOK_BLOCK)
        }
    }
}

/// The issues of the Python files in `files`. A file with a pipeline of steps also gets the
/// errors and warnings of `xpd graph`.
fn hook_lines(files: &[&PathBuf]) -> Result<Vec<String>> {
    // One call can edit a file and then move or delete it, so a named file may be gone.
    let files: Vec<PathBuf> = files
        .iter()
        .filter(|file| file.is_file())
        .map(|file| (*file).clone())
        .collect();
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let report = lint(&files, None, Scope::Files)?;
    let issues = report
        .issues
        .iter()
        .map(|issue| issue.line(Applicability::Safe));
    let steps = files.iter().flat_map(|file| {
        std::fs::read_to_string(file)
            .map(|src| Graph::step_lines(file, &src))
            .unwrap_or_default()
    });
    Ok(issues.chain(steps).collect())
}

/// Runs `script` with the project's Python and passes on its exit code.
fn run_py(script: &str, args: &[PathBuf]) -> ExitCode {
    match Command::new(venv::project_python())
        .arg("-c")
        .arg(script)
        .args(args)
        .status()
    {
        Ok(status) => status
            .code()
            .and_then(|c| u8::try_from(c).ok())
            .map_or(ExitCode::FAILURE, ExitCode::from),
        Err(e) => {
            eprintln!("xpd: could not start python: {e}");
            ExitCode::from(PYTHON_NOT_STARTED)
        }
    }
}

fn discovery(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xpd: {error:#}");
            ExitCode::from(CHECK_FAILED)
        }
    }
}

#[derive(Clone, Copy)]
enum Detail {
    Summary,
    Full,
}

fn print_entries(entries: &[&index::Entry], format: Format, detail: Detail) -> Result<()> {
    let entries: Vec<_> = entries
        .iter()
        .map(|entry| {
            let mut entry = (*entry).clone();
            entry.keywords.clear();
            if matches!(detail, Detail::Summary) {
                entry.members.clear();
                entry.summary = entry
                    .summary
                    .split("\n\n")
                    .next()
                    .unwrap_or_default()
                    .to_owned();
            }
            entry
        })
        .collect();
    match format {
        Format::Json => println!("{}", serde_json::to_string(&entries)?),
        Format::Text => {
            for entry in entries {
                let signature = if entry.kind == Kind::Symbol {
                    let params = if matches!(detail, Detail::Summary)
                        && entry.parameters.join(", ").len() > 70
                    {
                        format!(
                            "{}, ...",
                            entry
                                .parameters
                                .iter()
                                .take(4)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    } else {
                        entry.parameters.join(", ")
                    };
                    format!("({params})")
                } else {
                    String::new()
                };
                println!(
                    "{}{signature} [{}]\n{}",
                    entry.name,
                    entry
                        .callable
                        .as_deref()
                        .unwrap_or(if entry.kind == Kind::Recipe {
                            "recipe"
                        } else {
                            "rule"
                        }),
                    entry.summary
                );
                if matches!(detail, Detail::Full)
                    && let Some(recipe) = &entry.recipe
                {
                    println!(
                        "Source: {}\nSymbols: {}\nLicense: {}\n{}\nModified by xpd fixes: {}",
                        recipe.source,
                        recipe.symbols.join(", "),
                        recipe.license,
                        recipe.attribution,
                        recipe.modified
                    );
                    if let Some(code) = &recipe.code {
                        println!("\n```python\n{code}```");
                    }
                }
                for member in &entry.members {
                    println!(
                        "  {}({}) {}",
                        member.name,
                        member.parameters.join(", "),
                        member.summary
                    );
                }
                if !entry.rules.is_empty() {
                    println!("Rules: {}", entry.rules.join(", "));
                }
            }
        }
    }
    Ok(())
}

/// Asks each question of `path` and fails when its expected entry misses the top five. A
/// question is asked with keyword search, which is what runs without the embedding model, and
/// again with the model when it is there. A question with `"needs_model": true` is asked only
/// with the model.
fn check_ranking(index: Option<&std::path::Path>, path: &std::path::Path) -> Result<()> {
    let questions: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let with_model = index::load(index)?;
    let keywords = index::load(index)?.keywords_only();
    let searches = [
        ("keywords", &keywords),
        ("keywords and the embedding model", &with_model),
    ];
    // An open question records what a user asked and the answer they expect. Search cannot
    // answer it yet, so it is kept as data and not checked.
    let (open, questions): (Vec<_>, Vec<_>) = questions
        .into_iter()
        .partition(|question| question["open"] == true);
    let has_model = with_model.has_model();
    let mut asked = [0; 2];
    let mut failed = [0; 2];
    for question in &questions {
        let query = question["query"].as_str().context("question needs query")?;
        // A list names entries that each answer the question.
        let expected: Vec<&str> = match &question["expected"] {
            Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
            name => name.as_str().into_iter().collect(),
        };
        let mut kinds = Vec::new();
        for name in &expected {
            let entry = index::api(&keywords, name)?
                .with_context(|| format!("expected entry {name} is absent from the index"))?;
            kinds.push(entry.kind);
        }
        let kind = *kinds.first().context("question needs expected")?;
        anyhow::ensure!(
            kinds.iter().all(|k| *k == kind),
            "{query}: the expected entries must be of one kind"
        );
        let needs_model = question["needs_model"] == true;
        for (search, (name, library)) in searches.iter().enumerate() {
            if (search == 0 && needs_model) || (search == 1 && !has_model) {
                continue;
            }
            asked[search] += 1;
            let matches = index::find(library, query, None, Some(kind), Some(5));
            let found = match kind {
                Kind::Symbol => matches.symbols,
                Kind::Rule => matches.rules,
                Kind::Recipe => matches.recipes,
            };
            if !found.iter().any(|e| expected.contains(&e.name.as_str())) {
                failed[search] += 1;
                println!(
                    "FAIL with {name}: {query} expected {}; found {}",
                    expected.join(" or "),
                    found
                        .iter()
                        .map(|e| e.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    }
    for (search, (name, _)) in searches.iter().enumerate() {
        if search == 1 && !has_model {
            println!("The embedding model is absent, so search with it is not checked");
        } else {
            println!(
                "{} of {} questions passed with {name}",
                asked[search] - failed[search],
                asked[search]
            );
        }
    }
    println!("{} open questions are recorded and not checked", open.len());
    let missing = index::records_without_vector();
    if let Some(first) = missing.first() {
        println!(
            "{} records have no vector in indexes/vectors.bin, such as {first}. Write it again with xpd find --write-vectors indexes/vectors.bin",
            missing.len()
        );
    }
    let failed = failed[0] + failed[1];
    anyhow::ensure!(failed == 0, "{failed} discovery questions failed");
    anyhow::ensure!(
        missing.is_empty(),
        "{} records have no vector",
        missing.len()
    );
    Ok(())
}
