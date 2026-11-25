//! The command line: its arguments, and what each command prints and exits with.

use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::severity::Severity;
use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Generator;
use serde_json::Value;

use crate::author;
use crate::catalog::{Catalog, Filters};
use crate::category::{Categories, Category};
use crate::check::FixMode;
use crate::fix_safety::Applicability;
use crate::graph::Graph;
use crate::impact::ImpactArgs;
use crate::index::{self, Kind};
use crate::lsp;
use crate::mcp;
use crate::migration::MapArgs;
use crate::outline::OutlineArgs;
use crate::output::Format;
use crate::query::RunArgs;
use crate::settings::Settings;
use crate::skills;
use crate::source;
use crate::style::Style;
use crate::venv;
use crate::workspace::Workspace;

/// The agent skill that `xpd skill` prints.
const SKILL: &str = include_str!("../skills/xpd/SKILL.md");
/// The scripts that `profile`, `refit` and `equiv` hand to the project's Python.
const PROFILE_PY: &str = include_str!("../py/profile.py");
const REFIT_PY: &str = include_str!("../py/refit.py");
const EQUIV_PY: &str = include_str!("../py/equiv.py");

/// Exit code when xpd itself fails, as opposed to finding issues.
const CHECK_FAILED: u8 = 2;
/// The shell's code for a command that could not be started.
const PYTHON_NOT_STARTED: u8 = 127;

/// xpd: fast linter for Python library habits, for humans and coding agents.
#[derive(Debug, Parser)]
#[command(name = "xpd", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CheckFormat {
    Text,
    Json,
    Sarif,
    Github,
}

#[derive(Debug, clap::Args)]
struct IndexArgs {
    /// The library's source folder or installed package folder, such as a site-packages folder.
    source: PathBuf,
    /// The Python import name of the library, such as `sklego`.
    #[arg(long)]
    package: String,
    /// The library version. Default: the version in the wheel metadata, so give it for a
    /// source checkout.
    #[arg(long)]
    version: Option<String>,
    /// The file to write. Default: indexes/<package>.json, in a folder that exists.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Documentation and gallery root to harvest with the symbol index.
    #[arg(long, requires_all = ["source_url", "license"])]
    examples: Option<PathBuf>,
    /// Repository URL ending in /blob/<pinned revision> for example attribution.
    #[arg(long, requires = "examples")]
    source_url: Option<String>,
    /// The source's license file, which names the license of each recipe.
    #[arg(long, requires = "examples")]
    license: Option<PathBuf>,
    /// Report each public module's category and source on stderr.
    #[arg(long)]
    category_report: bool,
}

