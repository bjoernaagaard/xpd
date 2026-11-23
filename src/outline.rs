//! Outlines: the imports, definitions and direct members of source files, with their ranges.
//!
//! The extraction is ast-grep's own: `CombinedExtractors` runs rules in the schema that
//! `ast-grep outline --outline-rules` reads. Python uses xpd's rules in `outline/python.yml`,
//! which keep whole parameter lists, decorators and private names. Every other language uses the
//! rules that ship with ast-grep. `xpd graph` reads Hamilton nodes and pipeline steps from the
//! Python items, and finds the syntax node of an item by its range to read what its decorators
//! and body mean.

use std::collections::HashMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use ast_grep_config::GlobalRules;
use ast_grep_core::Language;
use ast_grep_core::tree_sitter::LanguageExt;
use ast_grep_language::SupportLang;
use ast_grep_outline::DEFAULT_OUTLINE_RULES;
use ast_grep_outline::combined_extractor::CombinedExtractors;
use ast_grep_outline::extractor::{SerializableOutlineRule, parse_outline_rules};
use ast_grep_outline::model::{OutlineEntry, OutlineItem, OutlineMember, SymbolType};
use ast_grep_outline::options::{
    OutlineEntryDetail, OutlineExtractorOptions, OutlineFlagFilter, OutlineMemberOptions,
};
use clap::ValueEnum;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::project::{self, OutlineFolders};
use crate::query::{JsonStyle, NoIgnore, languages};
use crate::syntax::{self, Node, Root};

/// The Python rules of `xpd outline` and `xpd graph`.
const PYTHON_RULES: &str = include_str!("outline/python.yml");
/// Every function and method of a Python file with its parameters, for `xpd index`.
const PYTHON_CALLABLE_RULES: &str = include_str!("outline/python-callables.yml");

type Rules = Vec<SerializableOutlineRule<SupportLang>>;

/// Compiled outline rules, by language.
struct Extractors(HashMap<SupportLang, CombinedExtractors<SupportLang>>);

impl Extractors {
    fn compile(rules: Rules, options: &OutlineExtractorOptions) -> Result<Self> {
        let mut by_language: HashMap<SupportLang, Rules> = HashMap::new();
        for rule in rules {
            by_language
                .entry(rule.common().language)
                .or_default()
                .push(rule);
        }
        let globals = GlobalRules::default();
        by_language
            .into_iter()
            .map(|(language, rules)| {
                let compiled = CombinedExtractors::try_from_rules(rules, options.clone(), &globals)
                    .with_context(|| format!("invalid {language} outline rules"))?;
                Ok((language, compiled))
            })
            .collect::<Result<_>>()
            .map(Self)
    }

    fn reads(&self, language: SupportLang) -> bool {
        self.0.contains_key(&language)
    }

    /// The items of a parsed file and of the code of other languages that it embeds.
    fn items(&self, language: SupportLang, root: &Root) -> Vec<OutlineItem<'static>> {
        let mut items: Vec<OutlineItem<'static>> = (self.0.get(&language).into_iter())
            .flat_map(|extractors| extractors.extract(root.root()).map(owned))
            .collect();
        let embedded = root.get_injections(|name| name.parse().ok().filter(|l| self.reads(*l)));
        for code in &embedded {
            if let Some(extractors) = self.0.get(code.lang()) {
                items.extend(extractors.extract(code.root()).map(owned));
            }
        }
        items.sort_by_key(|item| item.entry.range.byte_offset.start);
        items
    }
}

fn owned(item: OutlineItem<'_>) -> OutlineItem<'static> {
    let entry = |entry: OutlineEntry<'_>| OutlineEntry {
        name: entry.name.into_owned().into(),
        signature: entry.signature.into_owned().into(),
        ast_kind: entry.ast_kind.into_owned().into(),
        ..entry
    };
    OutlineItem {
        entry: entry(item.entry),
        is_import: item.is_import,
        is_exported: item.is_exported,
        members: (item.members.into_iter())
            .map(|member| OutlineMember {
                entry: entry(member.entry),
                is_public: member.is_public,
            })
            .collect(),
    }
}

