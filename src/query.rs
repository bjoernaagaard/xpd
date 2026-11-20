//! `xpd run`: one structural search, with an optional rewrite, on ast-grep's own run command.
//!
//! The flags are declared here, so `xpd run --help` and the shell completions show them, and
//! clap checks them before ast-grep reads them again through `ast_grep::main_with_args`.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{ArgGroup, ValueEnum};

use crate::output::Format;

/// How JSON is printed: one array over several lines, one array on one line, or one object per
/// line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum JsonStyle {
    Pretty,
    Compact,
    Stream,
}

/// An ignore file that a walk disregards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum NoIgnore {
    /// Read hidden files and directories.
    Hidden,
    /// Disregard .ignore files.
    Dot,
    /// Disregard .git/info/exclude.
    Exclude,
    /// Disregard the global git ignore file.
    Global,
    /// Disregard ignore files in parent directories.
    Parent,
    /// Disregard .gitignore files.
    Vcs,
}

/// How exactly a pattern must match the code.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Strictness {
    /// Every node, including punctuation.
    Cst,
    /// Every node except trivial ones (the default).
    Smart,
    /// Named nodes only.
    Ast,
    /// Named nodes, without comments.
    Relaxed,
    /// Named nodes without comments, and without comparing their text.
    Signature,
    /// The text of the nodes, whatever their kinds.
    Template,
}

/// What `--debug-query` prints.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum DebugQuery {
    Pattern,
    Ast,
    Cst,
    Sexp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Ansi,
    Never,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Heading {
    Auto,
    Always,
    Never,
}

/// Search code by syntax, in any language ast-grep parses, and preview or apply a rewrite.
#[derive(Debug, clap::Args)]
#[command(
    group(ArgGroup::new("matcher").required(true).args(["pattern", "kind"])),
    after_long_help = "Exit 0 when something matched, 1 when nothing did, 2 if xpd can't run.

Examples:
  xpd run -p 'pd.concat($$$FRAMES)' src/
  xpd run -k call_expression -l ts web/
  xpd run -p '$S.tz_convert(\"UTC\").tz_localize(None)' -r '$S.tz_convert(None)' analysis.py
  xpd run -p 'np.matrix($A)' -r 'np.asarray($A)' -U src/
  cat job.py | xpd run -p 'print($$$A)' -l python --stdin --json"
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an on-off flag of the command line"
)]
pub(crate) struct RunArgs {
    /// The pattern to match, as code with metavariables such as $X and $$$ARGS.
    #[arg(short, long)]
    pattern: Option<String>,
    /// The kind of the part of the pattern that is the actual match.
    #[arg(long, value_name = "KIND", requires = "pattern")]
    selector: Option<String>,
    /// How exactly the pattern must match.
    #[arg(long, value_enum, requires = "pattern")]
    strictness: Option<Strictness>,
    /// The syntax kind to match, or an ESQuery-style selector such as `call > identifier`.
    #[arg(short, long, conflicts_with = "pattern")]
    kind: Option<String>,
    /// The code that replaces each match. Metavariables of the pattern are filled in.
    #[arg(short, long, value_name = "FIX")]
    rewrite: Option<String>,
    /// The language of the pattern. Without it, each file's extension names its language.
    #[arg(short, long, value_parser = languages(), ignore_case = true)]
    lang: Option<SupportLang>,
    /// Print the syntax tree of the pattern on stderr.
    #[arg(
        long,
        value_enum,
        requires = "lang",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "pattern"
    )]
    debug_query: Option<DebugQuery>,
    /// Files or directories (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// Follow symbolic links.
    #[arg(long)]
    follow: bool,
    /// Ignore files to disregard. Repeat the flag for more than one.
    #[arg(long, value_enum, value_name = "FILE_TYPE")]
    no_ignore: Vec<NoIgnore>,
    /// Read the code from stdin, in the language of --lang.
    #[arg(long, requires = "lang")]
    stdin: bool,
    /// Include or exclude paths that match a glob, as in .gitignore. `!` excludes.
    #[arg(long)]
    globs: Vec<String>,
    /// Threads to use. 0 lets ast-grep choose.
    #[arg(short = 'j', long, default_value_t = 0, value_name = "NUM")]
    threads: usize,
    /// Confirm each rewrite, or edit the match in your editor.
    #[arg(short, long)]
    interactive: bool,
    /// Apply every rewrite without asking.
    #[arg(short = 'U', long, requires = "rewrite")]
    update_all: bool,
    /// Print only the paths of files that match.
    #[arg(long, conflicts_with_all = ["interactive", "update_all", "json", "format"])]
    files_with_matches: bool,
    /// Print the matches, and each rewrite as a preview, as JSON (pretty unless a style is
    /// given). JSON never writes files, so it conflicts with --update-all.
    #[arg(
        long,
        value_enum,
        value_name = "STYLE",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "pretty",
        conflicts_with_all = ["interactive", "update_all", "format"]
    )]
    json: Option<JsonStyle>,
    /// Output format. Default: text on a terminal, json otherwise. JSON is one line.
    #[arg(long, value_enum)]
    format: Option<Format>,
    /// When to color the text output.
    #[arg(long, value_enum, default_value = "auto", value_name = "WHEN")]
    color: Color,
    /// When to print the file name above its matches instead of before each one.
    #[arg(long, value_enum, default_value = "auto", value_name = "WHEN")]
    heading: Heading,
    /// Lines to show after each match.
    #[arg(
        short = 'A',
        long,
        default_value_t = 0,
        value_name = "NUM",
        conflicts_with = "context"
    )]
    after: u16,
    /// Lines to show before each match.
    #[arg(
        short = 'B',
        long,
        default_value_t = 0,
        value_name = "NUM",
        conflicts_with = "context"
    )]
    before: u16,
    /// Lines to show around each match.
    #[arg(short = 'C', long, default_value_t = 0, value_name = "NUM")]
    context: u16,
    /// The ast-grep project file, whose utility rules and custom languages the pattern can use.
    /// Default: the sgconfig.yml in this folder or the nearest one above it.
    #[arg(short, long, value_name = "CONFIG_FILE")]
    config: Option<PathBuf>,
}

