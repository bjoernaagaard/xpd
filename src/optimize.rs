//! `xpd optimize`: the ledger of the places where code does work that a shorter or faster
//! program would not do, and that a rule of `xpd check` cannot see, because each of them shows
//! only when two places of the code are read together.
//!
//! It reads the files and runs no code. Near copies are found in every language ast-grep parses
//! as code. The calls, the loops and the arguments are read from Python, and the defaults of
//! library functions from the index that `xpd api` reads.

mod python;
mod twins;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;

use crate::index::{self, Library, Method};
use crate::ledger::{self, Ledger, Place, Select, Tree};
use crate::output::Format;
use crate::syntax;

use python::{Callee, Keyword};

type Finding = ledger::Finding<Kind>;
type Entry = ledger::Entry<Kind>;

/// How many texts that near copies differ in the evidence names.
const SWAPS_SHOWN: usize = 3;
/// How many characters of one such text the evidence quotes.
const SWAP_CHARS: usize = 30;
/// How many classes with a method the evidence names.
const CLASSES_SHOWN: usize = 3;

/// The kinds of work that code does for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    WrittenTwice,
    DoneAgain,
    AlreadyGuaranteed,
    LargerThanTheAnswer,
}

/// What a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// Two stretches of code are the same but for a few names or values.
    NearCopy,
    /// A function computes the same call more than once, with nothing changed in between.
    RepeatedCall,
    /// A loop computes a call on every pass, and changes nothing that the call reads.
    LoopInvariantCall,
    /// A keyword argument passes the value that the library function defaults to.
    DefaultArgument,
    /// A matrix of every element against every other is built and then reduced along an axis.
    MatrixReduced,
}

impl ledger::Kind for Kind {
    type Group = Group;

    const GROUPS: &'static [Group] = &[
        Group::WrittenTwice,
        Group::DoneAgain,
        Group::AlreadyGuaranteed,
        Group::LargerThanTheAnswer,
    ];

    fn group(self) -> Group {
        match self {
            Self::NearCopy => Group::WrittenTwice,
            Self::RepeatedCall | Self::LoopInvariantCall => Group::DoneAgain,
            Self::DefaultArgument => Group::AlreadyGuaranteed,
            Self::MatrixReduced => Group::LargerThanTheAnswer,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::WrittenTwice => {
                "Written twice: code that is the same but for a few names, which one function with a parameter replaces"
            }
            Group::DoneAgain => {
                "Done again: a value computed more than once, which one computation serves"
            }
            Group::AlreadyGuaranteed => {
                "Already guaranteed: an argument that changes nothing, because the library does the same without it"
            }
            Group::LargerThanTheAnswer => {
                "Larger than the answer: a value much larger than the result it is reduced to"
            }
        }
    }
}

/// What a run read.
#[derive(Debug, Clone, Copy, Serialize)]
struct Read {
    /// The files of code.
    files: usize,
    /// The Python files among them.
    python: usize,
}

/// List the places where code does work that a shorter or faster program would not do.
///
/// It reads the files and runs no code, and it lists what no rule of `xpd check` reports,
/// because each finding shows only when two places are read together. Near copies are found in
/// every programming language ast-grep parses. Repeated calls, calls that a loop repeats,
/// arguments that pass a library default and matrices that are built to be reduced are read
/// from Python. A finding is a place to read, and not yet a change: the code may differ in a way
/// the syntax does not show. The JSON has `read`, `summary` with the count of each kind, and
/// `findings`, each with `id`, `kind`, `places` and `evidence`. `--since` shows the count at an
/// earlier commit beside the count of today.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd optimize
  xpd optimize src/ --format json
  xpd optimize --kind near-copy,repeated-call --limit 0
  xpd optimize --since main")]
pub(crate) struct OptimizeArgs {
    #[command(flatten)]
    select: Select<Kind>,
}

impl OptimizeArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let header = |ledger: &Ledger<Kind, Read>| {
            let Read { files, python } = ledger.read;
            let files = if files == 1 {
                "1 file".to_owned()
            } else {
                format!("{files} files")
            };
            format!(
                "xpd optimize: {} in {files}, {python} of them Python\n",
                ledger.count()
            )
        };
        (self.select).print(default, |paths, files, _, _| survey(paths, files), header)
    }
}