/// The rules that ship with xpd: ast-grep's for every language but Python, and xpd's for Python.
fn bundled_rules() -> Result<Rules> {
    let mut rules: Rules = parse_outline_rules(DEFAULT_OUTLINE_RULES)?;
    rules.retain(|rule| rule.common().language != SupportLang::Python);
    rules.extend(parse_outline_rules(PYTHON_RULES)?);
    Ok(rules)
}

/// Rules that ship with xpd, compiled once for every file a run reads.
fn compiled(
    rules: &'static str,
    cell: &'static OnceLock<CombinedExtractors<SupportLang>>,
) -> &'static CombinedExtractors<SupportLang> {
    cell.get_or_init(|| {
        let rules = parse_outline_rules(rules).expect("the bundled outline rules parse");
        CombinedExtractors::try_from(rules, &GlobalRules::default())
            .expect("the bundled outline rules compile")
    })
}

fn python_rules() -> &'static CombinedExtractors<SupportLang> {
    static RULES: OnceLock<CombinedExtractors<SupportLang>> = OnceLock::new();
    compiled(PYTHON_RULES, &RULES)
}

/// The syntax node that an outline entry was extracted from, found by its range below `node`.
fn node_at<'r>(mut node: Node<'r>, entry: &OutlineEntry<'_>) -> Node<'r> {
    let range = &entry.range.byte_offset;
    loop {
        if node.range() == *range && node.kind() == entry.ast_kind {
            return node;
        }
        let inner = node
            .children()
            .find(|child| child.range().start <= range.start && range.end <= child.range().end)
            .expect("an outline entry is a node of the tree it was extracted from");
        node = inner;
    }
}

/// One parameter of a Python function. `*args` and `**kwargs` are left out.
#[derive(Debug)]
pub(crate) struct Parameter {
    pub(crate) name: String,
    pub(crate) hinted: bool,
    /// The parameter has a default.
    pub(crate) optional: bool,
}

impl Parameter {
    fn new(member: &OutlineMember<'_>) -> Option<Self> {
        let (hinted, optional) = match member.entry.ast_kind.as_ref() {
            "identifier" => (false, false),
            "typed_parameter" => (true, false),
            "default_parameter" => (false, true),
            "typed_default_parameter" => (true, true),
            _ => return None,
        };
        let name = member.entry.name.as_ref();
        (!name.starts_with('*')).then(|| Self {
            name: name.to_owned(),
            hinted,
            optional,
        })
    }
}

/// One decorator of a Python definition.
pub(crate) struct Decorator<'r> {
    /// The dotted name, such as `config.when`.
    pub(crate) path: String,
    pub(crate) node: Node<'r>,
}

/// A function at the top of a Python module.
pub(crate) struct Function<'r> {
    pub(crate) name: String,
    /// The name has no leading underscore.
    pub(crate) exported: bool,
    /// The `def`, without its decorators.
    pub(crate) definition: Node<'r>,
    pub(crate) decorators: Vec<Decorator<'r>>,
    pub(crate) parameters: Vec<Parameter>,
}

/// The functions and the `name = value` statements at the top of a Python module, in the order
/// of the file.
#[derive(Default)]
pub(crate) struct Module<'r> {
    pub(crate) functions: Vec<Function<'r>>,
    /// Each assigned name with its value.
    pub(crate) assignments: Vec<(String, Node<'r>)>,
}

