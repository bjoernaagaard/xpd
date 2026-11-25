//! `xpd untangle`: the ledger of the places where a codebase gets one question wrong, which is
//! whether two pieces of it should change together.
//!
//! It reads the files and the git history, and runs no code. Copies and files that change
//! together are found in every language ast-grep parses as code. The import graph, the
//! parameters and the special cases are read from Python.

mod copies;
mod history;
mod python;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use ast_grep_core::Language;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;

use crate::git;
use crate::output::Format;
use crate::source;
use crate::style::Style;
use crate::syntax;
use crate::workspace::Workspace;

use history::History;

/// How many places of a finding the text shows. The JSON holds every place.
const PLACES_SHOWN: usize = 3;

/// The three ways to get the question wrong, and the code that stands outside the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    HiddenLink,
    NeedlessLink,
    Surplus,
    SpecialCase,
}

impl Group {
    const ALL: [Self; 4] = [
        Self::HiddenLink,
        Self::NeedlessLink,
        Self::Surplus,
        Self::SpecialCase,
    ];

    fn heading(self) -> &'static str {
        match self {
            Self::HiddenLink => {
                "Hidden links: pieces that must change together, where the code does not show it"
            }
            Self::NeedlessLink => {
                "Needless links: pieces tied together that change for different reasons"
            }
            Self::Surplus => "Surplus: pieces with no reason of their own to change",
            Self::SpecialCase => {
                "Special cases: code for one situation, which the design did not account for"
            }
        }
    }
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
    /// A module named `utils`, `helpers` or `common` holds code with no single purpose.
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
    /// An exception is caught and dropped.
    SwallowedException,
    /// Code waits a fixed time, or runs again when it fails.
    SleepOrRetry,
    /// A test is skipped, or may fail while the run passes.
    DisabledTest,
}

impl Kind {
    fn group(self) -> Group {
        match self {
            Self::Copy | Self::ChangesTogether | Self::SharedState => Group::HiddenLink,
            Self::VolatileImport | Self::ImportCycle | Self::PrivateReach | Self::VagueModule => {
                Group::NeedlessLink
            }
            Self::PassThrough | Self::Unreferenced | Self::ConstantParameter => Group::Surplus,
            Self::OneValueBranch
            | Self::OneCallerParameter
            | Self::SwallowedException
            | Self::SleepOrRetry
            | Self::DisabledTest => Group::SpecialCase,
        }
    }

    fn name(self) -> String {
        self.to_possible_value()
            .expect("no kind is skipped")
            .get_name()
            .to_owned()
    }

    /// The kind that a rule of the Python pack reports, for the rules that the ledger takes
    /// over from `xpd check`.
    fn of_rule(rule: &str) -> Option<Self> {
        match rule {
            "python/pass-through-function" => Some(Self::PassThrough),
            "python/vague-module-name" => Some(Self::VagueModule),
            "python/test-skip" => Some(Self::DisabledTest),
            "python/test-sleep" | "python/test-rerun" => Some(Self::SleepOrRetry),
            _ => None,
        }
    }
}

/// One place of a finding. Lines count from 1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Place {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
}

impl Place {
    fn file(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            line: None,
            end_line: None,
        }
    }

    fn line(path: &str, line: usize) -> Self {
        Self {
            line: Some(line),
            ..Self::file(path)
        }
    }

    fn text(&self) -> String {
        match (self.line, self.end_line) {
            (Some(line), Some(end)) => format!("{}:{line}-{end}", self.path),
            (Some(line), None) => format!("{}:{line}", self.path),
            _ => self.path.clone(),
        }
    }
}

/// One entry of the ledger.
#[derive(Debug, Clone, Serialize)]
struct Finding {
    kind: Kind,
    places: Vec<Place>,
    /// What was measured, as one sentence.
    evidence: String,
    /// How strong the evidence is among findings of the same kind. The strongest come first.
    #[serde(skip)]
    weight: usize,
}

