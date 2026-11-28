mod copies;
mod history;
mod python;
mod redundancy;
mod zones;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;

use crate::ledger::{self, Ledger, Place, Select, SurveyOutput, Tree, twins};
use crate::output::Format;
use crate::source;
use crate::style::Style;
use crate::syntax;
use crate::workspace::Workspace;

use crate::ledger::gix_source::Listing;
use history::History;

type Finding = ledger::Finding<Kind>;

/// The three ways to get the question wrong, and the code that stands outside the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    HiddenLink,
    NeedlessLink,
    Surplus,
    SpecialCase,
}

/// What a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// The same code is written out in more than one place.
    Copy,
    /// Two files change in the same commits, and no import joins them.
    ChangesTogether,
    /// Code writes state that another module or a function's callers depend on.
    SharedState,
    /// A module imports a module that changes at least twice as often as it does.
    VolatileImport,
    /// Modules import one another in a circle.
    ImportCycle,
    /// Code uses a private name of another module.
    PrivateReach,
    /// A module named `utils`, `helpers`, `config` or another role holds code with no single purpose.
    VagueModule,
    /// A function only passes its parameters to one call.
    PassThrough,
    /// No file mentions a function or class apart from its definition.
    Unreferenced,
    /// Every call gives a parameter the same value.
    ConstantParameter,
    /// A branch runs for one string value.
    OneValueBranch,
    /// One call alone passes a parameter that has a default.
    OneCallerParameter,
    NearCopy,
    DefaultArgument,
    /// A production module that no other surveyed Python file imports.
    OrphanedModule,
    /// A module that only re-exports names from other modules.
    FacadeModule,
    /// A helper module imported by only one non-test file.
    SingleUseModule,
    /// A large module whose top-level functions split into unrelated clusters.
    MixedResponsibilities,
}

impl ledger::Kind for Kind {
    type Group = Group;

    const GOALS: &'static [ledger::Goal] = &[ledger::Goal::Shorter, ledger::Goal::Design];

    const COMMAND: &'static str = "untangle";

    const GROUPS: &'static [Group] = &[
        Group::HiddenLink,
        Group::NeedlessLink,
        Group::Surplus,
        Group::SpecialCase,
    ];

    fn group(self) -> Group {
        match self {
            Self::Copy | Self::NearCopy | Self::ChangesTogether | Self::SharedState => {
                Group::HiddenLink
            }
            Self::VolatileImport
            | Self::ImportCycle
            | Self::PrivateReach
            | Self::VagueModule
            | Self::MixedResponsibilities => Group::NeedlessLink,
            Self::DefaultArgument
            | Self::PassThrough
            | Self::Unreferenced
            | Self::ConstantParameter
            | Self::OrphanedModule
            | Self::FacadeModule
            | Self::SingleUseModule => Group::Surplus,
            Self::OneValueBranch | Self::OneCallerParameter => Group::SpecialCase,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::HiddenLink => {
                "Hidden links: pieces that must change together, where the code does not show it"
            }
            Group::NeedlessLink => {
                "Needless links: pieces tied together that change for different reasons"
            }
            Group::Surplus => "Surplus: pieces with no reason of their own to change",
            Group::SpecialCase => {
                "Special cases: code for one situation, which the design did not account for"
            }
        }
    }
}

impl Kind {
    /// The kind that a rule of the Python pack reports, for the rules that the ledger takes
    /// over from `xpd check`.
    fn of_rule(rule: &str) -> Option<Self> {
        match rule {
            "python/pass-through-function" => Some(Self::PassThrough),
            "python/vague-module-name" => Some(Self::VagueModule),
            _ => None,
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
    /// The commits of the history. 0 outside a git repository.
    commits: usize,
}

/// List the places where the code gets wrong which pieces change together.
///
/// It reads the files and the git history, and runs no code. Copies of code and files that
/// change in the same commits are found in every programming language ast-grep parses. Imports,
/// parameters and special cases are read from Python. The JSON has `read`, `summary` with the
/// count of each kind, and `findings`, each with `kind`, `places` and `evidence`. The count is
/// the measure of a refactor: `--since` shows it at an earlier commit beside the count of today.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd untangle
  xpd untangle src/ --format json
  xpd untangle --kind copy,changes-together --limit 0
  xpd untangle --since main")]
pub(crate) struct UntangleArgs {
    #[command(flatten)]
    select: Select<Kind>,
    #[arg(long)]
    questions: bool,
}

impl UntangleArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let catalog = ledger::Catalog::load(include_str!("../rules/xpd/untangle.yml"))?;
        if self.questions {
            return catalog.print(self.select.format_override().unwrap_or(default));
        }
        let survey = |paths: &[PathBuf],
                      files: Tree<'_>,
                      top: Option<&Path>,
                      commit: &str,
                      _: Option<&str>,
                      _| {
            let history = top.map_or_else(History::default, |top| History::read(top, commit));
            survey(
                files,
                &Listing::of(paths, files, top, commit)?,
                &history,
                &catalog,
            )
        };
        self.select.print_with(default, None, survey, header)
    }
}