/// What the top of the Python module in `root` defines.
pub(crate) fn python_module(root: &Root) -> Module<'_> {
    let mut module = Module::default();
    for item in python_rules().extract(root.root()) {
        let node = node_at(root.root(), &item.entry);
        match item.entry.symbol_type {
            SymbolType::Function => {
                let mut decorators = Vec::new();
                let mut parameters = Vec::new();
                for member in &item.members {
                    if member.entry.ast_kind == "decorator" {
                        decorators.push(Decorator {
                            path: member.entry.name.to_string(),
                            node: node_at(node.clone(), &member.entry),
                        });
                    } else {
                        parameters.extend(Parameter::new(member));
                    }
                }
                module.functions.push(Function {
                    name: item.entry.name.into_owned(),
                    exported: item.is_exported,
                    definition: node.field("definition").unwrap_or(node),
                    decorators,
                    parameters,
                });
            }
            SymbolType::Constant | SymbolType::Variable => {
                if let Some(value) = node.field("right") {
                    module
                        .assignments
                        .push((item.entry.name.into_owned(), value));
                }
            }
            _ => {}
        }
    }
    module
}

/// One part of the parameter list of a Python function: a parameter, or a `*` or `/` that
/// stands alone between parameters.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ParameterText {
    /// The name of a parameter, with `args` for `*args` and `kwargs` for `**kwargs`. `None` for
    /// a `*` or `/` that stands alone.
    pub(crate) name: Option<String>,
    /// The part as the source writes it, with its type annotation and its default, on one line.
    pub(crate) written: String,
}

/// A function or method of a Python file.
#[derive(Debug)]
pub(crate) struct Callable {
    pub(crate) name: String,
    /// The line of its first decorator, or of its `def`, counted from 1.
    pub(crate) line: usize,
    /// Its parameter list, in the order of the source.
    pub(crate) parameters: Vec<ParameterText>,
}

/// Every function and method of the Python source `src`.
pub(crate) fn python_callables(src: &str) -> Vec<Callable> {
    static RULES: OnceLock<CombinedExtractors<SupportLang>> = OnceLock::new();
    let root = syntax::parse(src);
    compiled(PYTHON_CALLABLE_RULES, &RULES)
        .extract(root.root())
        .map(|item| {
            let definition = node_at(root.root(), &item.entry);
            Callable {
                line: item.entry.range.start.line + 1,
                parameters: (item.members.iter())
                    .map(|member| {
                        let stands_alone = matches!(
                            member.entry.ast_kind.as_ref(),
                            "keyword_separator" | "positional_separator"
                        );
                        ParameterText {
                            name: (!stands_alone).then(|| member.entry.name.to_string()),
                            written: one_line(src, &node_at(definition.clone(), &member.entry)),
                        }
                    })
                    .collect(),
                name: item.entry.name.into_owned(),
            }
        })
        .collect()
}

/// The source of `node` on one line, without comments. A line break becomes a space, or nothing
/// when it follows an opening bracket or comes before a closing one. The comma before a closing
/// bracket on the next line goes too, unless it is what makes a tuple of one item.
fn one_line(src: &str, node: &Node<'_>) -> String {
    let mut tokens = Vec::new();
    collect_tokens(node, &mut tokens);
    let mut text = String::new();
    let mut previous: Option<&Node<'_>> = None;
    for token in &tokens {
        if let Some(previous) = previous {
            let between = &src[previous.range().end..token.range().start];
            if !between.contains('\n') {
                text.push_str(between);
            } else if matches!(token.kind().as_ref(), ")" | "]" | "}") {
                if previous.kind() == "," && !makes_tuple(previous) {
                    text.pop();
                }
            } else if !matches!(previous.kind().as_ref(), "(" | "[" | "{") {
                text.push(' ');
            }
        }
        if token.kind() == "string" {
            push_string(src, token, &mut text);
        } else {
            text.push_str(&src[token.range()]);
        }
        previous = Some(token);
    }
    text
}

/// Whether `comma` is the only comma of a tuple or of a subscript, as in `(1,)` and `a[1,]`,
/// which mean something else without it.
fn makes_tuple(comma: &Node<'_>) -> bool {
    comma.parent().is_some_and(|parent| {
        matches!(parent.kind().as_ref(), "tuple" | "subscript")
            && parent.children().filter(|c| c.kind() == ",").count() == 1
    })
}