/// How many findings of one kind there are.
#[derive(Debug, Serialize)]
struct KindCount {
    kind: Kind,
    count: usize,
    /// The count at the commit of `--since`.
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
}

#[derive(Debug, Serialize)]
struct GroupCount {
    group: Group,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    kinds: Vec<KindCount>,
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

#[derive(Debug, Serialize)]
struct Summary {
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    groups: Vec<GroupCount>,
}

/// What `xpd untangle` prints.
#[derive(Debug, Serialize)]
struct Ledger {
    read: Read,
    /// The commit that `was` counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
    summary: Summary,
    /// The strongest findings of each kind, up to the limit.
    findings: Vec<Finding>,
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
    /// Files or directories (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// List only the findings of these kinds, separated by commas.
    #[arg(long, value_enum, value_delimiter = ',')]
    kind: Vec<Kind>,
    /// The most findings to list of each kind, strongest first. 0 lists every finding.
    #[arg(long, default_value_t = 20)]
    limit: usize,
    /// Count the findings at this commit too, and print each count beside the count of today.
    #[arg(long, value_name = "COMMIT")]
    since: Option<String>,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

impl UntangleArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let top = repository(&self.paths[0]);
        let history =
            (top.as_deref()).map_or_else(History::default, |top| History::read(top, "HEAD"));
        let (read, findings) = survey(&self.paths, top.as_deref(), &history)?;
        let before = match &self.since {
            Some(commit) => {
                let top = top.as_deref().with_context(|| {
                    format!(
                        "{} is in no git repository, which --since needs",
                        self.paths[0].display()
                    )
                })?;
                Some(survey_commit(&self.paths, top, commit)?)
            }
            None => None,
        };
        let ledger = Ledger {
            read,
            since: self.since.clone(),
            summary: summary(&findings, before.as_deref()),
            findings: self.listed(findings),
        };
        match self.format.unwrap_or(default) {
            Format::Json => println!("{}", serde_json::to_string(&ledger)?),
            Format::Text => print!("{}", self.text(&ledger)),
        }
        Ok(ExitCode::SUCCESS)
    }

    /// The findings of the kinds that were asked for, up to the limit of each kind.
    fn listed(&self, findings: Vec<Finding>) -> Vec<Finding> {
        let mut listed: HashMap<Kind, usize> = HashMap::new();
        (findings.into_iter())
            .filter(|finding| self.kind.is_empty() || self.kind.contains(&finding.kind))
            .filter(|finding| {
                let count = listed.entry(finding.kind).or_default();
                *count += 1;
                self.limit == 0 || *count <= self.limit
            })
            .collect()
    }

    fn text(&self, ledger: &Ledger) -> String {
        let mut text = String::new();
        let was = |was: Option<usize>| was.map_or_else(String::new, |was| format!(" (was {was})"));
        let Read {
            files,
            python,
            commits,
        } = ledger.read;
        let _ = writeln!(
            text,
            "xpd untangle: {} findings{} in {files} files, {python} of them Python, with {commits} commits of history",
            ledger.summary.count,
            was(ledger.summary.was),
        );
        if commits == 0 {
            text.push_str(
                "No git history was read, so no finding rests on how the files changed.\n",
            );
        }
        for group in &ledger.summary.groups {
            let _ = writeln!(
                text,
                "\n{}: {}{}",
                group.group.heading(),
                group.count,
                was(group.was)
            );
            for kind in &group.kinds {
                if kind.count == 0 && kind.was.unwrap_or(0) == 0 {
                    continue;
                }
                let _ = writeln!(
                    text,
                    "  {}: {}{}",
                    kind.kind.name(),
                    kind.count,
                    was(kind.was)
                );
                let listed: Vec<_> = (ledger.findings.iter())
                    .filter(|finding| finding.kind == kind.kind)
                    .collect();
                for finding in &listed {
                    let mut places: Vec<_> =
                        (finding.places.iter().take(PLACES_SHOWN).map(Place::text)).collect();
                    if finding.places.len() > PLACES_SHOWN {
                        places.push(format!("{} more", finding.places.len() - PLACES_SHOWN));
                    }
                    let _ = writeln!(text, "    {}  {}", places.join(", "), finding.evidence);
                }
                let selected = self.kind.is_empty() || self.kind.contains(&kind.kind);
                if selected && listed.len() < kind.count {
                    let _ = writeln!(
                        text,
                        "    and {} more with --kind {} --limit 0",
                        kind.count - listed.len(),
                        kind.kind.name()
                    );
                }
            }
        }
        text
    }
}

