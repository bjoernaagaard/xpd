//! The difference between a project and the profile: the edits that bring its `pyproject.toml`
//! to the profile, the lock files its kind needs, the conflicts that no edit of
//! `pyproject.toml` resolves, and the state of each exception the project records.
//!
//! Doctor keeps the whole setup in one place per tool, the tool's table of `pyproject.toml`
//! at the project root. A file that a tool reads before that table, a nested Ruff
//! configuration and an `extend` of another file are conflicts. Every value is compared by its
//! effect, from the pinned releases' rules and defaults, without running a tool, and so is
//! whether the pinned Ruff and ty accept each setting.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Value};

use super::catalog::{Catalog, Level, Tool};
use super::edit;
use super::packaging;
use super::placement::Destination;
use super::profile::{self, Exception, LockStyle, Presence, Profile, Scope, Tracked, Values};
use super::pytest::{self, Release};
use super::ruff;
use super::ty;
use super::validate;
use crate::toml::{self, Toml};

/// The file that holds the profile's settings.
pub(crate) const PYPROJECT: &str = "pyproject.toml";
/// Git's list of files to leave untracked, at the project root.
const GITIGNORE: &str = ".gitignore";
/// The file that names the Python release uv runs the project with.
const PYTHON_VERSION: &str = ".python-version";
/// Where the profile and its record are documented.
const DOCS: &str = "https://bjoernaagaard.github.io/xpd/reference/commands/doctor/";
/// The most files and folders that the search for nested configuration looks at.
const MOST_ENTRIES: usize = 50_000;
/// Where Ruff's selector arrays can be, in the `lint` table or at their old place at the top.
const RUFF_TABLES: [&[&str]; 2] = [&["tool", "ruff", "lint"], &["tool", "ruff"]];
const RUFF_LINT: &[&str] = &["tool", "ruff", "lint"];
const TY_RULES: &[&str] = &["tool", "ty", "rules"];

/// Whether the plan changes a dependency list, which makes the lock out of date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lists {
    Changed,
    Unchanged,
}

impl Lists {
    /// Changed when either is.
    fn or(self, other: Self) -> Self {
        if self == Self::Changed { self } else { other }
    }
}

/// The dependency group that holds the exact pins of the profile's tools.
const DEV: [&str; 2] = ["dependency-groups", "dev"];

/// What the caller asks of the plan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Request<'a> {
    /// The kind named on the command line, which replaces the recorded one.
    pub(crate) kind: Option<&'a str>,
    /// The tools named on the command line, which replace the recorded ones. Empty for none.
    pub(crate) tools: &'a [String],
    /// Whether the plan records the profile revision and kind in `[tool.xpd.profile]`.
    pub(crate) stamp: bool,
}

/// One value that differs from the profile, and the edit that changes it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Change {
    /// `ruff`, `ty`, `pytest`, or `xpd` for the profile record.
    pub(crate) tool: &'static str,
    /// The rule, setting, argument or plugin, as the catalog names it.
    pub(crate) id: String,
    /// What the value is, such as `ruff PTH118 os-path-join`.
    pub(crate) item: String,
    /// The dotted path of the key the edit changes in `pyproject.toml`, or the path of an owned
    /// file.
    pub(crate) path: String,
    /// The line of that key before the edit, when it exists.
    pub(crate) line: Option<usize>,
    pub(crate) old: String,
    pub(crate) new: String,
    pub(crate) reason: String,
    pub(crate) source: String,
}

/// Something that blocks `--apply` until the owner resolves it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Conflict {
    pub(crate) file: PathBuf,
    pub(crate) line: Option<usize>,
    pub(crate) message: String,
    /// Whether doctor cannot tell which setup applies, or cannot hold this layout to the
    /// profile at all. `--check` exits 2 for such a conflict, and 1 for the others.
    pub(crate) ambiguous: bool,
}

/// A file that the profile owns, and the text the plan writes to it.
#[derive(Debug, Clone)]
pub(crate) struct OwnedFile {
    /// The path relative to the project root.
    pub(crate) path: PathBuf,
    pub(crate) before: Option<String>,
    pub(crate) after: String,
}

/// What decides the pytest settings the profile wants: the release that a fresh checkout gets,
/// and whether the project uses pytest at all.
#[derive(Debug, Clone, Copy)]
struct PytestFacts {
    release: Release,
    uses: bool,
    /// Whether the kind lets the profile add plugins to a dependency list of `pyproject.toml`.
    lists: bool,
}

/// An exception the project records, and whether the profile accepts it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExceptionState {
    pub(crate) tool: String,
    pub(crate) id: String,
    pub(crate) files: Vec<String>,
    pub(crate) value: Option<String>,
    pub(crate) reason: String,
    pub(crate) line: usize,
    /// `valid`, `invalid`, `redundant` or `orphaned`.
    pub(crate) status: &'static str,
    pub(crate) message: Option<String>,
}

/// A lock file that the project's kind needs.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Lock {
    pub(crate) file: PathBuf,
    /// `present`, `missing`, or `refresh` when the plan changes the dependencies.
    pub(crate) state: &'static str,
    /// The command that `--apply` runs to write it, from the project root.
    pub(crate) command: Vec<String>,
    /// Why the lock is out of date, when doctor can tell.
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Plan {
    pub(crate) schema: i64,
    pub(crate) revision: i64,
    /// The revision the project records. `None` before the first apply.
    pub(crate) applied: Option<i64>,
    pub(crate) kind: Option<String>,
    /// Where the kind comes from: `flag`, `record` or `suggestion`.
    pub(crate) kind_from: Option<&'static str>,
    /// Why doctor suggests the kind.
    pub(crate) kind_evidence: Option<String>,
    /// What the kind means, from the profile.
    pub(crate) kind_summary: Option<String>,
    /// The tools whose settings the project holds. `None` when it holds the whole profile.
    pub(crate) tools: Option<Vec<&'static str>>,
    pub(crate) file: PathBuf,
    pub(crate) changes: Vec<Change>,
    pub(crate) conflicts: Vec<Conflict>,
    pub(crate) exceptions: Vec<ExceptionState>,
    pub(crate) locks: Vec<Lock>,
    /// What doctor could not verify, so that it cannot say the project matches.
    pub(crate) unverified: Vec<String>,
    /// The unified diff of `pyproject.toml` and of each owned file that differs.
    pub(crate) diff: String,
    /// The text of `pyproject.toml` the plan was made from. `None` when it does not exist.
    #[serde(skip)]
    pub(crate) before: Option<String>,
    /// The text the plan writes.
    #[serde(skip)]
    pub(crate) after: String,
    /// The owned files that differ from their templates.
    #[serde(skip)]
    pub(crate) owned: Vec<OwnedFile>,
    /// The tools that `[dependency-groups] dev` pins for this project.
    #[serde(skip)]
    pins: Vec<(String, String)>,
    /// Whether the catalog already found a setting that the pinned Ruff or ty rejects.
    #[serde(skip)]
    rejected: bool,
    /// The part of the profile that the project holds.
    #[serde(skip)]
    pub(crate) scope: Scope,
}

/// The plan for the project at `root`, whose Python files are `files`.
///
/// # Errors
///
/// Returns an error if `pyproject.toml` cannot be read or parsed, its `[tool.xpd.profile]` table
/// has the wrong shape, or the kind is not one of the profile's.
pub(crate) fn compute(
    root: &Path,
    catalog: &Catalog,
    profile: &Profile,
    files: &[PathBuf],
    request: Request<'_>,
) -> Result<Plan> {
    let file = root.join(PYPROJECT);
    let before = crate::source::read_optional(&file)?;
    let original = before
        .as_ref()
        .map(|text| toml_edit::Document::parse(text.clone()))
        .transpose()
        .map_err(|error| {
            let text = before.as_deref().unwrap_or_default();
            anyhow::anyhow!("{PYPROJECT}: {}", toml::parse_error(text, &error))
        })?;
    let record = original
        .as_ref()
        .map(profile::record)
        .transpose()?
        .flatten()
        .unwrap_or_default();
    let suggestion = suggest_kind(original.as_ref(), root, files);
    let (kind, kind_from) = choose_kind(request.kind, &record, suggestion.as_ref(), profile)?;
    let scope = if request.tools.is_empty() {
        record.scope.clone()
    } else {
        Scope::parse(request.tools).context("--tools")?
    };
    let mut doc: DocumentMut = before
        .as_deref()
        .unwrap_or_default()
        .parse()
        .with_context(|| format!("{PYPROJECT} does not parse"))?;
    let mut plan = Plan {
        schema: profile::SCHEMA,
        revision: profile.revision,
        applied: record.revision,
        kind_evidence: (kind_from == Some("suggestion"))
            .then(|| suggestion.map(|(_, evidence)| evidence))
            .flatten(),
        kind_summary: kind
            .as_deref()
            .and_then(|kind| profile.kind(kind))
            .map(|kind| kind.summary.clone()),
        kind,
        kind_from,
        tools: scope.names(),
        file: PathBuf::from(PYPROJECT),
        changes: Vec::new(),
        conflicts: Vec::new(),
        exceptions: Vec::new(),
        locks: Vec::new(),
        unverified: Vec::new(),
        diff: String::new(),
        before,
        after: String::new(),
        owned: Vec::new(),
        pins: profile.pins(tracked(root)?, &scope),
        rejected: false,
        scope,
    };
    let facts = planned_facts(
        pytest_facts(root, original.as_ref()),
        profile,
        plan.kind.as_deref(),
        &plan.scope,
    );
    let values = profile.values(plan.kind.as_deref());
    plan.locations(root, original.as_ref(), profile);
    plan.layout(original.as_ref(), profile, files);
    plan.reject(original.as_ref(), catalog);
    let required = values.ruff.required(catalog);
    let accepted = plan.judge(
        &values,
        &required,
        &profile.forbid,
        facts,
        &record.exceptions,
    );
    let mut edits = Edits {
        doc: &mut doc,
        original: original.as_ref(),
        file: &file,
        catalog,
        plan: &mut plan,
    };
    let dependencies = edits.held(&values, profile, facts, &required, &accepted);
    if request.stamp {
        edits.stamp(profile, &record);
    }
    let scope = edits.plan.scope.clone();
    if scope.holds(Tool::Ruff) {
        edits.verify(&required, &accepted);
    }
    plan.after = doc.to_string();
    plan.beside(root, profile, original.as_ref(), files, dependencies)?;
    plan.diff = plan.diffs();
    Ok(plan)
}

/// The pytest release a fresh checkout of the project at `root` gets: the one `uv.lock` locks
/// when it meets every declared requirement, so that a lock the apply refreshes does not count,
/// else pytest 8 when a declared requirement admits no pytest 9, else the pinned release when a
/// list declares pytest, else the release in the virtual environment.
fn pytest_facts(root: &Path, doc: Option<&Toml>) -> PytestFacts {
    let project = pytest::Project {
        destination: Destination {
            file: root.join(PYPROJECT),
            prefix: &[],
            doc: doc.cloned(),
        },
    };
    let groups = project.groups();
    let runtime = project.runtime_dependencies();
    let declared: Vec<String> = groups
        .iter()
        .chain(runtime.iter())
        .flat_map(|group| group.entries("pytest"))
        .filter_map(|entry| entry.as_str().map(str::to_owned))
        .collect();
    let installed = pytest::installed(root).map(|(version, _)| version);
    let locked = pytest::locked(root)
        .filter(|locked| declared.iter().all(|text| pytest::satisfies(text, locked)));
    let release = if let Some(locked) = locked {
        pytest::release(Some(&locked))
    } else if declared.iter().any(|text| pytest::excludes_nine(text)) {
        Release::BeforeNine
    } else if !declared.is_empty() {
        Release::NineOrLater
    } else {
        pytest::release(installed.as_ref())
    };
    PytestFacts {
        release,
        uses: !declared.is_empty() || installed.is_some(),
        lists: true,
    }
}