/// Adds the string `node` on one line. A line break in the string becomes `\n`, and a
/// backslash that continues the string on the next line goes with its line break.
fn push_string(src: &str, node: &Node<'_>, text: &mut String) {
    let mut push = |part: &str| text.push_str(&part.replace("\r\n", "\\n").replace('\n', "\\n"));
    let mut start = node.range().start;
    for escape in node.dfs().filter(|n| n.kind() == "escape_sequence") {
        if src[escape.range()].ends_with('\n') {
            push(&src[start..escape.range().start]);
            start = escape.range().end;
        }
    }
    push(&src[start..node.range().end]);
}

/// The tokens below `node`, in the order of the source. A string is one token, and a comment
/// and a backslash that continues a line are none.
fn collect_tokens<'r>(node: &Node<'r>, tokens: &mut Vec<Node<'r>>) {
    match node.kind().as_ref() {
        "comment" | "line_continuation" => {}
        kind if kind != "string" && node.children().len() > 0 => {
            for child in node.children() {
                collect_tokens(&child, tokens);
            }
        }
        _ => tokens.push(node.clone()),
    }
}

/// Which top-level items `xpd outline` lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Items {
    /// `structure` for files and stdin, `exports` when a directory is given.
    Auto,
    /// What each file defines, without its imports.
    Structure,
    /// What each file exports: public names in Python.
    Exports,
    /// What each file imports.
    Imports,
    All,
}

/// How `xpd outline` prints the items as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum View {
    /// `digest` for files and stdin, `names` when a directory is given.
    Auto,
    /// The names of each file, one line per kind of symbol.
    Names,
    /// One line per item, with its line number and signature.
    Signatures,
    /// The signatures, each with the names of its members.
    Digest,
    /// The signatures, each with one line per member.
    Expanded,
}

/// The symbol types of the outline schema, as `--type` takes them.
const SYMBOL_TYPES: &[&str] = &[
    "file",
    "module",
    "namespace",
    "package",
    "class",
    "method",
    "property",
    "field",
    "constructor",
    "enum",
    "interface",
    "function",
    "variable",
    "constant",
    "string",
    "number",
    "boolean",
    "array",
    "object",
    "key",
    "null",
    "enumMember",
    "struct",
    "event",
    "operator",
    "typeParameter",
];

fn symbol_types() -> impl TypedValueParser<Value = SymbolType> {
    PossibleValuesParser::new(SYMBOL_TYPES).map(|name| {
        serde_json::from_value(serde_json::Value::String(name))
            .expect("a symbol type of the schema")
    })
}

/// The name of a symbol type in the outline schema, such as `function` or `enumMember`.
fn type_name(symbol_type: SymbolType) -> String {
    match serde_json::to_value(symbol_type) {
        Ok(serde_json::Value::String(name)) => name,
        _ => unreachable!("a symbol type serializes as its name"),
    }
}