impl RunArgs {
    /// Runs the search through ast-grep, which prints the matches.
    pub(crate) fn run(self) -> Result<ExitCode> {
        anyhow::ensure!(
            !(matches!(self.format, Some(Format::Json)) && (self.interactive || self.update_all)),
            "--format json never writes files, so it cannot be combined with --interactive or \
             --update-all"
        );
        // ast-grep takes its arguments as text and exits with 1 for a path it cannot open,
        // which is the code of a search that matched nothing.
        if !self.stdin {
            for path in &self.paths {
                anyhow::ensure!(
                    path.to_str().is_some(),
                    "{} is not a UTF-8 path, which xpd run cannot search",
                    path.display()
                );
                anyhow::ensure!(path.exists(), "{} does not exist", path.display());
            }
        }
        ast_grep::main_with_args(self.into_args().into_iter())
    }

    /// The arguments of `ast-grep run` that this search is.
    fn into_args(self) -> Vec<String> {
        let json = match (self.json, self.format) {
            (Some(style), _) => Some(style),
            (None, Some(Format::Json)) => Some(JsonStyle::Compact),
            (None, Some(Format::Text)) => None,
            (None, None) => {
                let wants_text = self.interactive
                    || self.update_all
                    || self.files_with_matches
                    || std::io::stdout().is_terminal();
                (!wants_text).then_some(JsonStyle::Compact)
            }
        };
        let mut args = vec!["xpd".to_owned(), "run".to_owned()];
        let mut option = |flag: &str, value: Option<String>| {
            if let Some(value) = value {
                args.push(format!("--{flag}={value}"));
            }
        };
        option("pattern", self.pattern);
        option("selector", self.selector);
        option("strictness", self.strictness.as_ref().map(name));
        option("kind", self.kind);
        option("rewrite", self.rewrite);
        option("lang", self.lang.map(|lang| lang.aliases()[0].to_owned()));
        option("debug-query", self.debug_query.as_ref().map(name));
        option("json", json.as_ref().map(name));
        // ast-grep sees a flag with its default value as given, and rejects --after with
        // --context, so a default is left out.
        option("color", Some(name(&self.color)).filter(|c| c != "auto"));
        option("heading", Some(name(&self.heading)).filter(|h| h != "auto"));
        for (flag, count) in [
            ("threads", self.threads),
            ("after", self.after.into()),
            ("before", self.before.into()),
            ("context", self.context.into()),
        ] {
            option(flag, (count > 0).then(|| count.to_string()));
        }
        option("config", self.config.map(|path| path.display().to_string()));
        for ignored in self.no_ignore {
            option("no-ignore", Some(name(&ignored)));
        }
        for glob in self.globs {
            option("globs", Some(glob));
        }
        for (flag, set) in [
            ("--follow", self.follow),
            ("--stdin", self.stdin),
            ("--interactive", self.interactive),
            ("--update-all", self.update_all),
            ("--files-with-matches", self.files_with_matches),
        ] {
            if set {
                args.push(flag.to_owned());
            }
        }
        // The paths go last, after `--`, so that a path that starts with a dash is no flag.
        if !self.stdin {
            args.push("--".to_owned());
            args.extend(self.paths.iter().map(|path| path.display().to_string()));
        }
        args
    }
}

/// The name of a value as the command line writes it.
fn name(value: &impl ValueEnum) -> String {
    value
        .to_possible_value()
        .expect("every value has a name")
        .get_name()
        .to_owned()
}

/// The language names and aliases that ast-grep reads, such as `python`, `py` and `rs`, as the
/// values of a flag, so that the shell completes them.
pub(crate) fn languages() -> impl TypedValueParser<Value = SupportLang> {
    let names = (SupportLang::all_langs().iter()).flat_map(|lang| lang.aliases().iter().copied());
    PossibleValuesParser::new(names)
        .map(|name| name.parse().expect("each alias names its language"))
}