/// The kind of the project, and where it comes from: the command line, the project's record or
/// doctor's suggestion.
fn choose_kind(
    flag: Option<&str>,
    record: &profile::Record,
    suggestion: Option<&(&'static str, String)>,
    profile: &Profile,
) -> Result<(Option<String>, Option<&'static str>)> {
    let (kind, from) = match (flag, &record.kind) {
        (Some(kind), _) => (Some(kind.to_owned()), Some("flag")),
        (None, Some(kind)) => (Some(kind.clone()), Some("record")),
        (None, None) => (
            suggestion.map(|(kind, _)| (*kind).to_owned()),
            suggestion.map(|_| "suggestion"),
        ),
    };
    if let Some(kind) = &kind
        && profile.kind(kind).is_none()
    {
        bail!(
            "{kind} is no kind of the profile. The kinds are {}",
            profile.kind_names().join(", ")
        );
    }
    Ok((kind, from))
}

/// The pytest facts once the plan is written: a kind that keeps `pyproject.toml` to tool settings
/// gets no dependency, and a kind that gets the profile's pin of pytest uses that release.
fn planned_facts(
    mut facts: PytestFacts,
    profile: &Profile,
    kind: Option<&str>,
    scope: &Scope,
) -> PytestFacts {
    if kind
        .and_then(|kind| profile.kind(kind))
        .is_some_and(|kind| kind.tool_only)
    {
        facts.lists = false;
    }
    if facts.lists
        && scope.holds(Tool::Pytest)
        && let Some(pinned) = profile
            .dev_pins
            .iter()
            .find(|(name, _)| name == "pytest")
            .and_then(|(_, version)| version.parse::<crate::version::Version>().ok())
    {
        facts.release = pytest::release(Some(&pinned));
        facts.uses = true;
    }
    facts
}

impl Plan {
    /// The diff of `pyproject.toml`, with the lines of tables doctor does not own hidden, and of
    /// each owned file that differs.
    fn diffs(&self) -> String {
        let mut text = pyproject_diff(self.before.as_deref(), &self.after);
        for owned in &self.owned {
            let name = owned.path.display().to_string();
            text.push_str(&diff(&name, owned.before.as_deref(), &owned.after));
        }
        text
    }

    /// What the plan holds beside the edits of `pyproject.toml`: the packaging conflicts and
    /// `.python-version` of a project that holds the whole profile, the owned files and the
    /// locks. `dependencies` says whether an edit changed a dependency list.
    fn beside(
        &mut self,
        root: &Path,
        profile: &Profile,
        doc: Option<&Toml>,
        files: &[PathBuf],
        dependencies: Lists,
    ) -> Result<()> {
        if self.scope.whole() {
            self.packaging(root, profile);
            self.python_version(root, profile)?;
        }
        self.owned_files(root, profile, doc)?;
        self.locks(root, profile, files, dependencies);
        self.ignored_locks(root);
        self.locked_pins(root, profile);
        Ok(())
    }

    /// Whether the project differs from the profile in any way: a value, a lock, a conflict or
    /// an exception the profile does not accept.
    pub(crate) fn drifts(&self) -> bool {
        !self.drift().is_empty()
    }

    /// One line for each difference from the profile. A line names the value and its state, so
    /// the same difference gives the same line in two plans.
    pub(crate) fn drift(&self) -> Vec<String> {
        let changes = self.changes.iter().map(|change| {
            format!(
                "{PYPROJECT} {}: {} is {}, and the profile wants {}",
                change.path, change.item, change.old, change.new
            )
        });
        let conflicts = self
            .conflicts
            .iter()
            .map(|conflict| format!("{}: {}", conflict.file.display(), conflict.message));
        let exceptions = self
            .exceptions
            .iter()
            .filter(|exception| exception.status != "valid")
            .map(|exception| {
                let scope = if exception.files.is_empty() {
                    String::new()
                } else {
                    format!(" for {}", exception.files.join(", "))
                };
                format!(
                    "{PYPROJECT} [tool.xpd.profile]: the exception for {} {}{scope} is {}: {}",
                    exception.tool,
                    exception.id,
                    exception.status,
                    exception.message.as_deref().unwrap_or_default()
                )
            });
        let locks = self
            .locks
            .iter()
            .filter(|lock| lock.state != "present")
            .map(|lock| {
                let state = if lock.state == "missing" {
                    "missing"
                } else {
                    "out of date"
                };
                let reason = lock
                    .reason
                    .as_deref()
                    .map_or_else(String::new, |reason| format!(", because {reason}"));
                format!(
                    "{}: the lock is {state}{reason}. xpd doctor --apply runs {}.",
                    lock.file.display(),
                    lock.command.join(" ")
                )
            });
        changes
            .chain(conflicts)
            .chain(exceptions)
            .chain(locks)
            .collect()
    }

    /// Whether `--apply` can write the plan: no conflict and only accepted exceptions.
    pub(crate) fn blockers(&self) -> Vec<String> {
        let conflicts = self
            .conflicts
            .iter()
            .map(|conflict| format!("{}: {}", conflict.file.display(), conflict.message));
        let exceptions = self
            .exceptions
            .iter()
            .filter(|exception| exception.status != "valid")
            .map(|exception| {
                format!(
                    "{PYPROJECT}:{}: the exception for {} {} is {}: {}",
                    exception.line,
                    exception.tool,
                    exception.id,
                    exception.status,
                    exception.message.as_deref().unwrap_or_default()
                )
            });
        conflicts.chain(exceptions).collect()
    }

    /// Whether a conflict makes the layout ambiguous, so that `--check` cannot tell drift.
    pub(crate) fn ambiguous(&self) -> bool {
        self.conflicts.iter().any(|conflict| conflict.ambiguous)
    }

    fn conflict(&mut self, file: impl Into<PathBuf>, line: Option<usize>, message: String) {
        self.conflicts.push(Conflict {
            file: file.into(),
            line,
            message,
            ambiguous: false,
        });
    }

    /// A conflict for a layout that doctor cannot read as one setup, or cannot hold to the
    /// profile at all.
    fn ambiguity(&mut self, file: impl Into<PathBuf>, line: Option<usize>, message: String) {
        self.conflicts.push(Conflict {
            file: file.into(),
            line,
            message,
            ambiguous: true,
        });
    }

    /// A conflict for each setting of `doc` that the pinned Ruff or ty rejects.
    fn reject(&mut self, doc: Option<&Toml>, catalog: &Catalog) {
        for problem in doc
            .map(|doc| validate::problems(doc, catalog, &self.scope))
            .unwrap_or_default()
        {
            self.rejected = true;
            self.conflict(PYPROJECT, problem.line, problem.message);
        }
    }

    /// The parts of the layout that the kind needs or forbids.
    fn layout(&mut self, doc: Option<&Toml>, profile: &Profile, files: &[PathBuf]) {
        let Some(kind) = self.kind.as_deref().and_then(|kind| profile.kind(kind)) else {
            return;
        };
        let name = kind.name.clone();
        let has = |path: &[&str]| doc.is_some_and(|doc| toml::get(doc, path).is_some());
        for (part, present, presence) in [
            ("[project]", has(&["project"]), kind.project),
            ("[build-system]", has(&["build-system"]), kind.build_system),
        ] {
            match (presence, present) {
                (Presence::Required, false) => self.conflict(
                    PYPROJECT,
                    None,
                    format!("The kind {name} needs {part} in {PYPROJECT}, and it has none. Add it, or name the kind that fits."),
                ),
                (Presence::Forbidden, true) if !kind.tool_only => self.conflict(
                    PYPROJECT,
                    None,
                    format!("The kind {name} keeps {PYPROJECT} to tool settings, and it has {part}. Move the metadata into the inline metadata of the scripts, or name the kind that fits."),
                ),
                _ => {}
            }
        }
        if kind.tool_only {
            let other: Vec<String> = doc
                .map(|doc| {
                    doc.as_table()
                        .iter()
                        .map(|(key, _)| key)
                        .filter(|key| *key != "tool")
                        .map(|key| format!("[{key}]"))
                        .collect()
                })
                .unwrap_or_default();
            if !other.is_empty() {
                self.conflict(
                    PYPROJECT,
                    None,
                    format!("The kind {name} keeps {PYPROJECT} to the [tool] table, and it has {}. Move dependencies into the inline metadata of the scripts, or name the kind that fits.", other.join(", ")),
                );
            }
        }
        let scripts = scripts(files);
        match (kind.scripts, scripts.is_empty()) {
            (Presence::Required, true) => self.conflict(
                PYPROJECT,
                None,
                format!("The kind {name} needs Python files with inline script metadata, a # /// script block, and the project has none."),
            ),
            (Presence::Forbidden, false) => self.conflict(
                PYPROJECT,
                None,
                format!("The kind {name} has no scripts with inline script metadata, and the project has {}.", scripts.len()),
            ),
            _ => {}
        }
    }

    /// The owned files of the kind that differ from their templates, each with a change, and a
    /// conflict for each other place where the tool of an owned file would read its settings.
    fn owned_files(&mut self, root: &Path, profile: &Profile, doc: Option<&Toml>) -> Result<()> {
        let backend = profile.packaging.backend(root);
        for owned in profile.owned(self.kind.as_deref(), &self.scope) {
            if owned
                .backend
                .as_deref()
                .is_some_and(|wanted| Some(wanted) != backend)
            {
                continue;
            }
            for rival in &owned.rivals {
                if root.join(rival).exists() {
                    self.conflict(
                        rival.clone(),
                        None,
                        format!("The profile keeps these settings in {}, and the same tool would also read {rival}. Remove {rival}.", owned.path),
                    );
                }
            }
            for table in &owned.tables {
                let keys: Vec<&str> = table.split('.').collect();
                if doc.is_some_and(|doc| toml::get(doc, &keys).is_some()) {
                    self.conflict(
                        PYPROJECT,
                        None,
                        format!("The profile keeps these settings in {}, and the same tool would also read [{table}] in {PYPROJECT}. Remove [{table}].", owned.path),
                    );
                }
            }
            let path = PathBuf::from(&owned.path);
            let file = root.join(&path);
            let before = crate::source::read_optional(&file)?;
            if before.as_deref() == Some(owned.text) {
                continue;
            }
            let after = match (&before, owned.insert) {
                (Some(text), true) => match inserted(text, owned.text) {
                    Some(after) => after,
                    None => continue,
                },
                _ => owned.text.to_owned(),
            };
            if let (Some(line), Some(text)) = (&owned.holds, &before) {
                if !text.lines().any(|held| held.trim_end() == line) {
                    self.conflict(
                        owned.path.clone(),
                        None,
                        format!("{} lacks the line {line}, which the profile needs there. Add it with its import, as src/doctor/profiles/{} of xpd shows.", owned.path, owned.template),
                    );
                }
                continue;
            }
            self.changes.push(Change {
                tool: "xpd",
                id: owned.name.clone(),
                item: format!("owned file {}", owned.path),
                path: owned.path.clone(),
                line: None,
                old: match (&before, owned.insert) {
                    (None, _) => "missing",
                    (Some(_), true) => "lacks lines of the profile",
                    (Some(_), false) => "differs",
                }
                .to_owned(),
                new: if owned.insert && before.is_some() {
                    "the profile's lines at its top"
                } else {
                    "the profile's copy"
                }
                .to_owned(),
                reason: if owned.insert {
                    "The profile needs each line of its template in this file, so doctor puts the missing lines at its top and keeps the project's own lines below them.".to_owned()
                } else {
                    owned.holds.as_ref().map_or_else(
                        || "The profile owns this file, so the project keeps an exact copy of its template.".to_owned(),
                        |line| format!("The profile needs the line {line} in this file, so doctor writes its template where the project has none."),
                    )
                },
                source: DOCS.to_owned(),
            });
            self.owned.push(OwnedFile {
                path,
                before,
                after,
            });
        }
        Ok(())
    }

    /// A conflict for each way the packaging of the `pyproject.toml` the plan leaves differs from
    /// the profile, and for each old packaging or hook file.
    fn packaging(&mut self, root: &Path, profile: &Profile) {
        let Some(kind) = self.kind.as_deref().and_then(|kind| profile.kind(kind)) else {
            return;
        };
        let doc = toml_edit::Document::parse(self.after.clone()).ok();
        let pinned: Vec<String> = self.pins.iter().map(|(name, _)| name.clone()).collect();
        let needs = packaging::KindNeeds {
            metadata: &kind.metadata,
            project: kind.project != Presence::Forbidden,
        };
        for found in packaging::findings(root, doc.as_ref(), &profile.packaging, needs, &pinned) {
            self.conflict(found.file, None, found.message);
        }
    }

    /// `.python-version`, which names the Python that uv runs the project with. A project
    /// without one gets the lowest release that `requires-python` admits, and one that names a
    /// release outside `requires-python` gets a conflict.
    fn python_version(&mut self, root: &Path, profile: &Profile) -> Result<()> {
        let Some(kind) = self.kind.as_deref().and_then(|kind| profile.kind(kind)) else {
            return Ok(());
        };
        if !kind.python_version {
            return Ok(());
        }
        let Some(spec) = toml_edit::Document::parse(self.after.clone())
            .ok()
            .and_then(|doc| {
                toml::get(&doc, &["project", "requires-python"])
                    .and_then(Item::as_str)
                    .map(str::to_owned)
            })
        else {
            return Ok(());
        };
        let path = PathBuf::from(PYTHON_VERSION);
        let file = root.join(&path);
        let before = crate::source::read_optional(&file)?;
        if let Some(text) = &before {
            let named = text.lines().next().unwrap_or_default().trim();
            let admitted = named
                .parse::<crate::version::Version>()
                .is_ok_and(|version| pytest::satisfies(&format!("python{spec}"), &version));
            if !admitted {
                self.conflict(
                    PYTHON_VERSION,
                    None,
                    format!("{PYTHON_VERSION} names Python {named:?}, which requires-python {spec:?} does not admit. Name a release that it admits, such as its lowest."),
                );
            }
            return Ok(());
        }
        let Some(lowest) = pytest::clauses(&format!("python{spec}"))
            .find_map(|clause| clause.strip_prefix(">="))
            .map(str::trim)
            .map(str::to_owned)
        else {
            self.conflict(
                PYTHON_VERSION,
                None,
                format!("The project has no {PYTHON_VERSION}, and requires-python {spec:?} names no lowest release with >= for doctor to pin. Write the Python release to {PYTHON_VERSION}."),
            );
            return Ok(());
        };
        let after = format!("{lowest}\n");
        self.changes.push(Change {
            tool: "xpd",
            id: "python-version".to_owned(),
            item: format!("owned file {PYTHON_VERSION}"),
            path: PYTHON_VERSION.to_owned(),
            line: None,
            old: "missing".to_owned(),
            new: format!("Python {lowest}"),
            reason: "uv runs the project, its hooks and its CI with the Python that .python-version names, so every checkout uses the same release. It starts at the lowest release that requires-python admits.".to_owned(),
            source: DOCS.to_owned(),
        });
        self.owned.push(OwnedFile {
            path,
            before: None,
            after,
        });
        Ok(())
    }

    /// A conflict for each lock file of the kind that the `.gitignore` the plan leaves ignores,
    /// since every checkout needs the lock.
    fn ignored_locks(&mut self, root: &Path) {
        let planned = self
            .owned
            .iter()
            .find(|owned| owned.path == Path::new(GITIGNORE))
            .map(|owned| owned.after.clone())
            .or_else(|| fs::read_to_string(root.join(GITIGNORE)).ok());
        let Some(text) = planned else { return };
        let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
        for line in text.lines() {
            if builder.add_line(None, line).is_err() {
                return;
            }
        }
        let Ok(matcher) = builder.build() else { return };
        let ignored: Vec<PathBuf> = self
            .locks
            .iter()
            .filter(|lock| {
                matcher
                    .matched_path_or_any_parents(&lock.file, false)
                    .is_ignore()
            })
            .map(|lock| lock.file.clone())
            .collect();
        for file in ignored {
            self.conflict(
                GITIGNORE,
                None,
                format!("{GITIGNORE} ignores {}, which every checkout needs to install the locked releases. Remove the line that matches it.", file.display()),
            );
        }
    }

    /// A conflict for each configuration file below the root that a tool reads there.
    fn nested_conflicts(&mut self, root: &Path) {
        for (nested, tools) in nested(root) {
            let shown = nested.strip_prefix(root).unwrap_or(&nested).to_path_buf();
            for tool in tools {
                if !self.scope.holds(tool) {
                    continue;
                }
                let message = match tool {
                    Tool::Ruff => format!(
                        "Ruff reads {} for the files below it instead of {PYPROJECT} at the root. Move its settings into [tool.ruff] of the root {PYPROJECT}, with per-file-ignores for its files.",
                        shown.display()
                    ),
                    Tool::Ty => format!(
                        "ty reads {} when it runs below it, instead of {PYPROJECT} at the root. Move its settings into [tool.ty] of the root {PYPROJECT}, with an override for its files.",
                        shown.display()
                    ),
                    Tool::Pytest => format!(
                        "pytest reads {} when it runs below it, instead of {PYPROJECT} at the root. Move its settings into the pytest table of the root {PYPROJECT}.",
                        shown.display()
                    ),
                };
                self.conflict(shown.clone(), None, message);
            }
        }
    }

    /// The files that keep a tool's settings outside the tool's table in `pyproject.toml`.
    fn locations(&mut self, root: &Path, doc: Option<&Toml>, profile: &Profile) {
        for name in [".ruff.toml", "ruff.toml"] {
            if self.scope.holds(Tool::Ruff) && root.join(name).is_file() {
                self.conflict(
                    name,
                    None,
                    format!("Ruff reads {name} instead of {PYPROJECT}. Move its settings into [tool.ruff] of {PYPROJECT} and delete it."),
                );
            }
        }
        if self.scope.holds(Tool::Ty) && root.join("ty.toml").is_file() {
            self.conflict(
                "ty.toml",
                None,
                format!("ty reads ty.toml instead of {PYPROJECT}. Move its settings into [tool.ty] of {PYPROJECT} and delete it."),
            );
        }
        for (name, marker) in pytest::OTHER_FILES {
            let path = root.join(name);
            let holds = match marker {
                None => path.is_file(),
                Some(marker) => fs::read_to_string(&path)
                    .is_ok_and(|text| text.lines().any(|line| line.trim() == marker)),
            };
            if holds && self.scope.holds(Tool::Pytest) {
                self.conflict(
                    name,
                    None,
                    format!("{name} holds pytest settings. Move them into the pytest table of {PYPROJECT} and remove them from {name}."),
                );
            }
        }
        if let Some(doc) = doc {
            if let Some(base) = toml::get(doc, &["tool", "ruff", "extend"])
                .and_then(Item::as_str)
                .filter(|_| self.scope.holds(Tool::Ruff))
            {
                let line = toml::line(doc, &["tool", "ruff", "extend"]);
                self.conflict(
                    PYPROJECT,
                    line,
                    format!("[tool.ruff] extends {base}. The profile keeps the whole Ruff setup in {PYPROJECT}, so copy what it needs from {base} and remove extend."),
                );
            }
            if toml::get(doc, &["tool", "uv", "workspace"]).is_some() {
                let line = toml::line(doc, &["tool", "uv", "workspace"]);
                self.ambiguity(
                    PYPROJECT,
                    line,
                    "[tool.uv.workspace] makes this a uv workspace, and doctor holds a single project to the profile.".to_owned(),
                );
            }
            if let Some(applied) = toml::get(
                doc,
                &[
                    profile::TABLE[0],
                    profile::TABLE[1],
                    profile::TABLE[2],
                    "revision",
                ],
            )
            .and_then(Item::as_integer)
            .filter(|applied| *applied > profile.revision)
            {
                self.ambiguity(
                    PYPROJECT,
                    toml::line(doc, &["tool", "xpd", "profile", "revision"]),
                    format!("{PYPROJECT} records profile revision {applied}, and this xpd holds revision {}. Use the xpd that wrote it, or a newer one.", profile.revision),
                );
            }
        }
        for parent in root.ancestors().skip(1) {
            let file = parent.join(PYPROJECT);
            if let Ok(Some(doc)) = toml::read(&file)
                && toml::get(&doc, &["tool", "uv", "workspace"]).is_some()
            {
                self.ambiguity(
                    PYPROJECT,
                    None,
                    format!("{} defines a uv workspace above this project, and doctor holds a single project to the profile.", file.display()),
                );
            }
        }
        self.nested_conflicts(root);
        if self.kind.is_none() {
            self.conflict(
                PYPROJECT,
                None,
                format!(
                    "No kind is recorded. Name one with xpd doctor --apply --kind <{}>.",
                    profile.kind_names().join("|")
                ),
            );
        }
    }

    /// The state of each recorded exception, and the exceptions the profile accepts.
    fn judge(
        &mut self,
        values: &Values,
        required: &[String],
        forbid: &profile::PytestForbid,
        facts: PytestFacts,
        exceptions: &[Exception],
    ) -> Vec<Exception> {
        let mut accepted: Vec<Exception> = Vec::new();
        for exception in exceptions {
            let (status, message) = match verdict(
                values,
                required,
                forbid,
                facts,
                exception,
                &accepted,
                &self.scope,
            ) {
                Ok(()) => {
                    accepted.push(exception.clone());
                    ("valid", None)
                }
                Err((status, message)) => (status, Some(message)),
            };
            self.exceptions.push(ExceptionState {
                tool: exception.tool.clone(),
                id: exception.id.clone(),
                files: exception.files.clone(),
                value: exception.value.clone(),
                reason: exception.reason.clone(),
                line: exception.line,
                status,
                message,
            });
        }
        accepted
    }

    /// The lock files of the kind, each with what `--apply` does about it.
    fn locks(&mut self, root: &Path, profile: &Profile, files: &[PathBuf], lists: Lists) {
        let Some(kind) = self.kind.as_deref().and_then(|kind| profile.kind(kind)) else {
            return;
        };
        match kind.lock {
            LockStyle::Project => {
                let file = PathBuf::from("uv.lock");
                let state = if !root.join(&file).is_file() {
                    "missing"
                } else if lists == Lists::Changed {
                    "refresh"
                } else {
                    "present"
                };
                self.locks.push(Lock {
                    file,
                    state,
                    command: vec!["uv".to_owned(), "lock".to_owned()],
                    reason: None,
                });
            }
            LockStyle::Script => {
                for script in scripts(files) {
                    let shown = script.strip_prefix(root).unwrap_or(&script).to_path_buf();
                    let mut lock = shown.clone().into_os_string();
                    lock.push(".lock");
                    let file = PathBuf::from(lock);
                    let state = if !root.join(&file).is_file() {
                        "missing"
                    } else if lists == Lists::Changed {
                        "refresh"
                    } else {
                        "present"
                    };
                    self.locks.push(Lock {
                        file,
                        state,
                        command: vec![
                            "uv".to_owned(),
                            "lock".to_owned(),
                            "--script".to_owned(),
                            shown.display().to_string(),
                        ],
                        reason: None,
                    });
                }
            }
        }
    }

    /// Marks `uv.lock` as one to refresh when it locks another release of a tool that the
    /// profile pins in `[dependency-groups] dev`, or none. A plan that changes a dependency list
    /// refreshes the lock anyway.
    fn locked_pins(&mut self, root: &Path, profile: &Profile) {
        let tool_only = self
            .kind
            .as_deref()
            .and_then(|kind| profile.kind(kind))
            .is_some_and(|kind| kind.tool_only);
        if tool_only {
            return;
        }
        let Some(lock) = self
            .locks
            .iter_mut()
            .find(|lock| lock.state == "present" && lock.file == Path::new("uv.lock"))
        else {
            return;
        };
        let mut reasons = Vec::new();
        for (name, version) in &self.pins {
            let locked = pytest::locked_versions(root, name);
            if locked.is_empty() {
                reasons.push(format!(
                    "it locks no {name}, and the profile pins {name} {version}"
                ));
            } else if locked.iter().any(|locked| locked != version) {
                reasons.push(format!(
                    "it locks {name} {}, and the profile pins {name} {version}",
                    locked.join(" and ")
                ));
            }
        }
        if !reasons.is_empty() {
            lock.state = "refresh";
            lock.reason = Some(reasons.join("; "));
        }
    }

    /// Runs the pinned Ruff on the settings that the plan leaves in `pyproject.toml`, and records
    /// a conflict when it rejects them, or the reason it could not run. A setting that the catalog
    /// already rejects needs no run.
    pub(crate) fn check_tools(&mut self, root: &Path, fetch: validate::Fetch) {
        if self.rejected || !self.scope.holds(Tool::Ruff) {
            return;
        }
        let release = super::catalog::RUFF_RELEASE;
        match validate::ruff_verdict(root, &self.after, fetch) {
            validate::Verdict::Accepts => {}
            validate::Verdict::Rejects(cause) => {
                self.rejected = true;
                self.conflict(
                    PYPROJECT,
                    None,
                    format!("Ruff {release} rejects the settings that the plan leaves in {PYPROJECT}: {cause}"),
                );
            }
            validate::Verdict::Unverified(reason) => self.unverified.push(format!(
                "doctor could not run Ruff {release} to check the [tool.ruff] settings: {reason}"
            )),
        }
    }

    /// Runs the locked pytest of a project that matches the profile, and records a conflict when
    /// pytest cannot start with the settings on disk, or the reason it could not run. A project
    /// that drifts is not run, because its settings are about to change. Neither is a project
    /// with no `uv.lock`, such as one of standalone scripts, which has no environment of its own.
    pub(crate) fn check_pytest(&mut self, root: &Path) {
        if self.drifts() || !root.join("uv.lock").is_file() || !self.scope.holds(Tool::Pytest) {
            return;
        }
        match validate::pytest_verdict(root) {
            validate::Verdict::Accepts => {}
            validate::Verdict::Rejects(cause) => self.conflict(
                PYPROJECT,
                None,
                format!("pytest cannot start with the settings in {PYPROJECT}: {cause}"),
            ),
            validate::Verdict::Unverified(reason) => self.unverified.push(format!(
                "doctor could not run pytest to check the [tool.pytest] settings: {reason}"
            )),
        }
    }

    /// Marks each present lock that `uv lock --check` finds out of date as one to refresh. Apart
    /// from this step and [`Plan::check_tools`], the plan reads files only.
    ///
    /// # Errors
    ///
    /// Returns an error if uv cannot run or cannot tell whether a lock is up to date.
    pub(crate) fn check_locks(&mut self, root: &Path) -> Result<()> {
        match self.run_lock_checks(root, &[]).into_iter().next() {
            Some(problem) => bail!("{problem}"),
            None => Ok(()),
        }
    }

    /// Marks each present lock that `uv lock --check --offline` finds out of date as one to
    /// refresh, from uv's cache and without the network, so that the Stop hook can run it on
    /// every turn. Returns one line, which starts with the lock file, for each lock that uv
    /// could not check.
    pub(crate) fn check_locks_offline(&mut self, root: &Path) -> Vec<String> {
        self.run_lock_checks(root, &["--offline"])
    }

    /// Runs the check of each present lock with `extra` arguments, marks each that is out of
    /// date, and returns one line for each lock that uv could not check.
    fn run_lock_checks(&mut self, root: &Path, extra: &[&str]) -> Vec<String> {
        let mut problems = Vec::new();
        for lock in self.locks.iter_mut().filter(|lock| lock.state == "present") {
            let mut command = lock.command.clone();
            command.push("--check".to_owned());
            command.extend(extra.iter().map(|arg| (*arg).to_owned()));
            let shown = command.join(" ");
            let Some((program, args)) = command.split_first() else {
                continue;
            };
            let output = Command::new(program)
                .args(args)
                .current_dir(root)
                .stdin(Stdio::null())
                .output();
            let stderr = match output {
                Ok(output) if output.status.success() => continue,
                Ok(output) => String::from_utf8_lossy(&output.stderr).into_owned(),
                Err(error) => {
                    problems.push(format!(
                        "{}: cannot run {shown}: {error}",
                        lock.file.display()
                    ));
                    continue;
                }
            };
            if stderr.contains("needs to be updated") {
                lock.state = "refresh";
            } else {
                problems.push(format!(
                    "{}: {shown} could not check it: {}",
                    lock.file.display(),
                    crate::output::hosts_only(stderr.trim())
                ));
            }
        }
        problems
    }

    pub(crate) fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_head(&mut out);
        self.write_changes(&mut out);
        self.write_problems(&mut out);
        self.write_next(&mut out);
        out
    }

    /// The revision, the kind, what the project records and the diff.
    fn write_head(&self, out: &mut String) {
        let kind = match (&self.kind, self.kind_from, &self.kind_evidence) {
            (Some(kind), Some("suggestion"), Some(evidence)) => {
                format!("kind {kind}, suggested because {evidence}")
            }
            (Some(kind), _, _) => format!("kind {kind}"),
            (None, _, _) => "no kind".to_owned(),
        };
        let tools = self
            .tools
            .as_ref()
            .map_or_else(String::new, |tools| format!(", tools {}", tools.join(", ")));
        let _ = writeln!(out, "Profile revision {}, {kind}{tools}", self.revision);
        if let Some(summary) = &self.kind_summary {
            let _ = writeln!(out, "  {summary}");
        }
        let _ = match self.applied {
            None => writeln!(out, "{PYPROJECT} records no profile yet."),
            Some(applied) if applied == self.revision => {
                writeln!(out, "{PYPROJECT} records profile revision {applied}.")
            }
            Some(applied) => writeln!(
                out,
                "{PYPROJECT} records profile revision {applied}. Revision {} is the current one.",
                self.revision
            ),
        };
        if !self.diff.is_empty() {
            out.push('\n');
            out.push_str(&self.diff);
        }
    }

    /// Each change with its reason, and each lock with what `--apply` does about it.
    fn write_changes(&self, out: &mut String) {
        if !self.changes.is_empty() {
            out.push_str("\nChanges\n");
            for change in &self.changes {
                let line = change
                    .line
                    .map_or_else(String::new, |line| format!(" at line {line}"));
                let _ = writeln!(
                    out,
                    "  {}: {} -> {}, {}{line}",
                    change.item, change.old, change.new, change.path
                );
                let _ = writeln!(out, "    {}", change.reason);
                let _ = writeln!(out, "    Source: {}", change.source);
            }
        }
        if !self.locks.is_empty() {
            out.push_str("\nLocks\n");
            for lock in &self.locks {
                let command = lock.command.join(" ");
                let _ = match lock.state {
                    "present" => writeln!(out, "  {} is present.", lock.file.display()),
                    "missing" => writeln!(
                        out,
                        "  {} is missing. --apply runs {command}.",
                        lock.file.display()
                    ),
                    _ => writeln!(
                        out,
                        "  {} is out of date{}. --apply runs {command}.",
                        lock.file.display(),
                        lock.reason
                            .as_deref()
                            .map_or_else(String::new, |reason| format!(", because {reason}"))
                    ),
                };
            }
        }
    }

    /// The exceptions and the conflicts.
    fn write_problems(&self, out: &mut String) {
        if !self.exceptions.is_empty() {
            out.push_str("\nExceptions\n");
            for exception in &self.exceptions {
                let scope = if exception.files.is_empty() {
                    String::new()
                } else {
                    format!(" for {}", exception.files.join(", "))
                };
                let value = exception
                    .value
                    .as_deref()
                    .map_or_else(String::new, |value| format!(" at {value}"));
                let _ = writeln!(
                    out,
                    "  {PYPROJECT}:{} {} {}{value}{scope}: {}",
                    exception.line, exception.tool, exception.id, exception.status
                );
                let detail = exception.message.as_deref().unwrap_or(&exception.reason);
                let _ = writeln!(out, "    {detail}");
            }
        }
        if !self.unverified.is_empty() {
            out.push_str("\nUnverified\n");
            for reason in &self.unverified {
                let _ = writeln!(out, "  {reason}");
            }
        }
        if !self.conflicts.is_empty() {
            out.push_str("\nConflicts\n");
            for conflict in &self.conflicts {
                let line = conflict
                    .line
                    .map_or_else(String::new, |line| format!(":{line}"));
                let _ = writeln!(
                    out,
                    "  {}{line}: {}",
                    conflict.file.display(),
                    conflict.message
                );
            }
        }
    }

    /// The command that comes next.
    fn write_next(&self, out: &mut String) {
        out.push('\n');
        let blocked = !self.blockers().is_empty();
        let _ = if !self.drifts() && !self.unverified.is_empty() {
            writeln!(
                out,
                "Doctor could not verify every setting, so it cannot say that the project matches profile revision {}.",
                self.revision
            )
        } else if !self.drifts() {
            writeln!(
                out,
                "The project matches profile revision {}.",
                self.revision
            )
        } else if blocked {
            writeln!(
                out,
                "Resolve the conflicts and exceptions above, then run xpd doctor --apply."
            )
        } else if self.kind_from == Some("suggestion") {
            writeln!(
                out,
                "xpd doctor --apply --kind {} writes these changes.",
                self.kind.as_deref().unwrap_or_default()
            )
        } else {
            writeln!(out, "xpd doctor --apply writes these changes.")
        };
    }
}