/// The first lines of the text: the count, and what the run read.
fn header(ledger: &Ledger<Kind, Read>) -> String {
    let Read {
        files,
        python,
        commits,
    } = ledger.read;
    let mut text = String::new();
    let _ = writeln!(
        text,
        "xpd untangle: {} in {files} files, {python} of them Python, with {commits} commits of history",
        ledger.count(),
    );
    if commits == 0 {
        text.push_str("No git history was read, so no finding rests on how the files changed.\n");
    }
    text
}

/// What one file gives the ledger.
struct Surveyed {
    shown: String,
    tracked: Option<String>,
    blocks: Vec<copies::Block>,
    shapes: twins::Shapes,
    defaults: Option<redundancy::Facts>,
    python: Option<(PathBuf, python::Facts)>,
    /// The findings that rules of `xpd check` report.
    ruled: Vec<python::Local>,
}

/// One file as the ledger needs it. `None` for a file that cannot be read.
fn read(
    path: &Path,
    language: SupportLang,
    tree: Tree<'_>,
    listing: &Listing,
    workspace: Option<&Workspace>,
) -> Option<Surveyed> {
    let text = listing.text(path)?;
    let absolute = listing.absolute(path)?;
    let tracked = tree.tracked(&absolute);
    let root = syntax::parse_as(&text, language);
    let mut ruled = Vec::new();
    let python = workspace.map(|workspace| {
        let linted = workspace.analyze(source::File::on_disk(path), &text);
        for found in &linted.found {
            let issue = found.issue();
            if let Some(kind) = Kind::of_rule(&issue.id.to_string()) {
                let line = issue.position.map_or(1, |at| at.line);
                ruled.push((kind, line, issue.message, 1));
            }
        }
        let facts = python::facts(&absolute, &root, &text, |candidate| {
            listing.exists(candidate)
        });
        (absolute, facts)
    });
    Some(Surveyed {
        shown: tree.shown(path),
        tracked,
        blocks: copies::blocks(&root.root()),
        shapes: twins::shapes(&root.root()),
        defaults: (language == SupportLang::Python).then(|| redundancy::facts(&root)),
        python,
        ruled,
    })
}

/// The copies of code among the files.
fn copied(surveyed: &[Surveyed]) -> Vec<Finding> {
    let blocks: Vec<(&str, copies::Block)> = (surveyed.iter())
        .flat_map(|file| (file.blocks.iter()).map(|block| (file.shown.as_str(), block.clone())))
        .collect();
    (copies::groups(&blocks).into_iter())
        .map(|group| {
            let (_, block) = group[0];
            let lines = block.lines.1 - block.lines.0 + 1;
            Finding {
                kind: Kind::Copy,
                key: format!("{:016x}", block.hash),
                places: (group.iter())
                    .map(|(file, block)| Place::lines(file, block.lines))
                    .collect(),
                evidence: format!(
                    "The same {lines} lines of code are written in {} places.",
                    group.len()
                ),
                weight: block.tokens * group.len(),
            }
        })
        .collect()
}

/// The findings that each file holds on its own, one for each kind and line.
fn local(surveyed: &[Surveyed]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for file in surveyed {
        let facts = file.python.iter().flat_map(|(_, facts)| &facts.local);
        for (kind, line, evidence, weight) in file.ruled.iter().chain(facts) {
            if seen.insert((*kind, &file.shown, *line)) {
                findings.push(Finding {
                    kind: *kind,
                    key: format!(
                        "{}\n{evidence}",
                        file.tracked.as_deref().unwrap_or(&file.shown)
                    ),
                    places: vec![Place::line(&file.shown, *line)],
                    evidence: evidence.clone(),
                    weight: *weight,
                });
            }
        }
    }
    findings
}