/// The count of each kind and group, with the counts of `before` beside them.
fn summary(findings: &[Finding], before: Option<&[Finding]>) -> Summary {
    let count = |findings: &[Finding], kind: Kind| {
        findings
            .iter()
            .filter(|finding| finding.kind == kind)
            .count()
    };
    let groups: Vec<GroupCount> = (Group::ALL.into_iter())
        .map(|group| {
            let kinds: Vec<KindCount> = (Kind::value_variants().iter())
                .filter(|kind| kind.group() == group)
                .map(|&kind| KindCount {
                    kind,
                    count: count(findings, kind),
                    was: before.map(|before| count(before, kind)),
                })
                .collect();
            GroupCount {
                group,
                count: kinds.iter().map(|kind| kind.count).sum(),
                was: before.map(|_| kinds.iter().filter_map(|kind| kind.was).sum()),
                kinds,
            }
        })
        .collect();
    Summary {
        count: findings.len(),
        was: before.map(<[Finding]>::len),
        groups,
    }
}

/// The top folder of the git repository that holds `path`. `None` outside a repository.
fn repository(path: &Path) -> Option<PathBuf> {
    let folder = if path.is_dir() {
        path
    } else {
        path.parent()
            .filter(|folder| !folder.as_os_str().is_empty())?
    };
    let top = git::output(folder, &["rev-parse", "--show-toplevel"]).ok()?;
    fs::canonicalize(top.trim_end_matches(['\r', '\n'])).ok()
}

/// A folder that is removed when the value is dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The findings of the files under `paths` as they were at `commit`. The files of the commit
/// are written to a folder of their own through an index of their own, so the working tree and
/// the index of the repository stay as they are.
fn survey_commit(paths: &[PathBuf], top: &Path, commit: &str) -> Result<Vec<Finding>> {
    let resolved = git::output(
        top,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )
    .with_context(|| format!("{commit} names no commit"))?;
    let resolved = resolved.trim();
    let scratch =
        Scratch(std::env::temp_dir().join(format!("xpd-untangle-{}", std::process::id())));
    let tree = scratch.0.join("tree");
    fs::create_dir_all(&tree).with_context(|| format!("cannot create {}", tree.display()))?;
    let index = scratch.0.join("index");
    let env: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", index.as_os_str())];
    git::output_with(top, &["read-tree", resolved], &env)?;
    let prefix = format!("--prefix={}/", tree.display());
    git::output_with(top, &["checkout-index", "--all", &prefix], &env)?;
    let then: Vec<PathBuf> = (paths.iter())
        .filter_map(|path| {
            let below = fs::canonicalize(path).ok()?;
            Some(tree.join(below.strip_prefix(top).ok()?))
        })
        .filter(|path| path.exists())
        .collect();
    if then.is_empty() {
        return Ok(Vec::new());
    }
    let history = History::read(top, resolved);
    Ok(survey(&then, Some(&tree), &history)?.1)
}

/// A language whose files hold code. A data or markup file repeats its structure by nature, so
/// its copies and its changes say nothing about the design.
fn code(path: &Path) -> Option<SupportLang> {
    SupportLang::from_path(path).filter(|language| {
        !matches!(
            language,
            SupportLang::Json
                | SupportLang::Yaml
                | SupportLang::Html
                | SupportLang::Css
                | SupportLang::Markdown
        )
    })
}