#[derive(Debug, clap::Args)]
struct RulesOptions {
    /// List only the rules that ship with xpd, without the project's own rules.
    #[arg(long, conflicts_with = "config")]
    builtin: bool,
    /// The project file to read instead of the nearest sgconfig.yml.
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// List only this package or rule, as `pandas`, `chained-assignment` or
    /// `pandas/chained-assignment`. Repeat it to name several.
    #[arg(long = "rule", value_name = "NAME")]
    rules: Vec<String>,
    /// List only the rules of this severity and above: error, warning, info or hint.
    #[arg(long, value_parser = Severity::parse)]
    severity: Option<Severity>,
    /// List only rules in this category, and show their categories.
    #[arg(long, value_enum)]
    category: Option<Category>,
    /// Show the categories of each rule.
    #[arg(long)]
    categories: bool,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

#[derive(Debug, clap::Args)]
struct FindArgs {
    /// The task, in your own words, such as "grouped regression".
    query: Option<String>,
    /// Search only this library, by its Python import name, such as `sklearn`.
    #[arg(long)]
    package: Option<String>,
    /// Search only symbols, rules or recipes.
    #[arg(long, value_enum)]
    kind: Option<Kind>,
    /// Search only this category.
    #[arg(long, value_enum, conflicts_with_all = ["check", "write_vectors"])]
    category: Option<Category>,
    /// Maximum results per kind (defaults to 5 symbols, 3 rules and 3 recipes).
    #[arg(long)]
    limit: Option<usize>,
    /// Print text, or JSON with `symbols`, `rules` and `recipes`.
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    /// Additional index generated from local source.
    #[arg(long)]
    index: Option<PathBuf>,
    /// Run real discovery questions and fail if an expected entry misses the top five.
    #[arg(long, conflicts_with = "query")]
    check: Option<PathBuf>,
    /// Embed the index records that are new or whose text changed, and write all vectors to
    /// this file, which is indexes/vectors.bin in the repository. Remove the file and build
    /// again to embed every record.
    #[arg(long, hide = true, conflicts_with_all = ["query", "check", "index"])]
    write_vectors: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
struct CheckArgs {
    /// Files or directories (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum)]
    format: Option<CheckFormat>,
    #[command(flatten)]
    fixes: FixArgs,
    /// Run only this package or rule, as `pandas`, `chained-assignment` or
    /// `pandas/chained-assignment`. Repeat it to name several.
    #[arg(long = "rule", value_name = "NAME")]
    rules: Vec<String>,
    /// Run only the rules of this severity and above: error, warning, info or hint.
    #[arg(long, value_parser = Severity::parse)]
    severity: Option<Severity>,
    /// Exit 1 when an issue of this severity or above is left: error, warning, info or hint.
    #[arg(long, value_parser = Severity::parse, default_value = "error")]
    fail_on: Severity,
    /// Count remaining issues by category on stderr. An issue can count in several categories.
    #[arg(long)]
    category_summary: bool,
    /// The project file to read instead of the nearest sgconfig.yml.
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
struct FixArgs {
    /// Apply safe fixes, and report what is left.
    #[arg(long)]
    fix: bool,
    /// Review each proposed fix before writing it.
    #[arg(long, short = 'i', requires = "fix")]
    review: bool,
    /// Also count unsafe fixes, which may change behavior: apply them with --fix, or mark
    /// them as fixable without it.
    #[arg(long)]
    unsafe_fixes: bool,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Check rule examples and review diagnostic and fix snapshots.
    Test(crate::cases::Options),
    /// Find libraries and rules for a task, without importing Python.
    Find(FindArgs),
    /// Show a public symbol's parameters as the source writes them, with its summary, members
    /// and guarding rules.
    ///
    /// A parameter keeps its type annotation and its default. A stub file writes `...` for a
    /// default, which says only that the parameter is optional. The name is a class or a
    /// function that `find` returns, a method or property of a class, a rule or a recipe.
    #[command(after_long_help = "Examples:
  xpd api sklego.model_selection.TimeGapSplit
  xpd api mapie.regression.TimeSeriesRegressor.update
  xpd api pandas.DataFrame.merge --format json")]
    Api {
        /// A dotted name, such as `pandas.DataFrame` or `pandas.DataFrame.merge`.
        name: String,
        /// Show the entry's categories.
        #[arg(long)]
        categories: bool,
        /// Print text, or JSON with the entry's parameters, members and rules.
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Additional index generated from local source.
        #[arg(long)]
        index: Option<PathBuf>,
    },
    /// Parse library source into a versioned index. Never imports or runs library code.
    Index(IndexArgs),
    /// Lint Python files. Exit 0 when clean, 1 on any issue of the --fail-on severity, which is
    /// error by default, 2 if xpd can't run.
    ///
    /// A library's rules run only on files that import it, and the Python rules run on every file.
    /// A project with an sgconfig.yml adds its own ast-grep rules, and its xpd.yml can make a
    /// rule folder a pack. `# ast-grep-ignore: pandas/chained-assignment` silences one rule on
    /// the next line.
    /// `[*]` marks an issue `--fix` fixes.
    #[command(after_long_help = "Examples:
  xpd check src/
  xpd check analysis.py --format json
  xpd check . --fix
  xpd check . --fix --unsafe-fixes")]
    #[command(visible_alias = "scan")]
    Check(CheckArgs),
    /// Agent hook: reads an event's JSON on stdin, lints the files an edit changed, and holds a
    /// turn that leaves errors or warnings.
    ///
    /// On `PostToolUse` it lints the .py files that the event changed. In a project with an
    /// sgconfig.yml it also lints the files its own rules are for. A file with a pipeline of
    /// steps also gets the errors and warnings of `xpd graph`. For a client of the Agent Plugins
    /// 1.1.0 draft, it prints the issues as `additional_context` and exits 0. As a hook in the
    /// Claude Code settings, it prints them on stderr and exits 2.
    ///
    /// On `SessionStart` it records the commit that is checked out, and in a project with a
    /// Python file it prints a brief for the agent with the rule packs that apply, the style
    /// setting, and the pipelines and Hamilton nodes that `xpd graph` reads. On
    /// `UserPromptSubmit` it prints the playbook that answers the prompt, and the open issues
    /// of the project when the prompt asks what to make or improve. A prompt about other work
    /// gets no text.
    ///
    /// On `Stop` it lints the Python files that differ from the recorded commit and the
    /// untracked ones. While one has an error
    /// or a warning, it prints `{"decision": "block", "reason": ...}` with the issues and exits
    /// 0, which keeps the agent working in Claude Code and in Codex. It holds a turn once for
    /// the same list of issues.
    ///
    /// It exits 1 if xpd can't run.
    Hook,
    /// Profile a data file (csv, parquet, feather, json, xlsx). Needs pandas.
    Profile {
        /// The data file.
        file: PathBuf,
    },
    /// Check that two scripts produce an equal `result` (`DataFrame` or Series). Needs pandas.
    ///
    /// With `--since`, name one script, and xpd compares it with the version of the same file
    /// in that git commit, so a script that was edited in place needs no second file.
    #[command(
        after_help = "Examples:\n  xpd equiv old.py new.py\n  xpd equiv report.py --since HEAD"
    )]
    Equiv {
        /// The script before the change and the script after it. With `--since`, the one script
        /// after the change. Each must assign its output to `result`.
        #[arg(required = true, num_args = 1..=2, value_name = "SCRIPT")]
        scripts: Vec<PathBuf>,
        /// Take the script before the change from this git commit, such as HEAD or main.
        #[arg(long, value_name = "COMMIT")]
        since: Option<String>,
        /// Compare a pandas and native Polars table using this migration contract.
        #[arg(long)]
        contract: Option<PathBuf>,
    },
    /// Plan a pandas-to-Polars or eager-to-lazy Polars rewrite without running Python.
    Map(MapArgs),
    /// Trace a pipeline column or transformation to outputs and callers without running Python.
    Impact(ImpactArgs),
    /// Preview and render editable Manim animations.
    #[command(subcommand)]
    Animate(crate::animate::Animation),
    /// Refit a saved Pipeline on raw training rows and compare its outputs and scores.
    Refit {
        /// The contract JSON that the training script saved beside its model.
        contract: PathBuf,
    },
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
    /// Review the charts of a file against Edward Tufte's book "The Visual Display of
    /// Quantitative Information", in plotly, Dash, matplotlib, seaborn or pandas.
    ///
    /// It finds each chart call of the file and gives it a kind, such as pie, bar or line, and the
    /// plotly call that draws it. It prints the plotly rules that fire on the file, and each
    /// question of the book that applies to a chart it found, with the page of the book, what to
    /// look at and the plotly code to write. Without a path, or for a file that is not Python, it
    /// prints every question. `--run <adapter.py>` draws the figures that the adapter's
    /// `figures()` returns in the project's Python, and prints each measured fact under the
    /// questions it answers. A failed run leaves the questions in place. Exit 0 when it printed a
    /// review, 2 if xpd can't run.
    #[command(after_long_help = "Examples:
  xpd chart-review report.py
  xpd chart-review app.py --run chart_figures.py --format json
  xpd chart-review --questions")]
    ChartReview(crate::chart_review::ChartReviewArgs),
    /// Search code by syntax, in any language ast-grep parses, and preview or apply a rewrite.
    #[command(visible_alias = "query")]
    Run(RunArgs),
    /// Map the files of a project: imports, definitions and their members, with line numbers.
    Outline(OutlineArgs),
    /// List every rule and package check, with its severity, fix and documentation.
    ///
    /// The JSON has one object per rule, with `package`, `rule`, `severity`, `style`,
    /// `severities`, `message`, `fix`, `since`, `url` and `doc`, the Markdown that explains the
    /// rule. `style` is `house` for a rule of the house style, and `severities` holds the
    /// severity without a style setting and the one with `style = "strict"`.
    #[command(after_long_help = "Examples:
  xpd rules
  xpd rules --format json | jq '.[] | select(.package == \"numpy\")'")]
    Rules(RulesOptions),
    /// Compare this project with xpd's profile of Ruff, ty and pytest settings, show the edits, and apply them.
    ///
    /// The profile is one complete target setup that ships with xpd: the Ruff rules, ty rule
    /// levels, pytest settings and pytest plugins every project gets, and the lock files each
    /// kind of project needs. Doctor prints a unified diff of pyproject.toml with the reason for
    /// each change, then the conditional advice for Ruff 0.17.4, ty 0.0.106 and pytest 9.2.0
    /// that the project's code points to. `--check` exits 1 when the project drifts from the
    /// profile, and `--apply` writes the diff, runs uv lock, and records the profile revision in
    /// [tool.xpd.profile]. A project records a deliberate exception there with its reason.
    /// Doctor exits 2 when it cannot read the project, or when `--apply` finds a conflict.
    #[command(after_long_help = "Examples:
  xpd doctor
  xpd doctor --check
  xpd doctor --apply --kind library
  xpd doctor --format json | jq '.profile.changes[] | {item, path, old, new}'")]
    Doctor {
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
        /// Exit 1 when the project drifts from the profile, including a missing or stale lock.
        #[arg(long, conflicts_with = "apply")]
        check: bool,
        /// Write the plan to pyproject.toml, refresh the lock files, and record the profile.
        #[arg(long)]
        apply: bool,
        /// The kind of project: library, application, pipeline or script. The first apply needs
        /// it, and pyproject.toml records it.
        #[arg(long)]
        kind: Option<String>,
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
    /// Generate completion for the installed xpd command tree.
    Completions {
        /// The shell to print completion for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Create a rule project, or a rule, test case or utility in one.
    New(author::NewArgs),
    /// Print every command with its arguments, defaults and help as JSON, for the documentation
    /// site's command reference.
    #[command(hide = true)]
    Commands,
}

/// Runs the command the arguments name.
/// Runs the xpd command line and returns its exit code.
pub fn run() -> ExitCode {
    let default_format = terminal_format();
    match Cli::parse().cmd {
        Cmd::Check(args) => command_result(check(args, default_format)),
        Cmd::Find(args) => unit_result(find(args)),
        Cmd::Api {
            name,
            format,
            index,
            categories,
        } => unit_result((|| {
            let package = name.split_once('.').map(|(package, _)| package);
            let entries = index::load_for(index.as_deref(), package)?;
            let mut entry =
                index::api(&entries, &name)?.with_context(|| format!("no entry named {name}"))?;
            entry.keywords.clear();
            if categories {
                entry.categories = entries.entry_categories(&Categories::current()?, &entry);
            }
            print_entries(&[entry], format, Detail::Full)
        })()),
        Cmd::Index(args) => unit_result(refresh_index(args)),
        Cmd::Hook => crate::hook::run(),
        Cmd::Graph { paths, format } => {
            command_result(graph(&paths, format.unwrap_or(default_format)))
        }
        Cmd::ChartReview(args) => command_result(args.run(default_format)),
        Cmd::Run(args) => command_result(args.run()),
        Cmd::Outline(args) => command_result(args.run()),
        Cmd::Profile { file } => run_py(PROFILE_PY, &[file]),
        Cmd::Refit { contract } => run_py(REFIT_PY, &[contract]),
        Cmd::Equiv {
            scripts,
            since,
            contract,
        } => command_result(equiv(&scripts, since.as_deref(), contract)),
        Cmd::Map(args) => command_result(args.run(default_format)),
        Cmd::Impact(args) => unit_result(args.run(default_format)),
        Cmd::Animate(command) => command_result(command.run()),
        Cmd::Rules(options) => command_result(rules(options, default_format)),
        Cmd::Doctor {
            format,
            check,
            apply,
            kind,
        } => {
            let mode = match (check, apply) {
                (true, _) => crate::doctor::Mode::Check,
                (_, true) => crate::doctor::Mode::Apply,
                _ => crate::doctor::Mode::Report,
            };
            let format = format.unwrap_or(default_format);
            command_result(crate::doctor::run(format, mode, kind.as_deref()))
        }
        Cmd::Skill { json: false } => print_out(SKILL),
        Cmd::Skill { json: true } => print_out(&format!("{}\n", skills::to_json())),
        Cmd::Mcp { args } => mcp::run(&args),
        Cmd::Lsp => lsp::run(),
        Cmd::Test(options) => command_result(crate::cases::run(&options)),
        Cmd::Completions { shell } => run_completions(shell),
        Cmd::New(args) => command_result(author::run(args)),
        Cmd::Commands => print_out(&format!("{}\n", crate::commands::to_json(Cli::command()))),
    }
}

fn find(args: FindArgs) -> Result<()> {
    let FindArgs {
        query,
        package,
        kind,
        category,
        limit,
        format,
        index,
        check,
        write_vectors,
    } = args;
    if let Some(path) = write_vectors {
        return index::write_vectors(&path);
    }
    if let Some(path) = check {
        return check_ranking(index.as_deref(), &path);
    }
    let entries = index::load_for(index.as_deref(), package.as_deref())?;
    let query = query.context("provide a task query or --check")?;
    let policy = category.map(|_| Categories::current()).transpose()?;
    let found = index::find_in_category(
        &entries,
        &query,
        package.as_deref(),
        kind,
        limit,
        policy.as_ref().zip(category),
    );
    print_matches(found, kind, format)
}

fn terminal_format() -> Format {
    if io::stdout().is_terminal() {
        Format::Text
    } else {
        Format::Json
    }
}

fn run_completions(shell: clap_complete::Shell) -> ExitCode {
    let mut command = Cli::command().bin_name("xpd");
    command.build();
    match shell.try_generate(&command, &mut io::stdout().lock()) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        result => unit_result(result.map_err(Into::into)),
    }
}

/// Prints `text` as it is, for a command that cannot fail.
pub(crate) fn print_out(text: &str) -> ExitCode {
    print!("{text}");
    ExitCode::SUCCESS
}

fn unit_result(result: Result<()>) -> ExitCode {
    command_result(result.map(|()| ExitCode::SUCCESS))
}

/// The exit code of a command that chooses its own, or 2 when it fails.
fn command_result(result: Result<ExitCode>) -> ExitCode {
    result.unwrap_or_else(|error| {
        eprintln!("xpd: {}", crate::output::redact(&format!("{error:#}")));
        ExitCode::from(CHECK_FAILED)
    })
}

/// Prints what `find` found, each entry with the first paragraph of its summary and the names
/// of its parameters. `api` prints the members and the parameters as the source writes them.
fn print_matches(mut found: index::Matches, kind: Option<Kind>, format: Format) -> Result<()> {
    for group in [&mut found.symbols, &mut found.rules, &mut found.recipes] {
        for entry in group {
            entry.keywords.clear();
            entry.members.clear();
            entry.parameters_written.clear();
            entry.summary = first_paragraph(&entry.summary);
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
                    print_entries(group, format, Detail::Summary)?;
                }
            }
        }
    }
    Ok(())
}