/// Whether the profile accepts `exception`, given the values for the project's kind, the pytest
/// release it gets, and the exceptions accepted before it.
fn verdict(
    values: &Values,
    required: &[String],
    forbid: &profile::PytestForbid,
    facts: PytestFacts,
    exception: &Exception,
    accepted: &[Exception],
    scope: &Scope,
) -> Result<(), (&'static str, String)> {
    let invalid = |message: String| Err(("invalid", message));
    let redundant = |message: String| Err(("redundant", message));
    if !exception.problems.is_empty() {
        return invalid(exception.problems.join("; "));
    }
    let id = exception.id.as_str();
    let tool = match exception.tool.as_str() {
        "ruff" => Tool::Ruff,
        "ty" => Tool::Ty,
        "pytest" => Tool::Pytest,
        other => {
            return invalid(format!(
                "doctor takes exceptions for ruff, ty and pytest, not {other}"
            ));
        }
    };
    if !scope.holds(tool) {
        return redundant(format!(
            "the project holds only the tools that tools names, and {} is not one of them",
            tool.name()
        ));
    }
    let wanted = [&values.pytest, &values.pytest_before_nine]
        .into_iter()
        .any(|wanted| wanted.addopts.iter().any(|arg| arg == id));
    if tool == Tool::Pytest && forbid.forbids(id) && !wanted {
        return invalid(format!(
            "the profile removes {id} from every project, and no exception keeps it"
        ));
    }
    match tool {
        Tool::Ruff if !required.iter().any(|code| code == id) => {
            return Err(("orphaned", orphaned(&exception.tool, id)));
        }
        Tool::Ty if !values.ty_rules.iter().any(|(rule, _)| rule == id) => {
            return Err(("orphaned", orphaned(&exception.tool, id)));
        }
        Tool::Pytest => pytest_verdict(values, facts, &exception.tool, id)?,
        _ => {}
    }
    if !exception.files.is_empty() && tool != Tool::Ruff {
        return invalid("only an exception for a Ruff rule takes files".to_owned());
    }
    for pattern in &exception.files {
        let glob = pattern.strip_prefix('!').unwrap_or(pattern);
        if let Err(error) = globset::Glob::new(glob) {
            return invalid(format!("{pattern} is no file pattern Ruff reads: {error}"));
        }
    }
    match (tool, exception.value.as_deref()) {
        (Tool::Ty, None) => {
            return invalid(
                "an exception for a ty rule needs value, the level the rule keeps".to_owned(),
            );
        }
        (Tool::Ty, Some(value)) => {
            let Some(level) = Level::parse(value) else {
                return invalid(format!(
                    "{value} is no ty level. The levels are ignore, warn and error"
                ));
            };
            let wanted = values
                .ty_rules
                .iter()
                .find(|(rule, _)| rule == id)
                .map(|(_, level)| *level);
            if wanted.is_some_and(|wanted| level >= wanted) {
                return redundant(format!(
                    "{value} is no lower than the profile's {}, so the exception changes nothing",
                    wanted.map(Level::name).unwrap_or_default()
                ));
            }
        }
        (_, Some(_)) => return invalid("only an exception for a ty rule takes a value".to_owned()),
        (_, None) => {}
    }
    for earlier in accepted {
        let same = earlier.tool == exception.tool && earlier.id == exception.id;
        let overlap = earlier.files.is_empty()
            || exception.files.is_empty()
            || earlier
                .files
                .iter()
                .any(|file| exception.files.contains(file));
        if same && overlap {
            return invalid(format!(
                "an earlier exception already covers {} {id} for these files",
                exception.tool
            ));
        }
    }
    Ok(())
}