/// Map the files of a project: imports, definitions and their members, with line numbers.
///
/// Reads every language that ast-grep parses, without running anything. Python gets whole
/// signatures, decorators and private names. An sgconfig.yml in this folder or above adds the
/// rule files that its `outlineRules` names and the .yml files in its `outlineDirs`.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd outline features.py
  xpd outline src/ --view digest
  xpd outline . --items imports --format json
  xpd outline src/ --type class,function --match '^Frame'
  cat job.py | xpd outline --stdin --lang python")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an on-off flag of the command line"
)]
pub(crate) struct OutlineArgs {
    /// Files or directories (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// The language to read. With paths, only files of this language are read.
    #[arg(short, long, value_parser = languages(), ignore_case = true, required_if_eq("stdin", "true"))]
    lang: Option<SupportLang>,
    /// Read the code from stdin, in the language of --lang.
    #[arg(long)]
    stdin: bool,
    /// Output format. Default: text on a terminal, json otherwise. JSON is one line.
    #[arg(long, value_enum)]
    format: Option<Format>,
    /// Print JSON (pretty unless a style is given).
    #[arg(
        long,
        value_enum,
        value_name = "STYLE",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "pretty",
        conflicts_with = "format"
    )]
    json: Option<JsonStyle>,
    /// Which top-level items to list.
    #[arg(long, value_enum, default_value = "auto")]
    items: Items,
    /// Keep the items of these symbol types, separated by commas.
    #[arg(long = "type", value_name = "TYPE", value_delimiter = ',', value_parser = symbol_types())]
    types: Vec<SymbolType>,
    /// Keep the items whose name or signature matches this regular expression.
    #[arg(long = "match", value_name = "REGEX", value_parser = Regex::new)]
    matching: Option<Regex>,
    /// List only public members.
    #[arg(long)]
    pub_members: bool,
    /// How to print the items as text.
    #[arg(long, value_enum, default_value = "auto")]
    view: View,
    /// More outline rule files, in the schema of `ast-grep outline --outline-rules`.
    #[arg(long, value_name = "FILE")]
    outline_rules: Vec<PathBuf>,
    /// Do not use the rules that ship with xpd.
    #[arg(long)]
    no_default_outline_rules: bool,
    /// Follow symbolic links.
    #[arg(long)]
    follow: bool,
    /// Ignore files to disregard. Repeat the flag for more than one.
    #[arg(long, value_enum, value_name = "FILE_TYPE")]
    no_ignore: Vec<NoIgnore>,
    /// Include or exclude paths that match a glob, as in .gitignore. `!` excludes.
    #[arg(long)]
    globs: Vec<String>,
}

/// The outline of one file, in the JSON shape of `ast-grep outline`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct File {
    path: String,
    language: String,
    items: Vec<OutlineItem<'static>>,
}

impl OutlineArgs {
    /// Prints the outline of the files.
    pub(crate) fn run(self) -> Result<ExitCode> {
        let directories = self.reads_directories();
        let items = match self.items {
            Items::Auto if directories => Items::Exports,
            Items::Auto => Items::Structure,
            other => other,
        };
        let view = match self.view {
            View::Auto if directories => View::Names,
            View::Auto => View::Digest,
            other => other,
        };
        let json = match (self.json, self.format) {
            (Some(style), _) => Some(style),
            (None, Some(Format::Json)) => Some(JsonStyle::Compact),
            (None, Some(Format::Text)) => None,
            (None, None) => (!io::stdout().is_terminal()).then_some(JsonStyle::Compact),
        };
        let detail = if view == View::Names && self.matching.is_none() {
            OutlineEntryDetail::Name
        } else {
            OutlineEntryDetail::Signature
        };
        let options = OutlineExtractorOptions {
            symbol_types: (!self.types.is_empty()).then(|| self.types.clone()),
            item_regex: self.matching.clone(),
            imports: match items {
                Items::Structure => OutlineFlagFilter::No,
                Items::Imports => OutlineFlagFilter::Yes,
                _ => OutlineFlagFilter::Any,
            },
            exported: if items == Items::Exports {
                OutlineFlagFilter::Yes
            } else {
                OutlineFlagFilter::Any
            },
            detail,
            members: (json.is_some() || matches!(view, View::Digest | View::Expanded)).then(|| {
                OutlineMemberOptions {
                    public: if self.pub_members {
                        OutlineFlagFilter::Yes
                    } else {
                        OutlineFlagFilter::Any
                    },
                    detail: if json.is_some() || view == View::Expanded {
                        OutlineEntryDetail::Signature
                    } else {
                        OutlineEntryDetail::Name
                    },
                }
            }),
        };
        let mut rules = if self.no_default_outline_rules {
            Vec::new()
        } else {
            bundled_rules().context("invalid bundled outline rules")?
        };
        for path in project_rule_files()?.iter().chain(&self.outline_rules) {
            let source = fs::read_to_string(path)
                .with_context(|| format!("cannot read {}", path.display()))?;
            rules.extend(
                parse_outline_rules(&source)
                    .with_context(|| format!("invalid outline rules in {}", path.display()))?,
            );
        }
        let extractors = Extractors::compile(rules, &options)?;
        let (files, read_failed) = if self.stdin {
            let language = self.lang.expect("clap requires --lang with --stdin");
            let source = io::read_to_string(io::stdin()).context("cannot read stdin")?;
            let root = language.ast_grep(source);
            (
                vec![File {
                    path: "STDIN".to_owned(),
                    language: language.to_string(),
                    items: extractors.items(language, &root),
                }],
                false,
            )
        } else {
            self.read_files(&extractors)?
        };
        let mut out = io::stdout().lock();
        match json {
            Some(JsonStyle::Pretty) => serde_json::to_writer_pretty(&mut out, &files)?,
            Some(JsonStyle::Compact) => serde_json::to_writer(&mut out, &files)?,
            Some(JsonStyle::Stream) => {
                for file in &files {
                    serde_json::to_writer(&mut out, file)?;
                    writeln!(out)?;
                }
            }
            None => write_text(&mut out, &files, view)?,
        }
        if matches!(json, Some(JsonStyle::Pretty | JsonStyle::Compact)) {
            writeln!(out)?;
        }
        Ok(if read_failed {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        })
    }