/// What one file gives the ledger.
struct Surveyed {
    shown: String,
    /// The path that names the file in the key of a finding, which is the same at another
    /// commit.
    named: String,
    shapes: twins::Shapes,
    python: Option<python::Facts>,
}

/// One file as the ledger needs it. `None` for a file that cannot be read.
fn read(path: &std::path::Path, language: SupportLang, tree: Tree<'_>) -> Option<Surveyed> {
    let text = fs::read_to_string(path).ok()?;
    let absolute = fs::canonicalize(path).ok()?;
    let root = syntax::parse_as(&text, language);
    let shown = tree.shown(path);
    Some(Surveyed {
        named: tree.tracked(&absolute).unwrap_or_else(|| shown.clone()),
        shown,
        shapes: twins::shapes(&root.root()),
        python: (language == SupportLang::Python).then(|| python::facts(path, &root)),
    })
}

/// `text` on one line, cut to the length the evidence quotes for a text that differs.
fn swapped(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= SWAP_CHARS {
        return one_line;
    }
    let cut: String = one_line.chars().take(SWAP_CHARS).collect();
    format!("{cut}…")
}

/// The near copies among the files.
fn near_copies(files: &[(&str, twins::Shapes)]) -> Vec<Finding> {
    (twins::groups(files).into_iter())
        .map(|twins| {
            let (_, block) = twins.members[0];
            let lines = block.lines.1 - block.lines.0 + 1;
            let mut differ: Vec<String> = (twins.swaps.iter().take(SWAPS_SHOWN))
                .map(|(here, there)| format!("`{}` and `{}`", swapped(here), swapped(there)))
                .collect();
            if twins.swaps.len() > SWAPS_SHOWN {
                differ.push(format!("{} more", twins.swaps.len() - SWAPS_SHOWN));
            }
            let swaps: Vec<String> = (twins.swaps.iter())
                .map(|(here, there)| format!("{here}\t{there}"))
                .collect();
            Finding {
                kind: Kind::NearCopy,
                key: format!("{:016x}\n{}", block.shape, swaps.join("\n")),
                places: (twins.members.iter())
                    .map(|(file, block)| Place::lines(file, block.lines))
                    .collect(),
                evidence: format!(
                    "These {} stretches of {lines} lines are the same code but for {}.",
                    twins.members.len(),
                    differ.join(", ")
                ),
                weight: block.tokens * twins.members.len(),
            }
        })
        .collect()
}

/// The name and the default of a parameter as the source of a library writes it, as
/// `observed: bool = True`. `None` for a parameter with no default, or with one that a stub
/// file leaves out.
fn default_of(written: &str) -> Option<(&str, &str)> {
    let (name, default) = match written.split_once(':') {
        Some((name, annotated)) => (name, annotated.rsplit_once(" = ")?.1),
        None => written.split_once('=')?,
    };
    let (name, default) = (name.trim(), default.trim());
    (!name.starts_with('*') && default != "...").then_some((name, default))
}

/// Whether the literal `passed` is the value `default`, whichever quotes a string has.
fn same_value(passed: &str, default: &str) -> bool {
    let unquoted = |text: &str| {
        let quote = text.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        text.strip_prefix(quote)?
            .strip_suffix(quote)
            .map(str::to_owned)
    };
    match (unquoted(passed), unquoted(default)) {
        (Some(passed), Some(default)) => passed == default,
        (None, None) => passed == default,
        _ => false,
    }
}

/// The defaults of the library functions that the files call.
struct Defaults {
    library: Library,
    /// The parameters of each name that was looked up. `None` for a name the index lacks.
    symbols: HashMap<String, Option<Vec<String>>>,
    methods: HashMap<String, Vec<Method>>,
    /// The functions that the project defines, whose calls are not calls of a library.
    own: HashSet<String>,
}

impl Defaults {
    fn new(surveyed: &[Surveyed]) -> Result<Self> {
        let facts = || surveyed.iter().filter_map(|file| file.python.as_ref());
        let packages: BTreeSet<String> = facts()
            .flat_map(|facts| facts.packages.iter().cloned())
            .collect();
        let library = index::load(None)?;
        Ok(Self {
            methods: index::methods(&library, &packages)?,
            library,
            symbols: HashMap::new(),
            own: (facts().flat_map(|facts| facts.functions.iter().cloned())).collect(),
        })
    }