/// Whether an exception for the pytest value `id` changes anything for a project on the release
/// in `facts`.
fn pytest_verdict(
    values: &Values,
    facts: PytestFacts,
    tool: &str,
    id: &str,
) -> Result<(), (&'static str, String)> {
    let redundant = |message: String| Err(("redundant", message));
    let (active, inactive) = match facts.release {
        Release::NineOrLater => (&values.pytest, &values.pytest_before_nine),
        Release::BeforeNine => (&values.pytest_before_nine, &values.pytest),
    };
    let names = |wanted: &profile::PytestWanted| {
        wanted.settings.iter().any(|(key, _)| key == id)
            || wanted.addopts.iter().any(|arg| arg == id)
    };
    let plugin = values.plugins.iter().any(|plugin| plugin == id);
    if plugin && !facts.uses {
        return redundant(format!(
            "the project declares no pytest, so the profile does not add {id}"
        ));
    }
    if plugin && !facts.lists {
        return redundant(format!(
            "the kind keeps pyproject.toml to tool settings, so the profile does not add {id}"
        ));
    }
    if !plugin && !names(active) {
        if names(inactive) {
            let release = match facts.release {
                Release::NineOrLater => "9 or later",
                Release::BeforeNine => "before 9",
            };
            return redundant(format!(
                "the profile sets {id} only for another pytest release, and the project gets pytest {release}"
            ));
        }
        return Err(("orphaned", orphaned(tool, id)));
    }
    Ok(())
}

fn orphaned(tool: &str, id: &str) -> String {
    format!("the profile sets nothing for {tool} {id}, so the exception has nothing to change")
}

/// The kind that the project's files point to, with the evidence.
fn suggest_kind(
    doc: Option<&Toml>,
    root: &Path,
    files: &[PathBuf],
) -> Option<(&'static str, String)> {
    let has = |path: &[&str]| doc.is_some_and(|doc| toml::get(doc, path).is_some());
    if !has(&["project"])
        && let Some(script) = scripts(files).first()
    {
        let shown = script.strip_prefix(root).unwrap_or(script);
        return Some((
            "script",
            format!(
                "{} has inline script metadata and {PYPROJECT} has no [project]",
                shown.display()
            ),
        ));
    }
    if has(&["project", "scripts"]) || has(&["project", "gui-scripts"]) {
        return Some((
            "application",
            "[project.scripts] names a command".to_owned(),
        ));
    }
    if has(&["build-system"]) {
        return Some(("library", "[build-system] builds a package".to_owned()));
    }
    if has(&["project"]) {
        return Some((
            "application",
            "[project] has no [build-system], so uv runs it without building it".to_owned(),
        ));
    }
    None
}

/// The Python files of `files` that hold inline script metadata.
/// `text` with the lines of `template` that it lacks put at its top, in the template's order.
/// `None` when it lacks none. A line counts without the spaces at its end.
fn inserted(text: &str, template: &str) -> Option<String> {
    let have: BTreeSet<&str> = text.lines().map(str::trim_end).collect();
    let missing: Vec<&str> = template
        .lines()
        .filter(|line| !line.trim().is_empty() && !have.contains(line))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let mut after = missing.join("\n");
    after.push('\n');
    after.push_str(text);
    Some(after)
}

/// Which files the project at `root` tracks that only some tools check. Git's ignore files
/// decide what counts as tracked.
fn tracked(root: &Path) -> Result<Tracked> {
    let found = crate::source::files_under(Path::new(""), &[root.to_path_buf()], |path| {
        notebook(path) || shell_script(path)
    })?;
    Ok(Tracked {
        shell: found.iter().any(|path| shell_script(path)),
        notebooks: found.iter().any(|path| notebook(path)),
    })
}

fn notebook(path: &Path) -> bool {
    path.extension().is_some_and(|suffix| suffix == "ipynb")
}

/// Whether `ShellCheck` and `shfmt` read `path`: a `.sh` or `.bash` file, or a file without a
/// suffix whose first line runs `sh`, `bash`, `dash` or `ksh`, directly or through `env`.
fn shell_script(path: &Path) -> bool {
    const SHELLS: [&str; 4] = ["sh", "bash", "dash", "ksh"];
    if let Some(suffix) = path.extension() {
        return suffix == "sh" || suffix == "bash";
    }
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut first = String::new();
    if std::io::BufRead::read_line(&mut std::io::BufReader::new(file), &mut first).is_err() {
        return false;
    }
    let Some(command) = first.strip_prefix("#!") else {
        return false;
    };
    let mut words = command.split_whitespace();
    let program = words
        .next()
        .map(|word| word.rsplit('/').next().unwrap_or(word));
    let shell = if program == Some("env") {
        words.find(|word| !word.starts_with('-'))
    } else {
        program
    };
    shell.is_some_and(|shell| SHELLS.contains(&shell))
}

fn scripts(files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file| {
            fs::read_to_string(file)
                .is_ok_and(|text| text.lines().any(|line| line.trim_end() == "# /// script"))
        })
        .cloned()
        .collect()
}

/// The configuration files below the root, outside virtual environments, each with the tools
/// that read it there.
fn nested(root: &Path) -> Vec<(PathBuf, Vec<Tool>)> {
    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .sort_by_file_name(Ord::cmp)
        .filter_entry(|entry| {
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            !(is_dir && (entry.file_name() == ".git" || entry.path().join("pyvenv.cfg").is_file()))
        })
        .build();
    walk.take(MOST_ENTRIES)
        .flatten()
        .filter(|entry| entry.depth() > 1 && entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(ignore::DirEntry::into_path)
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let tools = match name {
                "ruff.toml" | ".ruff.toml" => vec![Tool::Ruff],
                "ty.toml" => vec![Tool::Ty],
                "pytest.toml" | ".pytest.toml" | "pytest.ini" | ".pytest.ini" => vec![Tool::Pytest],
                "tox.ini" | "setup.cfg" => {
                    let marker = if name == "tox.ini" {
                        "[pytest]"
                    } else {
                        "[tool:pytest]"
                    };
                    let text = fs::read_to_string(&path).ok()?;
                    if text.lines().any(|line| line.trim() == marker) {
                        vec![Tool::Pytest]
                    } else {
                        Vec::new()
                    }
                }
                PYPROJECT => {
                    let doc = toml::read(&path).ok().flatten()?;
                    let has = |table: &[&str]| toml::get(&doc, table).is_some();
                    [
                        (Tool::Ruff, has(&["tool", "ruff"])),
                        (Tool::Ty, has(&["tool", "ty"])),
                        (Tool::Pytest, has(&["tool", "pytest"])),
                    ]
                    .into_iter()
                    .filter_map(|(tool, present)| present.then_some(tool))
                    .collect()
                }
                _ => Vec::new(),
            };
            (!tools.is_empty()).then_some((path, tools))
        })
        .collect()
}

/// The tables of `pyproject.toml` that doctor owns: it reads and edits their values, and the
/// diff shows their lines.
const OWNED: [&[&str]; 9] = [
    &["tool", "ruff"],
    &["tool", "uv", "exclude-newer"],
    &["project", "requires-python"],
    &["tool", "ty"],
    &["tool", "pytest"],
    &["tool", "xpd"],
    &["dependency-groups"],
    &["project", "dependencies"],
    &["project", "optional-dependencies"],
];

/// The line that stands for a line of a table that doctor does not own.
const HIDDEN: &str = "<hidden>";