    /// Whether a directory is among the paths, which changes what `auto` lists and shows.
    fn reads_directories(&self) -> bool {
        !self.stdin && self.paths.iter().any(|path| path.is_dir())
    }

    /// The outlines of the files under the paths, in path order. A file with no items is left
    /// out when a directory was given.
    fn read_files(&self, extractors: &Extractors) -> Result<(Vec<File>, bool)> {
        let directories = self.reads_directories();
        let read = |(path, language): &(PathBuf, SupportLang)| -> Result<File> {
            let source = fs::read_to_string(path)
                .with_context(|| format!("cannot read {}", path.display()))?;
            let root = language.ast_grep(source);
            Ok(File {
                path: path.display().to_string(),
                language: language.to_string(),
                items: extractors.items(*language, &root),
            })
        };
        let files: Vec<Result<File>> = self.files(extractors)?.par_iter().map(read).collect();
        let mut read_files = Vec::new();
        let mut read_failed = false;
        for file in files {
            match file {
                Ok(file) if directories && file.items.is_empty() => {}
                Ok(file) => read_files.push(file),
                Err(e) => {
                    eprintln!("xpd: {e:#}");
                    read_failed = true;
                }
            }
        }
        Ok((read_files, read_failed))
    }

    /// The files under the paths that a rule reads, each with its language.
    fn files(&self, extractors: &Extractors) -> Result<Vec<(PathBuf, SupportLang)>> {
        let ignores = |kind: NoIgnore| !self.no_ignore.contains(&kind);
        let mut globs = ignore::overrides::OverrideBuilder::new(std::env::current_dir()?);
        for glob in &self.globs {
            globs.add(glob)?;
        }
        let (first, rest) = self.paths.split_first().expect("clap gives a default path");
        let mut walk = ignore::WalkBuilder::new(first);
        for path in rest {
            walk.add(path);
        }
        walk.follow_links(self.follow)
            .overrides(globs.build()?)
            .hidden(ignores(NoIgnore::Hidden))
            .parents(ignores(NoIgnore::Parent))
            .ignore(ignores(NoIgnore::Dot))
            .git_ignore(ignores(NoIgnore::Vcs))
            .git_global(ignores(NoIgnore::Vcs) && ignores(NoIgnore::Global))
            .git_exclude(ignores(NoIgnore::Vcs) && ignores(NoIgnore::Exclude));
        let mut files = Vec::new();
        for entry in walk.build() {
            let entry = entry?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            // A file named on the command line is read in the language that --lang names.
            let found = SupportLang::from_path(entry.path());
            let language = match self.lang {
                Some(language) if entry.depth() == 0 => Some(language),
                Some(language) => found.filter(|found| *found == language),
                None => found,
            };
            if let Some(language) = language.filter(|language| extractors.reads(*language)) {
                files.push((entry.into_path(), language));
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files.dedup_by(|a, b| a.0 == b.0);
        Ok(files)
    }
}

/// The outline rule files of the project that the working directory is in, relative to its
/// sgconfig.yml.
fn project_rule_files() -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir()?;
    let Some(config) = project::nearest_config(&cwd) else {
        return Ok(Vec::new());
    };
    let context = || format!("invalid {}", config.display());
    let project: OutlineFolders =
        ast_grep_config::from_str(&fs::read_to_string(&config).with_context(context)?)
            .with_context(context)?;
    let root = config.parent().unwrap_or(Path::new("."));
    let mut files: Vec<PathBuf> = (project.outline_rules.into_values())
        .map(|path| root.join(path))
        .collect();
    files.sort();
    for dir in project.outline_dirs {
        let dir = root.join(dir);
        let mut found = Vec::new();
        for entry in fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                found.push(path);
            }
        }
        found.sort();
        files.extend(found);
    }
    Ok(files)
}