/// The pairs of files that changed together with no import between them. `shown_as` holds the
/// tracked files with the path the ledger prints for each, and `imports` the imports between
/// tracked Python modules. A pair in two folders comes first, because the layout hides it.
fn changed_together(
    history: &History,
    shown_as: &HashMap<String, String>,
    imports: &HashSet<(&str, &str)>,
) -> Vec<Finding> {
    let tracked: BTreeSet<&str> = shown_as.keys().map(String::as_str).collect();
    let mut findings = Vec::new();
    for pair in history.pairs(&tracked) {
        let (first, second) = (pair.first.as_str(), pair.second.as_str());
        if imports.contains(&(first, second)) || imports.contains(&(second, first)) {
            continue;
        }
        let python = |file: &str| source::is_python(Path::new(file));
        let apart = Path::new(first).parent() != Path::new(second).parent();
        let mut evidence = format!(
            "These two files changed together in {} of the {} commits of the one that changed less.",
            pair.together, pair.of
        );
        if python(first) && python(second) {
            evidence.push_str(" Neither imports the other.");
        }
        if apart {
            evidence.push_str(" They are in different folders.");
        }
        findings.push(Finding {
            kind: Kind::ChangesTogether,
            key: format!("{first}\n{second}"),
            places: vec![
                Place::file(&shown_as[first]),
                Place::file(&shown_as[second]),
            ],
            evidence,
            weight: pair.together * 1000 / pair.of + if apart { 1000 } else { 0 },
        });
    }
    findings
}

/// What was read and every finding of the files that `listing` lists, by group, kind and
/// strength. The rules of `xpd check` take a
/// moment to compile, so they are loaded only when a Python file is read.
fn survey(
    tree: Tree<'_>,
    listing: &Listing,
    history: &History,
    catalog: &ledger::Catalog,
) -> Result<SurveyOutput<Kind, Read>> {
    let files = &listing.files;
    let workspace = (files
        .iter()
        .any(|(_, language)| *language == SupportLang::Python))
    .then(|| Workspace::bundled(Style::Default))
    .transpose()?;
    let mut surveyed: Vec<Surveyed> = (files.par_iter())
        .filter_map(|(path, language)| {
            let workspace = workspace
                .as_ref()
                .filter(|_| *language == SupportLang::Python);
            read(path, *language, tree, listing, workspace)
        })
        .collect();
    let read = Read {
        files: surveyed.len(),
        python: surveyed.iter().filter(|file| file.python.is_some()).count(),
        commits: history.len(),
    };
    let mut findings = copied(&surveyed);
    findings.extend(local(&surveyed));
    findings.extend(redundancy::default_arguments(&surveyed)?);
    let shapes: Vec<_> = surveyed
        .iter_mut()
        .map(|file| (file.shown.as_str(), std::mem::take(&mut file.shapes)))
        .collect();
    findings.extend(redundancy::near_copies(&shapes));
    let tracked: HashMap<_, _> = surveyed
        .iter()
        .map(|file| {
            (
                file.shown.clone(),
                file.tracked.clone().unwrap_or_else(|| file.shown.clone()),
            )
        })
        .collect();
    let shown_as: HashMap<String, String> = (surveyed.iter())
        .filter_map(|file| Some((file.tracked.clone()?, file.shown.clone())))
        .collect();
    let modules: Vec<python::Module> = (surveyed.into_iter())
        .filter_map(|file| {
            let (absolute, facts) = file.python?;
            Some(python::Module {
                shown: file.shown,
                file: absolute,
                tracked: file.tracked,
                facts,
            })
        })
        .collect();
    let entry_points = listing
        .repo_file("pyproject.toml")
        .as_deref()
        .map(zones::EntryPoints::from_pyproject)
        .unwrap_or_default();
    findings.extend(python::findings(
        &modules,
        &history.changes(),
        &entry_points,
    ));
    findings.extend(changed_together(
        history,
        &shown_as,
        &python::imports(&modules),
    ));
    Ok(catalog.modules(read, findings, |path| tracked.get(path).cloned()))
}