/// `text`, a `pyproject.toml`, with each line outside the tables doctor owns replaced by
/// [`HIDDEN`], so that no output of doctor shows a setting it does not change, such as an index
/// URL with a token. A line shows only when everything on it belongs to an owned table, so a line
/// with a value of another table is hidden, also inside an inline or dotted `tool` table. Table
/// headers and blank lines stay. Text that does not parse is all hidden.
fn shown(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let Ok(doc) = toml_edit::Document::parse(text.to_owned()) else {
        return lines.iter().map(|line| hide(line)).collect();
    };
    let line_of = |offset: usize| toml::line_of(&doc, offset) - 1;
    let mut headers: Vec<(usize, Vec<String>)> = Vec::new();
    let mut values: Vec<(usize, usize, bool)> = Vec::new();
    collect(
        doc.as_table(),
        &mut Vec::new(),
        &line_of,
        &mut headers,
        &mut values,
    );
    headers.sort();
    let mut show: Vec<bool> = (0..lines.len())
        .map(|index| {
            let table = headers
                .iter()
                .rev()
                .find(|(line, _)| *line <= index)
                .map(|(_, path)| path.iter().map(String::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            !table.is_empty() && OWNED.iter().any(|owned| table.starts_with(owned))
        })
        .collect();
    let mark = |show: &mut Vec<bool>, owned: bool| {
        for (start, end, _) in values.iter().filter(|(_, _, value)| *value == owned) {
            for line in show.iter_mut().take(end + 1).skip(*start) {
                *line = owned;
            }
        }
    };
    mark(&mut show, true);
    for (line, _) in &headers {
        if let Some(line) = show.get_mut(*line) {
            *line = true;
        }
    }
    mark(&mut show, false);
    lines
        .iter()
        .zip(show)
        .map(|(line, show)| {
            if show || line.trim().is_empty() {
                (*line).to_owned()
            } else {
                hide(line)
            }
        })
        .collect()
}

/// [`HIDDEN`] with the line ending of `line`.
fn hide(line: &str) -> String {
    if line.ends_with('\n') {
        format!("{HIDDEN}\n")
    } else {
        HIDDEN.to_owned()
    }
}

/// The line of each table header below `table`, at `path`, with the table's path, and the first
/// and last line of each value with whether doctor owns it. An inline table counts as the values
/// inside it, so a line that holds values of an owned and of another table has both.
fn collect(
    table: &dyn toml_edit::TableLike,
    path: &mut Vec<String>,
    line_of: &dyn Fn(usize) -> usize,
    headers: &mut Vec<(usize, Vec<String>)>,
    values: &mut Vec<(usize, usize, bool)>,
) {
    for (key, item) in table.iter() {
        path.push(key.to_owned());
        let header = |table: &toml_edit::Table, path: &[String]| {
            table
                .span()
                .map(|span| (line_of(span.start), path.to_vec()))
        };
        match item {
            Item::Table(child) => {
                if !child.is_dotted() {
                    headers.extend(header(child, path));
                }
                collect(child, path, line_of, headers, values);
            }
            Item::ArrayOfTables(tables) => {
                for child in tables {
                    headers.extend(header(child, path));
                    collect(child, path, line_of, headers, values);
                }
            }
            Item::Value(toml_edit::Value::InlineTable(child)) => {
                collect(child, path, line_of, headers, values);
            }
            Item::Value(value) => {
                let start = table
                    .key(key)
                    .and_then(toml_edit::Key::span)
                    .into_iter()
                    .chain(value.span())
                    .map(|span| span.start)
                    .min();
                if let (Some(start), Some(span)) = (start, value.span()) {
                    let parts: Vec<&str> = path.iter().map(String::as_str).collect();
                    let owned = OWNED.iter().any(|owned| parts.starts_with(owned));
                    values.push((line_of(start), line_of(span.end), owned));
                }
            }
            Item::None => {}
        }
        path.pop();
    }
}

/// The unified diff of `pyproject.toml` from `before` to `after`, with the lines of tables that
/// doctor does not own hidden on both sides.
fn pyproject_diff(before: Option<&str>, after: &str) -> String {
    if before == Some(after) {
        return String::new();
    }
    diff(PYPROJECT, before.map(shown).as_deref(), &shown(after))
}

/// The unified diff of the file `name` from `before` to `after`, empty when they are the same.
/// An empty context line is written empty, as GNU diff --suppress-blank-empty writes it and as
/// git apply and patch read it, so the diff has no trailing whitespace.
fn diff(name: &str, before: Option<&str>, after: &str) -> String {
    let old = before.unwrap_or_default();
    if before.is_some() && old == after {
        return String::new();
    }
    let old_name = if before.is_some() { name } else { "/dev/null" };
    let text = similar::TextDiff::from_lines(old, after)
        .unified_diff()
        .context_radius(3)
        .header(old_name, name)
        .to_string();
    text.split_inclusive('\n')
        .map(|line| match line {
            " \n" => "\n",
            " " => "",
            line => line,
        })
        .collect()
}

/// The edits to one `pyproject.toml`, and the plan they are recorded in.
struct Edits<'a> {
    doc: &'a mut DocumentMut,
    original: Option<&'a Toml>,
    file: &'a Path,
    catalog: &'a Catalog,
    plan: &'a mut Plan,
}

impl Edits<'_> {
    /// The document as it stands after the edits so far.
    fn current(&self) -> Toml {
        toml_edit::Document::parse(self.doc.to_string())
            .expect("toml_edit writes a document that it parses")
    }

    /// The line of `path` before any edit.
    fn line(&self, path: &[&str]) -> Option<usize> {
        self.original.and_then(|doc| toml::line(doc, path))
    }

    fn change(
        &mut self,
        (tool, id): (&'static str, &str),
        item: String,
        path: &[&str],
        old: String,
        new: String,
        explained: (String, String),
    ) {
        let line = self.line(path);
        let (reason, source) = explained;
        self.plan.changes.push(Change {
            tool,
            id: id.to_owned(),
            item,
            path: dotted(path),
            line,
            old,
            new,
            reason,
            source,
        });
    }

    /// The reason and source of the catalog record that explains `id` of `tool`.
    fn explain(&self, tool: Tool, id: &str) -> (String, String) {
        self.catalog.profiled(tool, id).map_or_else(
            || (String::new(), DOCS.to_owned()),
            |record| (record.plain_reason(), record.source.clone()),
        )
    }

    fn rule_name(&self, code: &str) -> String {
        self.catalog
            .ruff_rules()
            .find(|rule| rule.key.id == code)
            .and_then(|rule| rule.name.clone())
            .map_or_else(
                || format!("ruff {code}"),
                |name| format!("ruff {code} {name}"),
            )
    }

    fn resolve(
        &self,
    ) -> (
        BTreeSet<String>,
        BTreeMap<String, super::evidence::Location>,
    ) {
        ruff::resolve(self.catalog, self.file, self.current())
    }

    /// Removes the exact `code` from the selector arrays named `keys`, and returns the dotted
    /// path of each array it was in.
    fn unselect(&mut self, code: &str, keys: &[&str]) -> Vec<Vec<String>> {
        let mut found = Vec::new();
        for table in RUFF_TABLES {
            for key in keys {
                let Some(array) = edit::existing(self.doc, table)
                    .and_then(|table| table.get_mut(key))
                    .and_then(Item::as_array_mut)
                else {
                    continue;
                };
                let indexes: Vec<usize> = edit::strings(array)
                    .into_iter()
                    .filter(|(_, selector)| selector == code)
                    .map(|(index, _)| index)
                    .collect();
                if !indexes.is_empty() {
                    edit::remove_items(array, &indexes);
                    let mut path: Vec<String> =
                        table.iter().map(|part| (*part).to_owned()).collect();
                    path.push((*key).to_owned());
                    found.push(path);
                }
            }
        }
        found
    }

    /// The edits for each part of the profile that the project holds: the settings of each tool
    /// and its pins, and the packaging values when it holds the whole profile. Returns whether a
    /// dependency list changed.
    fn held(
        &mut self,
        values: &Values,
        profile: &Profile,
        facts: PytestFacts,
        required: &[String],
        accepted: &[Exception],
    ) -> Lists {
        let scope = self.plan.scope.clone();
        if scope.holds(Tool::Ruff) {
            self.ruff(values, required, accepted);
        }
        if scope.holds(Tool::Ty) {
            self.ty(values, accepted);
        }
        let wanted_pins = self.plan.pins.clone();
        let mut dependencies = self.pins(&wanted_pins, facts);
        if scope.holds(Tool::Pytest) {
            dependencies = dependencies.or(self.pytest(values, profile, facts, accepted));
        }
        if scope.whole() {
            dependencies = dependencies.or(self.packaging(&profile.packaging));
        }
        dependencies
    }

    /// The Ruff settings, selection, ignores and banned APIs of the profile. Every rule of the
    /// profile runs, outside the files and rules that the profile's reasons and the project's
    /// exceptions name.
    fn ruff(&mut self, values: &Values, required: &[String], accepted: &[Exception]) {
        let wanted = &values.ruff;
        self.ruff_select(&wanted.select);
        let ruff_exceptions = || accepted.iter().filter(|exception| exception.tool == "ruff");
        let whole: BTreeSet<&str> = ruff_exceptions()
            .filter(|exception| exception.files.is_empty())
            .map(|exception| exception.id.as_str())
            .collect();
        let required: Vec<&String> = required
            .iter()
            .filter(|code| !whole.contains(code.as_str()))
            .collect();
        self.ruff_ignore(wanted, &required);
        self.settings(Tool::Ruff, &["tool", "ruff"], &wanted.settings);
        self.ruff_off(&wanted.off);
        self.per_file(wanted, &required, accepted);
        self.banned_api(&wanted.banned_api);
        for exception in ruff_exceptions().filter(|exception| exception.files.is_empty()) {
            let (enabled, _) = self.resolve();
            if !enabled.contains(&exception.id) {
                continue;
            }
            let removed = self.unselect(&exception.id, &["select", "extend-select"]);
            let (enabled, _) = self.resolve();
            let path: Vec<String> = if enabled.contains(&exception.id) {
                if let Some(array) = edit::array(self.doc, RUFF_LINT, "ignore") {
                    edit::push(array, self.catalog.ruff_selector(&exception.id));
                }
                ["tool", "ruff", "lint", "ignore"]
                    .map(str::to_owned)
                    .to_vec()
            } else {
                removed.into_iter().next().unwrap_or_default()
            };
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            let item = self.rule_name(&exception.id);
            let explained = (exception.reason.clone(), DOCS.to_owned());
            self.change(
                ("ruff", &exception.id),
                item,
                &path,
                "on".to_owned(),
                "off, by an exception".to_owned(),
                explained,
            );
        }
    }

    /// Each setting of `wanted` holds the profile's value in the table at `table`. A lint
    /// setting of Ruff counts at its old place at the top of `[tool.ruff]` too, which the `lint`
    /// table overrides.
    fn settings(&mut self, tool: Tool, table: &[&str], wanted: &[profile::Setting]) {
        let current = self.current();
        let prefix = if tool == Tool::Ty { "tool.ty." } else { "" };
        for setting in wanted {
            let path: Vec<&str> = table
                .iter()
                .copied()
                .chain(setting.path.iter().map(String::as_str))
                .collect();
            let mut places = vec![path.clone()];
            if tool == Tool::Ruff && setting.path.first().is_some_and(|part| part == "lint") {
                places.push(
                    path.iter()
                        .copied()
                        .filter(|part| *part != "lint")
                        .collect(),
                );
            }
            let found = places
                .iter()
                .find_map(|place| toml::get(&current, place).map(|item| (place, item)));
            let value = toml::Observed::of_value(&setting.value);
            if found
                .and_then(|(_, item)| toml::Observed::of_item(item))
                .as_ref()
                == Some(&value)
            {
                continue;
            }
            let old = found
                .and_then(|(place, item)| {
                    let span = item.span()?;
                    let text = current.raw().get(span)?;
                    Some(if place.len() == path.len() {
                        text.to_owned()
                    } else {
                        format!("{text}, at {}", dotted(place))
                    })
                })
                .unwrap_or_else(|| "unset".to_owned());
            let Some((key, parent)) = path.split_last() else {
                continue;
            };
            edit::set(self.doc, parent, key, setting.value.clone());
            let id = profile::option(self.catalog, tool, prefix, &setting.path)
                .unwrap_or_else(|_| setting.dotted());
            let explained = self.explain(tool, &id);
            self.change(
                (tool.name(), &id),
                format!("{} {}", tool.name(), setting.dotted()),
                &path,
                old,
                setting.value.to_string().trim().to_owned(),
                explained,
            );
        }
    }

    /// Removes each Ruff setting of `off` that the project sets to `true`.
    fn ruff_off(&mut self, off: &[Vec<String>]) {
        for path in off {
            let canonical: Vec<&str> = ["tool", "ruff"]
                .into_iter()
                .chain(path.iter().map(String::as_str))
                .collect();
            let legacy: Vec<&str> = canonical
                .iter()
                .copied()
                .filter(|part| *part != "lint")
                .collect();
            for place in [canonical, legacy] {
                let on = toml::get(&self.current(), &place)
                    .and_then(Item::as_bool)
                    .unwrap_or(false);
                let Some((key, parent)) = place.split_last() else {
                    continue;
                };
                if !on || !edit::remove(self.doc, parent, key) {
                    continue;
                }
                let id = profile::option(self.catalog, Tool::Ruff, "", path)
                    .unwrap_or_else(|_| path.join("."));
                let explained = self.explain(Tool::Ruff, &id);
                self.change(
                    ("ruff", &id),
                    format!("ruff {}", path.join(".")),
                    &place,
                    "true".to_owned(),
                    "removed".to_owned(),
                    explained,
                );
            }
        }
    }

    /// `lint.select` holds every selector of the profile's selection, or else becomes it. The
    /// old `select` at the top of `[tool.ruff]` goes, so that one selection applies.
    fn ruff_select(&mut self, select: &[String]) {
        if select.is_empty() {
            return;
        }
        let current = self.current();
        let canonical = ["tool", "ruff", "lint", "select"];
        let legacy = ["tool", "ruff", "select"];
        let held = |path: &[&str]| -> Option<Vec<String>> {
            let array = toml::get(&current, path)?.as_array()?;
            Some(
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect(),
            )
        };
        let (path, old): (&[&str], Option<Vec<String>>) = match held(&canonical) {
            Some(old) => (&canonical, Some(old)),
            None => match held(&legacy) {
                Some(old) => (&legacy, Some(old)),
                None => (&canonical, None),
            },
        };
        if old
            .as_ref()
            .is_some_and(|old| select.iter().all(|selector| old.contains(selector)))
        {
            return;
        }
        if path == legacy {
            edit::remove(self.doc, &legacy[..2], "select");
        }
        let value = Value::Array(select.iter().map(String::as_str).collect());
        edit::set(self.doc, RUFF_LINT, "select", value);
        let shown = |items: &[String]| {
            format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| format!("{item:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let explained = self.explain(Tool::Ruff, "lint.select");
        self.change(
            ("ruff", "lint.select"),
            "ruff lint.select".to_owned(),
            path,
            old.as_deref().map_or_else(|| "unset".to_owned(), shown),
            shown(select),
            explained,
        );
    }

    /// The rules of `required` that `selector` turns off, in the preview mode of the edits so far.
    fn turned_off(&self, selector: &str, required: &[&String]) -> Vec<String> {
        let Some(selector) = self.catalog.redirect(selector) else {
            return Vec::new();
        };
        self.catalog
            .ruff_rules()
            .filter(|rule| required.contains(&&rule.key.id))
            .filter(|rule| ruff::selects(selector, rule))
            .map(|rule| rule.key.id.clone())
            .collect()
    }

    /// A selector of `ignore` or `extend-ignore` that turns off a rule of the profile goes, and
    /// each selector of the profile's ignore list that the project lacks comes.
    fn ruff_ignore(&mut self, wanted: &profile::RuffWanted, required: &[&String]) {
        let mut held: BTreeSet<String> = BTreeSet::new();
        for table in RUFF_TABLES {
            for key in ["ignore", "extend-ignore"] {
                let mut path: Vec<&str> = table.to_vec();
                path.push(key);
                let selectors: Vec<(usize, String)> = edit::existing(self.doc, table)
                    .and_then(|table| table.get(key))
                    .and_then(Item::as_array)
                    .map(edit::strings)
                    .unwrap_or_default();
                let mut drop = Vec::new();
                for (index, selector) in selectors {
                    let off = self.turned_off(&selector, required);
                    if off.is_empty() {
                        held.insert(self.catalog.ruff_code(&selector).to_owned());
                        continue;
                    }
                    drop.push(index);
                    let shown = format!("{} = [\"{selector}\"]", dotted(&path));
                    self.record_ruff_ignore_removal(
                        &selector,
                        &path,
                        &off,
                        &format!("off, by {shown}"),
                        "",
                        "lint.ignore",
                    );
                }
                if let Some(array) = edit::existing(self.doc, table)
                    .and_then(|table| table.get_mut(key))
                    .and_then(Item::as_array_mut)
                {
                    edit::remove_items(array, &drop);
                }
            }
        }
        let mut path: Vec<&str> = RUFF_LINT.to_vec();
        path.push("ignore");
        for selector in &wanted.ignore {
            if held.contains(selector) {
                continue;
            }
            if let Some(array) = edit::array(self.doc, RUFF_LINT, "ignore") {
                edit::push(array, self.catalog.ruff_selector(selector));
            }
            let explained = self.explain_ignore(selector);
            self.change(
                ("ruff", selector),
                format!("ruff {selector}"),
                &path,
                "absent".to_owned(),
                "present".to_owned(),
                explained,
            );
        }
    }

    /// Records the removal of a Ruff ignore selector that turned off one or more profile rules.
    fn record_ruff_ignore_removal(
        &mut self,
        selector: &str,
        path: &[&str],
        off: &[String],
        exact_old: &str,
        scope_suffix: &str,
        setting: &str,
    ) {
        let exact = off.len() == 1 && off[0] == self.catalog.ruff_code(selector);
        let (item, old, new, explained) = if exact {
            (
                self.rule_name(&off[0]),
                exact_old.to_owned(),
                "on".to_owned(),
                self.explain(Tool::Ruff, &off[0]),
            )
        } else {
            (
                format!("ruff {selector}"),
                format!(
                    "turns off {} rules of the profile{scope_suffix}, such as {}",
                    off.len(),
                    off[0]
                ),
                "removed".to_owned(),
                self.explain(Tool::Ruff, setting),
            )
        };
        self.change(("ruff", selector), item, path, old, new, explained);
    }

    /// The reason and source of the rules that the profile's ignore selector `selector` turns
    /// off, which share one reason.
    fn explain_ignore(&self, selector: &str) -> (String, String) {
        self.catalog
            .ruff_rules()
            .find(|rule| ruff::selects(selector, rule))
            .map_or_else(
                || self.explain(Tool::Ruff, "lint.ignore"),
                |rule| self.explain(Tool::Ruff, &rule.key.id),
            )
    }

    /// A per-file ignore that turns off a rule of the profile for a file pattern goes, unless the
    /// profile or an exception keeps that rule off for that pattern. Then each per-file ignore of
    /// the profile and of an exception comes.
    fn per_file(
        &mut self,
        wanted: &profile::RuffWanted,
        required: &[&String],
        accepted: &[Exception],
    ) {
        let scoped: BTreeSet<(&str, &str)> = accepted
            .iter()
            .filter(|exception| exception.tool == "ruff")
            .flat_map(|exception| {
                exception
                    .files
                    .iter()
                    .map(|pattern| (exception.id.as_str(), pattern.as_str()))
            })
            .chain(wanted.per_file_ignores.iter().flat_map(|(pattern, codes)| {
                codes.iter().map(|code| (code.as_str(), pattern.as_str()))
            }))
            .collect();
        let covered = self.drop_per_file(required, &scoped);
        self.add_per_file(wanted, accepted, covered);
    }

    /// Removes each selector of a per-file ignore that turns off a rule of `required` for its
    /// pattern, unless `scoped` keeps that rule off for that pattern. Returns each selector and
    /// pattern that stays.
    fn drop_per_file(
        &mut self,
        required: &[&String],
        scoped: &BTreeSet<(&str, &str)>,
    ) -> BTreeSet<(String, String)> {
        let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
        for table in RUFF_TABLES {
            for key in ["per-file-ignores", "extend-per-file-ignores"] {
                let mut path: Vec<&str> = table.to_vec();
                path.push(key);
                let patterns: Vec<(String, Vec<(usize, String)>)> = edit::existing(self.doc, &path)
                    .map(|table| {
                        table
                            .iter()
                            .filter_map(|(pattern, item)| {
                                Some((pattern.to_owned(), edit::strings(item.as_array()?)))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for (pattern, selectors) in patterns {
                    let mut full = path.clone();
                    full.push(&pattern);
                    let mut drop = Vec::new();
                    for (index, selector) in &selectors {
                        let off: Vec<String> = self
                            .turned_off(selector, required)
                            .into_iter()
                            .filter(|code| !scoped.contains(&(code.as_str(), pattern.as_str())))
                            .collect();
                        if off.is_empty() {
                            let code = self.catalog.ruff_code(selector).to_owned();
                            covered.insert((code, pattern.clone()));
                            continue;
                        }
                        drop.push(*index);
                        self.record_ruff_ignore_removal(
                            selector,
                            &full,
                            &off,
                            &format!("off for {pattern}"),
                            &format!(" for {pattern}"),
                            "lint.per-file-ignores",
                        );
                    }
                    if drop.is_empty() {
                        continue;
                    }
                    let Some(table) = edit::existing(self.doc, &path) else {
                        continue;
                    };
                    let empty = table
                        .get_mut(&pattern)
                        .and_then(Item::as_array_mut)
                        .is_some_and(|array| {
                            edit::remove_items(array, &drop);
                            array.is_empty()
                        });
                    if empty {
                        table.remove(&pattern);
                    }
                }
            }
        }
        covered
    }

    /// Adds each per-file ignore of the profile and of an exception that `covered` lacks.
    fn add_per_file(
        &mut self,
        wanted: &profile::RuffWanted,
        accepted: &[Exception],
        mut covered: BTreeSet<(String, String)>,
    ) {
        let profile_ignores = wanted.per_file_ignores.iter().flat_map(|(pattern, codes)| {
            codes
                .iter()
                .map(move |code| (code.clone(), pattern.clone(), None))
        });
        let exceptions = accepted
            .iter()
            .filter(|exception| exception.tool == "ruff")
            .flat_map(|exception| {
                exception.files.iter().map(move |pattern| {
                    (
                        exception.id.clone(),
                        pattern.clone(),
                        Some(exception.reason.clone()),
                    )
                })
            });
        let wanted: Vec<(String, String, Option<String>)> =
            profile_ignores.chain(exceptions).collect();
        for (code, pattern, reason) in wanted {
            if !covered.insert((code.clone(), pattern.clone())) {
                continue;
            }
            let mut path: Vec<&str> = RUFF_LINT.to_vec();
            path.push("per-file-ignores");
            if let Some(array) = edit::array(self.doc, &path, &pattern) {
                edit::push(array, self.catalog.ruff_selector(&code));
            }
            path.push(&pattern);
            let item = self.rule_name(&code);
            let (new, explained) = match reason {
                Some(reason) => (
                    format!("off for {pattern}, by an exception"),
                    (reason, DOCS.to_owned()),
                ),
                None => (
                    format!("off for {pattern}"),
                    self.explain(Tool::Ruff, "lint.per-file-ignores"),
                ),
            };
            self.change(
                ("ruff", &code),
                item,
                &path,
                "on".to_owned(),
                new,
                explained,
            );
        }
    }

    /// Each API of `bans` is an entry of `banned-api`. An entry the project has keeps its own
    /// message, and a new entry gets the profile's.
    fn banned_api(&mut self, bans: &[(String, String)]) {
        let table = ["tool", "ruff", "lint", "flake8-tidy-imports", "banned-api"];
        let legacy = ["tool", "ruff", "flake8-tidy-imports", "banned-api"];
        let current = self.current();
        let has = |path: &[&str], name: &str| {
            toml::get(&current, path)
                .and_then(Item::as_table_like)
                .is_some_and(|table| table.contains_key(name))
        };
        let (_, source) = self.explain(Tool::Ruff, "lint.flake8-tidy-imports.banned-api");
        for (name, message) in bans {
            if has(&table, name) || has(&legacy, name) {
                continue;
            }
            let mut entry = toml_edit::InlineTable::new();
            entry.insert("msg", Value::from(message.as_str()));
            edit::set(self.doc, &table, name, Value::InlineTable(entry));
            let mut path = table.to_vec();
            path.push(name);
            self.change(
                ("ruff", "lint.flake8-tidy-imports.banned-api"),
                format!("ruff banned API {name}"),
                &path,
                "allowed".to_owned(),
                "banned".to_owned(),
                (message.clone(), source.clone()),
            );
        }
    }

    /// Every ty rule of the profile has at least its level, or the level its exception keeps, in
    /// the project's own settings. A release default does not count, since it can change.
    fn ty(&mut self, values: &Values, accepted: &[Exception]) {
        let prefix = ["tool", "ty"];
        let state = ty::state(self.catalog, self.file, &prefix, Some(&self.current()));
        for (rule, level) in &values.ty_rules {
            let kept = accepted
                .iter()
                .find(|exception| exception.tool == "ty" && exception.id == *rule)
                .and_then(|exception| exception.value.as_deref().and_then(Level::parse));
            let set = state.rules.get(rule).or(state.all.as_ref());
            let effective = set.map(|set| set.level);
            let satisfied = match (kept, effective) {
                (Some(kept), Some(effective)) => effective == kept,
                (None, Some(effective)) => effective >= *level,
                (_, None) => false,
            };
            let target = kept.unwrap_or(*level);
            if !satisfied {
                edit::set(self.doc, TY_RULES, rule, Value::from(target.name()));
                let old =
                    effective.map_or_else(|| "unset".to_owned(), |level| level.name().to_owned());
                let mut path = TY_RULES.to_vec();
                path.push(rule);
                let explained = match kept {
                    Some(_) => (
                        accepted
                            .iter()
                            .find(|exception| exception.tool == "ty" && exception.id == *rule)
                            .map(|exception| exception.reason.clone())
                            .unwrap_or_default(),
                        DOCS.to_owned(),
                    ),
                    None => self.explain(Tool::Ty, rule),
                };
                self.change(
                    ("ty", rule),
                    format!("ty {rule}"),
                    &path,
                    old,
                    target.name().to_owned(),
                    explained,
                );
            }
            self.overrides(rule, target);
        }
        self.settings(Tool::Ty, &prefix, &values.ty_settings);
    }

    /// Raises each level below `target` that ty's override tables give `rule`, by its name or by
    /// `all`, in `[[tool.ty.overrides]]` tables and in an inline array of override tables alike.
    /// A rule's own entry wins over `all` in ty, so an override that lowers `all` gets an entry
    /// for the rule at `target`.
    fn overrides(&mut self, rule: &str, target: Level) {
        let catalog = self.catalog;
        let mut raised = Vec::new();
        if let Some(item) =
            edit::existing(self.doc, &["tool", "ty"]).and_then(|table| table.get_mut("overrides"))
        {
            for (index, table) in edit::tables_mut(item) {
                let Some(rules) = table.get_mut("rules").and_then(Item::as_table_like_mut) else {
                    continue;
                };
                let named: Vec<(String, Level)> = rules
                    .iter()
                    .filter_map(|(name, item)| {
                        let level = item.as_str().and_then(Level::parse)?;
                        (catalog.ty_rule(name)?.key.id == rule).then(|| (name.to_owned(), level))
                    })
                    .collect();
                let all = rules
                    .get("all")
                    .and_then(|item| item.as_str())
                    .and_then(Level::parse);
                let lower: Vec<(String, Level)> = if named.is_empty() {
                    all.filter(|all| *all < target)
                        .map(|all| (rule.to_owned(), all))
                        .into_iter()
                        .collect()
                } else {
                    named
                        .into_iter()
                        .filter(|(_, level)| *level < target)
                        .collect()
                };
                for (name, level) in lower {
                    let value = toml_edit::value(target.name());
                    match rules.get_mut(&name) {
                        Some(item) => *item = value,
                        None => {
                            rules.insert(&name, value);
                        }
                    }
                    raised.push((index, name, level));
                }
            }
        }
        for (index, name, level) in raised {
            let path = format!("tool.ty.overrides[{index}].rules.{name}");
            let (reason, source) = self.explain(Tool::Ty, rule);
            self.plan.changes.push(Change {
                tool: "ty",
                id: rule.to_owned(),
                item: format!("ty {rule}"),
                path,
                line: None,
                old: level.name().to_owned(),
                new: target.name().to_owned(),
                reason,
                source,
            });
        }
    }

    /// The pytest settings and plugins of the profile, and the removal of what it forbids, for
    /// the release in `facts`. Returns whether a dependency list changed.
    fn pytest(
        &mut self,
        values: &Values,
        profile: &Profile,
        facts: PytestFacts,
        accepted: &[Exception],
    ) -> Lists {
        let forbid = &profile.forbid;
        let wanted = match facts.release {
            Release::NineOrLater => &values.pytest,
            Release::BeforeNine => &values.pytest_before_nine,
        };
        let excepted = |id: &str| {
            accepted
                .iter()
                .any(|exception| exception.tool == "pytest" && exception.id == id)
        };
        let current = Some(self.current());
        match pytest::in_pyproject(facts.release, self.file, current.as_ref()) {
            Err(error) => {
                let prefix = format!("{}: ", self.file.display());
                let message = error.strip_prefix(&prefix).unwrap_or(&error).to_owned();
                self.plan.ambiguity(PYPROJECT, None, message);
            }
            Ok(settings) => {
                self.overridden(&settings, values, forbid, &excepted, facts);
                self.pytest_settings(&settings, wanted, &excepted);
                self.pytest_addopts(&settings, wanted, forbid, &excepted);
                let prefix = settings.prefix();
                for key in &forbid.settings {
                    if settings.has(key) {
                        self.forbid_key(&settings, prefix, key, key);
                    }
                }
                for key in &forbid.false_settings {
                    let set_by_profile = wanted.settings.iter().any(|(wanted, _)| wanted == key);
                    if settings.flag(key) == Some(false) && !set_by_profile {
                        self.forbid_key(&settings, prefix, key, &format!("{key} = false"));
                    }
                }
            }
        }
        self.plugins(values, profile, facts, &excepted)
    }

    /// Each pytest setting of `wanted` that the table lacks, and no exception keeps out.
    fn pytest_settings(
        &mut self,
        settings: &pytest::Settings,
        wanted: &profile::PytestWanted,
        excepted: &dyn Fn(&str) -> bool,
    ) {
        let prefix = settings.prefix();
        for (key, value) in &wanted.settings {
            if excepted(key) || !settings.lacks(key, value) {
                continue;
            }
            let old = settings
                .destination
                .literal(key)
                .unwrap_or_else(|| "unset".to_owned());
            let new = pytest_value(settings, key, value);
            let shown = toml::Observed::of_value(value).show();
            edit::set(self.doc, prefix, key, new);
            let mut path = prefix.to_vec();
            path.push(key);
            let explained = self.explain(Tool::Pytest, key);
            self.change(
                ("pytest", key),
                format!("pytest {key}"),
                &path,
                old,
                shown,
                explained,
            );
        }
    }

    /// Each argument of `wanted` that `addopts` lacks, and no exception keeps out, and the
    /// removal of each argument that `forbid` removes and `wanted` does not hold.
    fn pytest_addopts(
        &mut self,
        settings: &pytest::Settings,
        wanted: &profile::PytestWanted,
        forbid: &profile::PytestForbid,
        excepted: &dyn Fn(&str) -> bool,
    ) {
        let prefix = settings.prefix();
        let add: Vec<&str> = wanted
            .addopts
            .iter()
            .filter(|arg| !excepted(arg) && settings.missing(arg))
            .map(String::as_str)
            .collect();
        let tokens: Vec<String> = add
            .iter()
            .flat_map(|arg| shlex::split(arg).unwrap_or_else(|| vec![(*arg).to_owned()]))
            .collect();
        let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let kept = |flag: &&String| !wanted.addopts.contains(flag);
        let forbidden: Vec<&str> = forbid
            .addopts
            .iter()
            .filter(kept)
            .map(String::as_str)
            .collect();
        let switches: Vec<&str> = forbid
            .switches
            .iter()
            .filter(kept)
            .map(String::as_str)
            .collect();
        let removed = addopts(
            self.doc,
            prefix,
            &tokens,
            Forbidden {
                flags: &forbidden,
                switches: &switches,
                forbid,
            },
        );
        let mut path = prefix.to_vec();
        path.push("addopts");
        for (flag, text) in removed {
            let explained = self.explain(Tool::Pytest, &flag);
            let item = if forbid.plugins.contains(&flag) {
                format!("pytest plugin {flag}")
            } else {
                format!("pytest {flag}")
            };
            self.change(
                ("pytest", &flag),
                item,
                &path,
                text,
                "removed".to_owned(),
                explained,
            );
        }
        for arg in add {
            let explained = self.explain(Tool::Pytest, arg);
            self.change(
                ("pytest", arg),
                format!("pytest {arg}"),
                &path,
                "absent".to_owned(),
                "present".to_owned(),
                explained,
            );
        }
    }

    /// A conflict for each argument of `addopts` that overrides a setting the profile owns, with
    /// `-o` or `--override-ini`, or that turns off a plugin the profile adds, with `-p no:` or
    /// with `--disable-plugin-autoload` when no `-p` loads it again.
    fn overridden(
        &mut self,
        settings: &pytest::Settings,
        values: &Values,
        forbid: &profile::PytestForbid,
        excepted: &dyn Fn(&str) -> bool,
        facts: PytestFacts,
    ) {
        let owned: BTreeSet<&str> = [&values.pytest, &values.pytest_before_nine]
            .into_iter()
            .flat_map(|wanted| wanted.settings.iter().map(|(key, _)| key.as_str()))
            .chain(forbid.settings.iter().map(String::as_str))
            .chain(forbid.false_settings.iter().map(String::as_str))
            .chain(["addopts"])
            .collect();
        let disabled: BTreeSet<&str> = forbid
            .plugin_names
            .iter()
            .filter(|(plugin, _)| values.plugins.contains(plugin) && !excepted(plugin))
            .flat_map(|(_, names)| names.iter().map(String::as_str))
            .collect();
        let args = settings.args();
        let mut path = settings.prefix().to_vec();
        path.push("addopts");
        let line = self.line(&path);
        let mut index = 0;
        while index < args.len() {
            let arg = args[index];
            if let Some((value, used)) = plugin_arg(&args, index) {
                let shown = args[index..index + used].join(" ");
                if let Some(message) = plugin_conflict(&value, &shown, &disabled, forbid) {
                    self.plan.conflict(PYPROJECT, line, message);
                }
                index += used;
                continue;
            }
            let attached = arg
                .strip_prefix("--override-ini")
                .and_then(|rest| rest.strip_prefix('='))
                .or_else(|| arg.strip_prefix("-o").filter(|rest| !rest.is_empty()));
            let (value, used) = if matches!(arg, "-o" | "--override-ini") {
                (args.get(index + 1).copied(), 2)
            } else {
                (attached, 1)
            };
            if let Some(value) = value {
                let shown = if used == 2 {
                    format!("{arg} {value}")
                } else {
                    arg.to_owned()
                };
                let message = {
                    let key = value.split_once('=').map_or(value, |(key, _)| key).trim();
                    owned.contains(key).then(|| format!("addopts overrides {key} with {shown}, and the profile sets {key}. Remove {shown}, and record an exception where the profile takes one."))
                };
                if let Some(message) = message {
                    self.plan.conflict(PYPROJECT, line, message);
                }
            }
            index += used;
        }
        if facts.uses && args.contains(&"--disable-plugin-autoload") {
            let loaded = |name: &str| {
                (0..args.len())
                    .any(|index| plugin_arg(&args, index).is_some_and(|(value, _)| value == name))
            };
            let unloaded: Vec<&str> = forbid
                .plugin_names
                .iter()
                .filter(|(plugin, _)| values.plugins.contains(plugin) && !excepted(plugin))
                .filter(|(_, names)| !names.iter().any(|name| loaded(name)))
                .map(|(plugin, _)| plugin.as_str())
                .collect();
            if !unloaded.is_empty() {
                self.plan.conflict(
                    PYPROJECT,
                    line,
                    format!("addopts stops pytest from loading plugins with --disable-plugin-autoload, so {} do not run. Remove the flag, or load each with -p.", unloaded.join(" and ")),
                );
            }
        }
    }

    fn forbid_key(&mut self, settings: &pytest::Settings, prefix: &[&str], key: &str, id: &str) {
        let mut path = prefix.to_vec();
        path.push(key);
        let old = settings.destination.literal(key).unwrap_or_default();
        edit::remove(self.doc, prefix, key);
        let explained = self.explain(Tool::Pytest, id);
        self.change(
            ("pytest", id),
            format!("pytest {key}"),
            &path,
            old,
            "removed".to_owned(),
            explained,
        );
    }

    /// Adds the profile's plugins to the list that holds pytest and removes the forbidden ones
    /// from every list. A plugin counts only where uv installs it with pytest: in the list that
    /// holds pytest, or in a group that list includes, since a plugin in an unrelated group
    /// never runs. Returns whether a list changed.
    fn plugins(
        &mut self,
        values: &Values,
        profile: &Profile,
        facts: PytestFacts,
        excepted: &dyn Fn(&str) -> bool,
    ) -> Lists {
        let forbid = &profile.forbid;
        let current = self.current();
        let project = pytest::Project {
            destination: Destination {
                file: self.file.to_path_buf(),
                prefix: &[],
                doc: Some(current),
            },
        };
        let groups = project.groups();
        let runtime = project.runtime_dependencies();
        let lists: Vec<&pytest::Group<'_>> = groups.iter().chain(runtime.iter()).collect();
        let mut holders: Vec<&[&str]> = lists
            .iter()
            .filter(|group| group.has("pytest"))
            .map(|group| group.path.as_slice())
            .collect();
        if holders.is_empty() {
            holders.push(pytest::Project::plugin_group(&groups));
        }
        let mut removals: Vec<(Vec<String>, String, String)> = Vec::new();
        for name in &forbid.plugins {
            for group in &lists {
                for entry in group.entries(name) {
                    if let Some(text) = entry.as_str() {
                        let path = group.path.iter().map(|part| (*part).to_owned()).collect();
                        removals.push((path, name.clone(), text.to_owned()));
                    }
                }
            }
        }
        let mut additions: Vec<(Vec<String>, String)> = Vec::new();
        if facts.uses && facts.lists {
            for holder in holders {
                let beside = pytest::Project::installed_with(&lists, holder);
                for name in &values.plugins {
                    if !excepted(name) && !beside.iter().any(|group| group.installs(name)) {
                        let path = holder.iter().map(|part| (*part).to_owned()).collect();
                        additions.push((path, name.clone()));
                    }
                }
            }
        }
        drop(lists);
        drop(groups);
        let changed = if removals.is_empty() && additions.is_empty() {
            Lists::Unchanged
        } else {
            Lists::Changed
        };
        for (path, name, text) in removals {
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            let (key, table) = path.split_last().expect("a dependency list has a key");
            if let Some(array) = edit::existing(self.doc, table)
                .and_then(|table| table.get_mut(key))
                .and_then(Item::as_array_mut)
            {
                let indexes: Vec<usize> = edit::strings(array)
                    .into_iter()
                    .filter(|(_, entry)| *entry == text)
                    .map(|(index, _)| index)
                    .collect();
                edit::remove_items(array, &indexes);
            }
            let explained = self.explain(Tool::Pytest, &name);
            self.change(
                ("pytest", &name),
                format!("pytest plugin {name}"),
                &path,
                format!("{text:?}"),
                "removed".to_owned(),
                explained,
            );
        }
        for (path, name) in additions {
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            let (key, table) = path.split_last().expect("a dependency list has a key");
            let requirement = profile
                .dev_pins
                .iter()
                .find(|(pinned, _)| *pinned == name)
                .map_or_else(|| name.clone(), |(_, version)| format!("{name}=={version}"));
            if let Some(array) = edit::array(self.doc, table, key) {
                edit::push(array, &requirement);
            }
            let explained = self.explain(Tool::Pytest, &name);
            self.change(
                ("pytest", &name),
                format!("pytest plugin {name}"),
                &path,
                "absent".to_owned(),
                format!("{requirement:?}"),
                explained,
            );
        }
        changed
    }

    /// Pins each tool of `pins` exactly in `[dependency-groups] dev`, replacing a looser
    /// requirement there, and records a conflict for a requirement in another list that the pin
    /// does not meet, since uv locks one release for every list. A kind whose `pyproject.toml`
    /// holds tool settings only gets no pin. Returns whether `dev` changed.
    fn pins(&mut self, pins: &[(String, String)], facts: PytestFacts) -> Lists {
        if !facts.lists {
            return Lists::Unchanged;
        }
        let mut changed = Lists::Unchanged;
        for (name, version) in pins {
            let wanted = format!("{name}=={version}");
            let current = self.current();
            let project = pytest::Project {
                destination: Destination {
                    file: self.file.to_path_buf(),
                    prefix: &[],
                    doc: Some(current),
                },
            };
            let groups = project.groups();
            let runtime = project.runtime_dependencies();
            let mut elsewhere = Vec::new();
            for group in groups.iter().chain(runtime.iter()) {
                if group.is(&DEV) {
                    continue;
                }
                for entry in group.entries(name) {
                    let Some(text) = entry.as_str() else { continue };
                    let meets = version
                        .parse::<crate::version::Version>()
                        .is_ok_and(|pinned| pytest::satisfies(text, &pinned));
                    if !meets {
                        elsewhere.push((dotted(&group.path), text.to_owned()));
                    }
                }
            }
            let in_dev: Vec<String> = groups
                .iter()
                .filter(|group| group.is(&DEV))
                .flat_map(|group| group.entries(name))
                .filter_map(|entry| entry.as_str().map(str::to_owned))
                .collect();
            let dev_key = groups
                .iter()
                .find(|group| group.is(&DEV))
                .and_then(|group| group.path.last())
                .map_or_else(|| DEV[1].to_owned(), |key| (*key).to_owned());
            drop(groups);
            drop(runtime);
            for (list, text) in elsewhere {
                self.plan.conflict(
                    PYPROJECT,
                    None,
                    format!("{list} requires {text:?}, and the profile pins {wanted} in dependency-groups.dev, which uv cannot lock together. Remove {name} from {list}, or let it allow {version}."),
                );
            }
            if in_dev
                .iter()
                .any(|text| pytest::pins_exactly(text, version))
            {
                continue;
            }
            let Some(array) = edit::array(self.doc, &DEV[..1], &dev_key) else {
                continue;
            };
            let indexes: Vec<usize> = edit::strings(array)
                .into_iter()
                .filter(|(_, text)| in_dev.contains(text))
                .map(|(index, _)| index)
                .collect();
            match indexes.first() {
                Some(&first) => {
                    if let Some(old) = array.get_mut(first) {
                        let decor = old.decor().clone();
                        *old = Value::from(wanted.as_str());
                        *old.decor_mut() = decor;
                    }
                    edit::remove_items(array, &indexes[1..]);
                }
                None => edit::push(array, &wanted),
            }
            changed = Lists::Changed;
            let old = if in_dev.is_empty() {
                "absent".to_owned()
            } else {
                in_dev
                    .iter()
                    .map(|text| format!("{text:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let reason = format!(
                "The profile was reviewed for {name} {version}, so the project runs that release with uv run, and uv.lock locks it."
            );
            self.change(
                ("xpd", name),
                format!("pin {name}"),
                &[DEV[0], dev_key.as_str()],
                old,
                format!("{wanted:?}"),
                (reason, DOCS.to_owned()),
            );
        }
        changed
    }

    /// Sets uv's cooldown in `[tool.uv]`, and the Python floor of a `[project]` that names none.
    /// Returns whether either changed, since both change what uv locks.
    fn packaging(&mut self, rules: &packaging::Rules) -> Lists {
        let mut changed = Lists::Unchanged;
        let current = self.current();
        let cooldown = toml::get(&current, &["tool", "uv", "exclude-newer"])
            .and_then(Item::as_value)
            .map(|value| value.to_string().trim().to_owned());
        let wanted = Value::from(rules.cooldown.as_str());
        let wanted_text = wanted.to_string().trim().to_owned();
        if cooldown.as_deref() != Some(wanted_text.as_str())
            && edit::set(self.doc, &["tool", "uv"], "exclude-newer", wanted)
        {
            changed = Lists::Changed;
            self.change(
                ("xpd", "exclude-newer"),
                "uv cooldown".to_owned(),
                &["tool", "uv", "exclude-newer"],
                cooldown.unwrap_or_else(|| "unset".to_owned()),
                wanted_text,
                (
                    format!("uv takes an upload only after {}, which gives a bad release time to be noticed and pulled. For an urgent fix, name the package in exclude-newer-package until the upload ages in.", rules.cooldown),
                    DOCS.to_owned(),
                ),
            );
        }
        let has_project = toml::get(&current, &["project"]).is_some();
        let floor = toml::get(&current, &["project", "requires-python"]).is_some();
        let kind_has_project = self.plan.kind.is_some() && has_project;
        if kind_has_project && !floor {
            let text = format!(">={}", rules.floor);
            if edit::set(
                self.doc,
                &["project"],
                "requires-python",
                Value::from(text.as_str()),
            ) {
                changed = Lists::Changed;
                self.change(
                    ("xpd", "requires-python"),
                    "Python floor".to_owned(),
                    &["project", "requires-python"],
                    "unset".to_owned(),
                    format!("{text:?}"),
                    (
                        format!("A project names the oldest Python it supports, and a new one starts at {}, which is still in bugfix support. Review the floor each year.", rules.floor),
                        DOCS.to_owned(),
                    ),
                );
            }
        }
        changed
    }

    /// Records the revision, the kind and the tools in `[tool.xpd.profile]`.
    fn stamp(&mut self, profile: &Profile, record: &profile::Record) {
        let Some(kind) = self.plan.kind.clone() else {
            return;
        };
        if record.revision == Some(profile.revision)
            && record.kind.as_deref() == Some(kind.as_str())
            && record.scope == self.plan.scope
        {
            return;
        }
        let tools = |scope: &Scope| {
            scope
                .names()
                .map_or_else(String::new, |names| format!(", tools {}", names.join(", ")))
        };

        edit::set(
            self.doc,
            &profile::TABLE,
            "revision",
            Value::from(profile.revision),
        );
        edit::set(
            self.doc,
            &profile::TABLE,
            "kind",
            Value::from(kind.as_str()),
        );
        if let Some(names) = self.plan.scope.names() {
            let names: toml_edit::Array = names.into_iter().collect();
            edit::set(self.doc, &profile::TABLE, "tools", Value::Array(names));
        }
        let old = match (record.revision, &record.kind) {
            (None, None) => "none".to_owned(),
            (revision, kind) => format!(
                "revision {}, kind {}{}",
                revision.map_or_else(|| "none".to_owned(), |revision| revision.to_string()),
                kind.as_deref().unwrap_or("none"),
                tools(&record.scope)
            ),
        };
        let new = format!(
            "revision {}, kind {kind}{}",
            profile.revision,
            tools(&self.plan.scope)
        );
        let reason = "Records the profile revision, the kind and the tools that the settings follow, so a newer xpd can show the next change.".to_owned();
        self.change(
            ("xpd", "profile"),
            "profile record".to_owned(),
            &profile::TABLE,
            old,
            new,
            (reason, DOCS.to_owned()),
        );
    }

    /// Checks that the edited file reaches the profile, and records a conflict where it does not.
    fn verify(&mut self, required: &[String], accepted: &[Exception]) {
        let (enabled, ignored) = self.resolve();
        let whole: BTreeSet<&str> = accepted
            .iter()
            .filter(|exception| exception.tool == "ruff" && exception.files.is_empty())
            .map(|exception| exception.id.as_str())
            .collect();
        for code in required {
            let wanted = !whole.contains(code.as_str());
            if enabled.contains(code) != wanted {
                let cause = ignored
                    .get(code)
                    .map_or_else(String::new, |at| format!(" because of {}", at.text));
                let state = if wanted { "on" } else { "off" };
                self.plan.conflict(
                    PYPROJECT,
                    None,
                    format!("doctor cannot turn {code} {state} by editing {PYPROJECT}{cause}."),
                );
            }
        }
    }
}

/// What the profile removes from `addopts`.
#[derive(Clone, Copy)]
struct Forbidden<'a> {
    /// Flags that take a value.
    flags: &'a [&'a str],
    /// Flags that take none.
    switches: &'a [&'a str],
    forbid: &'a profile::PytestForbid,
}

/// Removes each forbidden flag, with its value, each `-p` that loads a plugin `forbid` removes
/// and each `-p no:` that turns off a plugin it keeps on, from `addopts` of the table at `prefix`,
/// and adds the arguments `add` before a `--`, which ends pytest's options, or else at the end.
/// An `addopts` that needs neither keeps its text. Returns each removed flag or plugin with the
/// text it had.
fn addopts(
    doc: &mut DocumentMut,
    prefix: &[&str],
    add: &[&str],
    forbidden: Forbidden<'_>,
) -> Vec<(String, String)> {
    let legacy = prefix.last() == Some(&"ini_options");
    let Some(table) = edit::existing(doc, prefix) else {
        if !add.is_empty() {
            let value = if legacy {
                Value::from(command_line(add.iter().copied()))
            } else {
                Value::Array(add.iter().copied().collect())
            };
            edit::set(doc, prefix, "addopts", value);
        }
        return Vec::new();
    };
    let Some(value) = table.get_mut("addopts").and_then(Item::as_value_mut) else {
        if !add.is_empty() {
            let value = if legacy {
                Value::from(command_line(add.iter().copied()))
            } else {
                Value::Array(add.iter().copied().collect())
            };
            table.insert("addopts", Item::Value(value));
        }
        return Vec::new();
    };
    if let Some(array) = value.as_array_mut() {
        let tokens: Vec<String> = array
            .iter()
            .map(|value| value.as_str().unwrap_or_default().to_owned())
            .collect();
        let (indexes, removed) = flags(&tokens, forbidden);
        edit::remove_items(array, &indexes);
        let at = edit::strings(array)
            .into_iter()
            .find(|(_, arg)| arg == "--")
            .map_or(array.len(), |(index, _)| index);
        for (offset, arg) in add.iter().enumerate() {
            edit::insert(array, at + offset, arg);
        }
        return removed;
    }
    let Some(text) = value.as_str() else {
        return Vec::new();
    };
    let Some(tokens) = shlex::split(text) else {
        return Vec::new();
    };
    let (indexes, removed) = flags(&tokens, forbidden);
    if indexes.is_empty() && add.is_empty() {
        return removed;
    }
    let kept: Vec<&str> = tokens
        .iter()
        .enumerate()
        .filter(|(index, _)| !indexes.contains(index))
        .map(|(_, token)| token.as_str())
        .collect();
    let split = kept
        .iter()
        .position(|token| *token == "--")
        .unwrap_or(kept.len());
    let (options, positional) = kept.split_at(split);
    let joined = command_line(
        options
            .iter()
            .copied()
            .chain(add.iter().copied())
            .chain(positional.iter().copied()),
    );
    let decor = value.decor().clone();
    *value = Value::from(joined);
    *value.decor_mut() = decor;
    removed
}

/// The indexes of the forbidden flags in `tokens`, each flag of `forbidden.flags` with the value
/// after it or attached to it, of each `-p` that loads a plugin that `forbid` removes, and of each
/// `-p no:` that turns off a plugin of `forbid.disabled`, before a `--`. Each comes with the flag
/// or plugin it removes and the text it had.
fn flags(tokens: &[String], forbidden: Forbidden<'_>) -> (Vec<usize>, Vec<(String, String)>) {
    let Forbidden {
        flags,
        switches,
        forbid,
    } = forbidden;
    let mut indexes = Vec::new();
    let mut removed = Vec::new();
    let mut index = 0;
    while index < tokens.len() && tokens[index] != "--" {
        let token = &tokens[index];
        if let Some((value, used)) = plugin_arg(tokens, index) {
            let value = value.trim_start_matches('=');
            let id = match value.strip_prefix("no:") {
                Some(name) if forbid.disabled.iter().any(|plugin| plugin == name) => {
                    Some(profile::PytestForbid::disable_id(name))
                }
                Some(_) => None,
                None => forbid.loaded_by(value).map(str::to_owned),
            };
            if let Some(id) = id {
                indexes.extend(index..index + used);
                removed.push((id, tokens[index..index + used].join(" ")));
            }
            index += used;
            continue;
        }
        if let Some(switch) = switches.iter().find(|switch| *token == **switch) {
            indexes.push(index);
            removed.push(((*switch).to_owned(), token.clone()));
        } else if let Some(flag) = flags.iter().find(|flag| *token == **flag) {
            indexes.push(index);
            let value = tokens.get(index + 1).filter(|next| {
                next.as_str() != "--" && (!next.starts_with('-') || next.parse::<f64>().is_ok())
            });
            if let Some(value) = value {
                indexes.push(index + 1);
                removed.push(((*flag).to_owned(), format!("{flag} {value}")));
                index += 1;
            } else {
                removed.push(((*flag).to_owned(), (*flag).to_owned()));
            }
        } else if let Some(flag) = flags.iter().find(|flag| {
            token.strip_prefix(**flag).is_some_and(|rest| {
                rest.starts_with('=') || (!flag.starts_with("--") && !rest.is_empty())
            })
        }) {
            indexes.push(index);
            removed.push(((*flag).to_owned(), token.clone()));
        }
        index += 1;
    }
    (indexes, removed)
}

/// The plugin argument of the `-p` at `index` of `args`, with the number of arguments it takes,
/// as pytest 9 reads it in `PytestPluginManager.consider_preparse`: `-p` takes the next argument,
/// and `-p<value>` the rest of its own, each with spaces stripped. So `-p=name` names the
/// plugin `=name`, which pytest cannot import. `None` when the argument is no `-p`.
fn plugin_arg<S: AsRef<str>>(args: &[S], index: usize) -> Option<(String, usize)> {
    let arg = args.get(index)?.as_ref();
    if arg == "-p" {
        let value = args.get(index + 1)?.as_ref();
        return Some((value.trim().to_owned(), 2));
    }
    arg.strip_prefix("-p")
        .map(|value| (value.trim().to_owned(), 1))
}

/// Why the `-p` with the plugin argument `value`, written `shown`, keeps the project from the
/// profile: it turns off a plugin of the profile, whose load names are `disabled`, or pytest
/// cannot import what it names. A load of a plugin that `forbid` removes is no conflict,
/// because the plan removes it.
fn plugin_conflict(
    value: &str,
    shown: &str,
    disabled: &BTreeSet<&str>,
    forbid: &profile::PytestForbid,
) -> Option<String> {
    if let Some(name) = value.strip_prefix("no:") {
        return disabled.contains(name).then(|| format!("addopts turns off the plugin {name} with {shown}, and the profile adds it. Remove {shown}, or record a pytest exception for the plugin."));
    }
    if forbid.loaded_by(value.trim_start_matches('=')).is_some() {
        return None;
    }
    let broken = value.is_empty() || value.starts_with('=');
    broken.then(|| format!("addopts passes {shown}, so pytest tries to import the plugin {value:?} and stops with an error. Write -p and the plugin's name, without ="))
}

/// The value that the pytest setting `key` gets for the profile's `value`: the value itself, or
/// for a list, the project's items with each of the profile's that it lacks. `filterwarnings`
/// gets them first, since pytest applies its last matching filter, so the project's own filters
/// keep their effect after them.
fn pytest_value(settings: &pytest::Settings, key: &str, value: &Value) -> Value {
    let Some(wanted) = value.as_array() else {
        return value.clone();
    };
    let held = settings.lines(key);
    let missing: Vec<&str> = wanted
        .iter()
        .filter_map(Value::as_str)
        .filter(|item| !held.iter().any(|line| pytest::same_line(key, line, item)))
        .collect();
    let items: Vec<&str> = if key == "filterwarnings" {
        missing
            .iter()
            .copied()
            .chain(held.iter().map(String::as_str))
            .collect()
    } else {
        held.iter()
            .map(String::as_str)
            .chain(missing.iter().copied())
            .collect()
    };
    Value::Array(items.into_iter().collect())
}

/// `args` as one command line, each quoted only when it needs quotes to stay one argument.
fn command_line<'a>(args: impl Iterator<Item = &'a str>) -> String {
    args.map(|arg| {
        let plain = !arg.is_empty()
            && !arg
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\'));
        if plain {
            arg.to_owned()
        } else {
            shlex::try_quote(arg).map_or_else(|_| arg.to_owned(), std::borrow::Cow::into_owned)
        }
    })
    .collect::<Vec<_>>()
    .join(" ")
}

pub(crate) fn dotted(path: &[&str]) -> String {
    path.iter()
        .map(|part| {
            if part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            {
                (*part).to_owned()
            } else {
                format!("{part:?}")
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}