fn write_text(out: &mut impl Write, files: &[File], view: View) -> io::Result<()> {
    if files.is_empty() {
        writeln!(out, "nothing found")?;
    }
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            writeln!(out)?;
        }
        writeln!(out, "{}", file.path)?;
        if file.items.is_empty() {
            writeln!(out, "nothing found")?;
            continue;
        }
        if view == View::Names {
            let names = file.items.iter().map(|item| &item.entry);
            for (kind, names) in grouped(names) {
                writeln!(out, "{kind}: {}", names.join(", "))?;
            }
            continue;
        }
        let lines = (file.items.iter()).flat_map(|item| {
            std::iter::once(&item.entry).chain(item.members.iter().map(|m| &m.entry))
        });
        let width = lines
            .map(|entry| entry.range.start.line + 1)
            .max()
            .unwrap_or(1)
            .to_string()
            .len();
        for item in &file.items {
            writeln!(out, "{}", line(&item.entry, width, ""))?;
            match view {
                View::Digest => {
                    // Public members first, as ast-grep's digest lists them.
                    let members = (item.members.iter().filter(|m| m.is_public))
                        .chain(item.members.iter().filter(|m| !m.is_public))
                        .map(|member| &member.entry);
                    for (kind, names) in grouped(members) {
                        writeln!(out, "{:width$}    {kind}: {}", "", names.join(", "))?;
                    }
                }
                View::Expanded => {
                    for member in &item.members {
                        writeln!(out, "{}", line(&member.entry, width, "  "))?;
                    }
                }
                View::Auto | View::Names | View::Signatures => {}
            }
        }
    }
    Ok(())
}

/// An entry as one line: its line number, then its signature, or its name when it has none.
fn line(entry: &OutlineEntry<'_>, width: usize, indent: &str) -> String {
    let shown = if entry.signature.is_empty() {
        &entry.name
    } else {
        &entry.signature
    };
    format!("{:>width$}: {indent}{shown}", entry.range.start.line + 1)
}

/// The names of the entries, grouped by symbol type in the order each type first appears.
fn grouped<'e>(entries: impl Iterator<Item = &'e OutlineEntry<'e>>) -> Vec<(String, Vec<&'e str>)> {
    let mut groups: Vec<(SymbolType, Vec<&str>)> = Vec::new();
    for entry in entries {
        match groups
            .iter_mut()
            .find(|(kind, _)| *kind == entry.symbol_type)
        {
            Some((_, names)) => names.push(&entry.name),
            None => groups.push((entry.symbol_type, vec![&entry.name])),
        }
    }
    (groups.into_iter())
        .map(|(kind, names)| (type_name(kind), names))
        .collect()
}