/// The files of code under `paths`, in path order. A folder that cannot be read is named on
/// stderr, and the rest of the project is still read.
fn files(paths: &[PathBuf]) -> Vec<(PathBuf, SupportLang)> {
    let (first, rest) = paths.split_first().expect("clap gives a default path");
    let mut walk = ignore::WalkBuilder::new(first);
    for path in rest {
        walk.add(path);
    }
    let mut files = Vec::new();
    for entry in walk.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("xpd: {error}");
                continue;
            }
        };
        if entry.file_type().is_some_and(|kind| kind.is_file())
            && let Some(language) = code(entry.path())
        {
            files.push((entry.into_path(), language));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    files
}

/// The path as the ledger prints it, without the `./` of a walk that starts in this folder.
fn shown(path: &Path) -> String {
    path.strip_prefix(".").unwrap_or(path).display().to_string()
}

/// What one file gives the ledger.
struct Surveyed {
    shown: String,
    tracked: Option<String>,
    blocks: Vec<copies::Block>,
    python: Option<(PathBuf, python::Facts)>,
    /// The findings that rules of `xpd check` report.
    ruled: Vec<python::Local>,
}

/// One file as the ledger needs it. `None` for a file that cannot be read. `top` is the folder
/// that the paths of the history start from.
fn read(
    path: &Path,
    language: SupportLang,
    top: Option<&Path>,
    workspace: Option<&Workspace>,
) -> Option<Surveyed> {
    let text = fs::read_to_string(path).ok()?;
    let absolute = fs::canonicalize(path).ok()?;
    let tracked = top
        .and_then(|top| absolute.strip_prefix(top).ok())
        .map(|below| below.to_string_lossy().replace('\\', "/"));
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
        let facts = python::facts(&absolute, &root);
        (absolute, facts)
    });
    Some(Surveyed {
        shown: shown(path),
        tracked,
        blocks: copies::blocks(&root.root()),
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
                places: (group.iter())
                    .map(|(file, block)| Place {
                        path: (*file).to_owned(),
                        line: Some(block.lines.0),
                        end_line: Some(block.lines.1),
                    })
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

/// What was read and every finding of the files under `paths`, by group, kind and strength.
/// `top` is the folder that the paths of `history` start from. The rules of `xpd check` take a
/// moment to compile, so they are loaded only when a Python file is read.
fn survey(
    paths: &[PathBuf],
    top: Option<&Path>,
    history: &History,
) -> Result<(Read, Vec<Finding>)> {
    let files = files(paths);
    let workspace = (files
        .iter()
        .any(|(_, language)| *language == SupportLang::Python))
    .then(|| Workspace::bundled(Style::Default))
    .transpose()?;
    let surveyed: Vec<Surveyed> = (files.par_iter())
        .filter_map(|(path, language)| {
            let workspace = workspace
                .as_ref()
                .filter(|_| *language == SupportLang::Python);
            read(path, *language, top, workspace)
        })
        .collect();
    let read = Read {
        files: surveyed.len(),
        python: surveyed.iter().filter(|file| file.python.is_some()).count(),
        commits: history.len(),
    };
    let mut findings = copied(&surveyed);
    findings.extend(local(&surveyed));
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
    findings.extend(python::findings(&modules, &history.changes()));
    findings.extend(changed_together(
        history,
        &shown_as,
        &python::imports(&modules),
    ));
    findings.sort_by(|a, b| {
        (
            a.kind.group(),
            a.kind,
            std::cmp::Reverse(a.weight),
            &a.places,
        )
            .cmp(&(
                b.kind.group(),
                b.kind,
                std::cmp::Reverse(b.weight),
                &b.places,
            ))
    });
    Ok((read, findings))
}