    fn symbol(&mut self, name: &str) -> Result<Option<&[String]>> {
        if !self.symbols.contains_key(name) {
            let parameters = index::api(&self.library, name)?.map(|entry| entry.parameters_written);
            self.symbols.insert(name.to_owned(), parameters);
        }
        Ok(self.symbols[name].as_deref())
    }

    /// The evidence that `keyword` passes a default. `packages` holds the packages that its
    /// file imports. A method counts when every class of those packages that has a method of
    /// the name, with the parameter, gives it this default.
    fn passed(&mut self, keyword: &Keyword, packages: &BTreeSet<String>) -> Result<Option<String>> {
        let is_default = |written: &String| {
            default_of(written).is_some_and(|(name, default)| {
                name == keyword.name && same_value(&keyword.value, default)
            })
        };
        let written = format!("`{}={}`", keyword.name, keyword.value);
        let method = match &keyword.callee {
            Callee::Symbol(symbol) => match self.symbol(symbol)? {
                Some(parameters) => {
                    return Ok((parameters.iter().any(is_default))
                        .then(|| format!("{written} passes the default of `{symbol}`.")));
                }
                None => symbol.rsplit('.').next().unwrap_or(symbol),
            },
            Callee::Method(method) => method,
        };
        if self.own.contains(method) {
            return Ok(None);
        }
        let has = |written: &String| {
            default_of(written).is_some_and(|(name, _)| name == keyword.name)
                || written.split([':', '=']).next().map(str::trim) == Some(keyword.name.as_str())
        };
        let classes: Vec<&Method> = (self.methods.get(method).into_iter().flatten())
            .filter(|(class, parameters)| {
                packages.contains(class.split('.').next().unwrap_or(class))
                    && parameters.iter().any(has)
            })
            .collect();
        let agree = !classes.is_empty()
            && (classes.iter()).all(|(_, parameters)| parameters.iter().any(is_default));
        Ok(agree.then(|| {
            let mut names: Vec<String> = (classes.iter().take(CLASSES_SHOWN))
                .map(|(class, _)| class.clone())
                .collect();
            if classes.len() > CLASSES_SHOWN {
                names.push(format!("{} more", classes.len() - CLASSES_SHOWN));
            }
            format!(
                "{written} passes the default that `{method}` has in {}.",
                names.join(", ")
            )
        }))
    }
}

/// The findings of the Python files: the ones each file holds on its own, and the keyword
/// arguments that pass a default of a library.
fn python_findings(surveyed: &[Surveyed]) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut defaults = None;
    for file in surveyed {
        let Some(facts) = &file.python else {
            continue;
        };
        for local in &facts.local {
            findings.push(Finding {
                kind: local.kind,
                key: format!("{}\n{}", file.named, local.key),
                places: (local.lines.iter().collect::<BTreeSet<_>>().into_iter())
                    .map(|line| Place::line(&file.shown, *line))
                    .collect(),
                evidence: local.evidence.clone(),
                weight: local.weight,
            });
        }
        for keyword in &facts.keywords {
            let defaults = match &mut defaults {
                Some(defaults) => defaults,
                None => defaults.insert(Defaults::new(surveyed)?),
            };
            let Some(evidence) = defaults.passed(keyword, &facts.packages)? else {
                continue;
            };
            let callee = match &keyword.callee {
                Callee::Symbol(name) | Callee::Method(name) => name,
            };
            findings.push(Finding {
                kind: Kind::DefaultArgument,
                key: format!("{}\n{callee}\n{}", file.named, keyword.name),
                places: vec![Place::line(&file.shown, keyword.line)],
                evidence,
                weight: 1,
            });
        }
    }
    Ok(findings)
}

/// What was read and every finding of the files under `paths`.
fn survey(paths: &[PathBuf], tree: Tree<'_>) -> Result<(Read, Vec<Entry>)> {
    let files = ledger::files(paths);
    let mut surveyed: Vec<Surveyed> = (files.par_iter())
        .filter_map(|(path, language)| read(path, *language, tree))
        .collect();
    let read = Read {
        files: surveyed.len(),
        python: surveyed.iter().filter(|file| file.python.is_some()).count(),
    };
    let mut findings = python_findings(&surveyed)?;
    let shapes: Vec<(&str, twins::Shapes)> = (surveyed.iter_mut())
        .map(|file| (file.shown.as_str(), std::mem::take(&mut file.shapes)))
        .collect();
    findings.extend(near_copies(&shapes));
    Ok((read, ledger::named(findings)))
}