/// Builds the search index of one library from its source.
fn refresh_index(args: IndexArgs) -> Result<()> {
    let IndexArgs {
        source,
        package,
        version,
        output,
        examples,
        source_url: url,
        license,
        category_report,
    } = args;
    let version = version.map_or_else(|| index::source_version(&source, &package), Ok)?;
    let output = output.unwrap_or_else(|| PathBuf::from(format!("indexes/{package}.json")));
    let categories = Categories::current()?;
    let mut index = index::generate(&source, &package, &version)?;
    index.classify(&categories);
    if let (Some(root), Some(url), Some(license)) =
        (examples.as_deref(), url.as_deref(), license.as_deref())
    {
        let stats = crate::recipe::harvest(
            &crate::recipe::Source { root, url, license },
            &package,
            &mut index.entries,
        )?;
        println!(
            "{package} recipes: {} candidates, {} clean, {} after fixes, {} dropped, {} capped, {} shipped",
            stats.candidates, stats.clean, stats.fixed, stats.dropped, stats.capped, stats.retained
        );
        index.recipes = Some(stats);
    } else {
        let previous = match std::fs::read(&output) {
            Ok(bytes) => Some(
                serde_json::from_slice::<index::Index>(&bytes)
                    .with_context(|| format!("cannot parse existing index {}", output.display()))?,
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("cannot read existing index {}", output.display()));
            }
        };
        if let Some(previous) =
            previous.filter(|previous| previous.package == package && previous.version == version)
        {
            index.entries.extend(
                previous
                    .entries
                    .into_iter()
                    .filter(|entry| entry.kind == index::Kind::Recipe),
            );
            index.recipes = previous.recipes;
        }
    }
    std::fs::write(&output, serde_json::to_vec(&index)?)?;
    if category_report {
        let mut counts = std::collections::BTreeMap::new();
        for (module, classification) in &index.categories.module_categories {
            eprintln!("{module}: {classification}");
            *counts.entry(classification.category).or_insert(0) += 1;
        }
        eprintln!(
            "xpd: {} public modules",
            index.categories.module_categories.len()
        );
        for (category, count) in counts {
            eprintln!("  {category}: {count}");
        }
    }
    println!(
        "{package} {version}: {} entries written to {}",
        index.entries.len(),
        output.display()
    );
    Ok(())
}

fn rules(options: RulesOptions, default: Format) -> Result<ExitCode> {
    let RulesOptions {
        format,
        builtin,
        config,
        rules,
        severity,
        category,
        categories,
    } = options;
    let format = format.unwrap_or(default);
    let filters = Filters { rules, severity };
    // Without a working directory there is no project folder, so only the embedded rules.
    let project = std::env::current_dir().unwrap_or_default();
    let mut catalog = if builtin {
        Catalog::embedded(None, Style::Default)?
    } else {
        let settings = Settings::find(&project)?;
        Catalog::load(
            Catalog::config(&project, config.as_deref())?.as_deref(),
            None,
            settings.style,
        )?
    };
    catalog.select(&filters)?;
    let project = catalog
        .project
        .as_ref()
        .map(|project| project.config.as_path());
    let policy = (categories || category.is_some())
        .then(|| Categories::load(project))
        .transpose()?;
    let entries = catalog.entries();
    let review = crate::chart_review::inventory()?;
    let selected = entries.iter().filter_map(|entry| {
        let labels = policy.as_ref().map(|policy| entry.categories(policy));
        category
            .is_none_or(|wanted| {
                labels
                    .as_ref()
                    .is_some_and(|labels| labels.contains(&wanted))
            })
            .then_some((entry, labels))
    });
    match format {
        Format::Json => {
            let rules = selected
                .map(|(entry, labels)| {
                    let mut value = entry.to_json();
                    if value["package"] == "plotly" {
                        value["questions"] = review_questions(&review, &value["rule"]);
                    }
                    if let Some(labels) = labels {
                        value["categories"] = serde_json::json!(labels);
                    }
                    value
                })
                .collect();
            println!("{}", Value::Array(rules));
        }
        Format::Text => {
            for (entry, labels) in selected {
                let line = entry.line();
                if let Some(labels) = labels {
                    println!("{line} [{}]", category_list(&labels));
                } else {
                    println!("{line}");
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn check(args: CheckArgs, default_format: Format) -> Result<ExitCode> {
    let CheckArgs {
        paths,
        format,
        fixes: FixArgs {
            fix,
            review,
            unsafe_fixes,
        },
        rules,
        severity,
        fail_on,
        config,
        category_summary,
    } = args;
    let format = format.unwrap_or(match default_format {
        Format::Text => CheckFormat::Text,
        Format::Json => CheckFormat::Json,
    });
    let allowed = if unsafe_fixes {
        Applicability::Unsafe
    } else {
        Applicability::Safe
    };
    let fixes = if review {
        FixMode::Review(allowed)
    } else {
        FixMode::Apply(fix.then_some(allowed))
    };
    let workspace = Workspace::open(
        std::env::current_dir()?,
        config.as_deref(),
        &Filters { rules, severity },
    )?;
    let policy = category_summary
        .then(|| {
            Categories::load(
                workspace
                    .catalog
                    .project
                    .as_ref()
                    .map(|project| project.config.as_path()),
            )
        })
        .transpose()?;
    let report = workspace.check(&paths, fixes)?;
    match format {
        CheckFormat::Json => println!("{}", report.to_json(allowed)),
        CheckFormat::Sarif => println!("{}", report.sarif()),
        CheckFormat::Github => {
            for line in report.github() {
                println!("{line}");
            }
        }
        CheckFormat::Text => {
            for i in &report.issues {
                println!("{}", i.line(allowed));
            }
            eprintln!("xpd: {}", report.summary(allowed));
        }
    }
    if let Some(policy) = policy {
        let mut counts = std::collections::BTreeMap::new();
        for issue in &report.issues {
            for category in issue.category.resolve(&policy, &issue.id) {
                *counts.entry(category).or_insert(0) += 1;
            }
        }
        eprintln!(
            "xpd: {} remaining issues by category (categories can overlap)",
            report.issues.len()
        );
        for (category, count) in counts {
            eprintln!("  {category}: {count}");
        }
    }
    if report.skipped > 0 {
        Ok(ExitCode::from(CHECK_FAILED))
    } else if report.has_issues_from(fail_on) {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// The questions of `xpd chart-review` that a plotly rule reports in code, as `xpd rules` and the
/// rule pages show them.
fn review_questions(review: &crate::chart_review::Inventory, rule: &Value) -> Value {
    let questions: Vec<Value> = (review.questions.iter())
        .filter(|question| {
            rule.as_str()
                .is_some_and(|rule| question.rules.iter().any(|named| named == rule))
        })
        .map(|question| {
            serde_json::json!({
                "id": question.id,
                "page": question.page,
                "principle": question.principle,
                "text": question.text,
            })
        })
        .collect();
    Value::Array(questions)
}

/// Prints the dataflow of the files under `paths`.
fn graph(paths: &[PathBuf], format: Format) -> Result<ExitCode> {
    let files = source::python_files(paths)?;
    let graph = Graph::read(Path::new(""), &files);
    match format {
        Format::Json => println!("{}", graph.to_json()),
        Format::Text => {
            println!("{}", graph.to_text());
            eprintln!("xpd: {}", graph.summary());
        }
    }
    if graph.skipped() > 0 {
        Ok(ExitCode::from(CHECK_FAILED))
    } else if graph.has_errors() {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Compares the `result` of two scripts, or of one script and its version in the commit
/// `since`. That version is written beside the script for the run, so it finds the same
/// modules and the same data files.
fn equiv(scripts: &[PathBuf], since: Option<&str>, contract: Option<PathBuf>) -> Result<ExitCode> {
    let (mut paths, committed) = match (scripts, since) {
        ([old, new], None) => (vec![old.clone(), new.clone()], None),
        ([script], Some(commit)) => {
            let committed = Committed::write(script, commit)?;
            (vec![committed.0.clone(), script.clone()], Some(committed))
        }
        ([_], None) => anyhow::bail!(
            "name the script before the change and the script after it, or one script with --since <commit>"
        ),
        (_, Some(_)) => anyhow::bail!("--since takes one script, and git gives the other"),
        (_, None) => unreachable!("clap takes one or two scripts"),
    };
    paths.extend(contract);
    let code = run_py(EQUIV_PY, &paths);
    drop(committed);
    Ok(code)
}

/// The version of a script in a git commit, in a file that is removed when this is dropped.
struct Committed(PathBuf);

impl Committed {
    fn write(script: &Path, commit: &str) -> Result<Self> {
        let name = script
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{} names no file", script.display()))?;
        let folder = match script.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => folder,
            _ => Path::new("."),
        };
        let shown = Command::new("git")
            .arg("-C")
            .arg(folder)
            .args(["show", &format!("{commit}:./{name}")])
            .output()
            .context("cannot start git")?;
        anyhow::ensure!(
            shown.status.success(),
            "git has no {} in {commit}: {}",
            script.display(),
            String::from_utf8_lossy(&shown.stderr).trim()
        );
        let path = folder.join(format!(".xpd-equiv-{}-{name}", std::process::id()));
        fs::write(&path, shown.stdout)
            .with_context(|| format!("cannot write {}", path.display()))?;
        Ok(Self(path))
    }
}

/// Removes the file. It may be gone already, and the comparison is printed either way.
impl Drop for Committed {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
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

#[derive(Clone, Copy)]
enum Detail {
    Summary,
    Full,
}

/// The first paragraph of a summary, which is what `find` and a short `api` listing show.
fn first_paragraph(summary: &str) -> String {
    summary.split("\n\n").next().unwrap_or_default().to_owned()
}

fn category_list(categories: &std::collections::BTreeSet<Category>) -> String {
    categories
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_entries(entries: &[index::Entry], format: Format, detail: Detail) -> Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string(entries)?),
        Format::Text => {
            for entry in entries {
                let signature = if entry.kind != Kind::Symbol
                    || entry.callable == Some(index::CallableKind::Property)
                {
                    String::new()
                } else {
                    let params = match detail {
                        Detail::Summary if entry.parameters.join(", ").len() > 70 => format!(
                            "{}, ...",
                            entry
                                .parameters
                                .iter()
                                .take(4)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        // A function that only overloads declare has no written list.
                        Detail::Full if !entry.parameters_written.is_empty() => {
                            entry.parameters_written.join(", ")
                        }
                        Detail::Summary | Detail::Full => entry.parameters.join(", "),
                    };
                    format!("({params})")
                };
                println!(
                    "{}{signature} [{}]\n{}",
                    entry.name,
                    entry.callable.map_or(
                        if entry.kind == Kind::Recipe {
                            "recipe"
                        } else {
                            "rule"
                        },
                        index::CallableKind::as_str,
                    ),
                    entry.summary
                );
                if !entry.categories.is_empty() {
                    println!("Categories: {}", category_list(&entry.categories));
                }
                if matches!(detail, Detail::Full)
                    && let Some(recipe) = &entry.recipe
                {
                    if recipe.is_playbook() {
                        println!("Source: {}\nLicense: {}", recipe.source, recipe.license);
                    } else {
                        println!(
                            "Source: {}\nSymbols: {}\nLicense: {}\nModified by xpd fixes: {}",
                            recipe.source,
                            recipe.symbols.join(", "),
                            recipe.license,
                            recipe.modified
                        );
                    }
                    if !recipe.attribution.is_empty() {
                        println!("{}", recipe.attribution);
                    }
                    if let Some(code) = &recipe.code {
                        println!("\n```python\n{code}```");
                    }
                    if recipe.is_playbook() {
                        println!("\nCommands, in order:");
                        for (nth, command) in recipe.commands.iter().enumerate() {
                            println!("{}. {command}", nth + 1);
                        }
                    }
                }
                for member in &entry.members {
                    // A property is read and not called, so it has no parameter list.
                    if member.property {
                        println!("  {} {}", member.name, member.summary);
                    } else {
                        println!(
                            "  {}({}) {}",
                            member.name,
                            member.parameters.join(", "),
                            member.summary
                        );
                    }
                }
                if !entry.rules.is_empty() {
                    println!("Rules: {}", entry.rules.join(", "));
                }
            }
        }
    }
    Ok(())
}

/// Prints the issues that the code of each playbook has under the strict style, and gives the
/// number of playbooks with an issue. A playbook states the house style, so its own code must
/// have none.
fn unclean_playbooks() -> Result<usize> {
    let mut unclean = 0;
    let playbooks = index::playbooks()?;
    for playbook in &playbooks {
        let issues = match &playbook.recipe {
            Some(recipe) => crate::recipe::playbook_issues(recipe)?,
            None => Vec::new(),
        };
        if !issues.is_empty() {
            unclean += 1;
            println!(
                "FAIL: the code of {} has issues under the strict style",
                playbook.name
            );
        }
        for issue in issues {
            println!("  {issue}");
        }
    }
    println!(
        "{} of {} playbooks have no issue under the strict style",
        playbooks.len() - unclean,
        playbooks.len()
    );
    Ok(unclean)
}

/// Asks each question of `path` and fails when its expected entry misses the top five. It also
/// fails when the code of a playbook has an issue under the strict style. A
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
    let has_model = with_model.ensure_model();
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
    let unclean = unclean_playbooks()?;
    println!("{} open questions are recorded and not checked", open.len());
    let missing = index::records_without_vector();
    if let Some(first) = missing.first() {
        println!(
            "{} records have no vector of their current text in indexes/vectors.bin, such as {first}. Write it again with xpd find --write-vectors indexes/vectors.bin",
            missing.len()
        );
    }
    let failed = failed[0] + failed[1];
    anyhow::ensure!(failed == 0, "{failed} discovery questions failed");
    anyhow::ensure!(unclean == 0, "{unclean} playbooks have issues");
    anyhow::ensure!(
        missing.is_empty(),
        "{} records have no vector",
        missing.len()
    );
    Ok(())
}
