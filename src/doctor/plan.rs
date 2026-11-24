//! The difference between a project and the profile: the edits that bring its `pyproject.toml`
//! to the profile, the lock files its kind needs, the conflicts that no edit of
//! `pyproject.toml` resolves, and the state of each exception the project records.
//!
//! Doctor keeps the whole setup in one place per tool, the tool's table of `pyproject.toml`
//! at the project root. A file that a tool reads before that table, a nested Ruff
//! configuration and an `extend` of another file are conflicts. Every value is compared by its
//! effect, from the pinned releases' rules and defaults, without running a tool.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Value};

use super::catalog::{Catalog, Level, Tool};
use super::edit;
use super::placement::Destination;
use super::profile::{self, Exception, LockStyle, Presence, Profile, Values};
use super::pytest::{self, Release};
use super::ruff;
use super::ty;
use crate::toml::{self, Toml};

/// The file that holds the profile's settings.
pub(crate) const PYPROJECT: &str = "pyproject.toml";
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

/// What the caller asks of the plan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Request<'a> {
    /// The kind named on the command line, which replaces the recorded one.
    pub(crate) kind: Option<&'a str>,
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
    pub(crate) file: PathBuf,
    pub(crate) changes: Vec<Change>,
    pub(crate) conflicts: Vec<Conflict>,
    pub(crate) exceptions: Vec<ExceptionState>,
    pub(crate) locks: Vec<Lock>,
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
    let before = match fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", file.display())),
    };
    let original = before
        .as_ref()
        .map(|text| toml_edit::Document::parse(text.clone()))
        .transpose()
        .map_err(|error| anyhow::anyhow!("{PYPROJECT}: {}", error.to_string().trim()))?;
    let record = original
        .as_ref()
        .map(profile::record)
        .transpose()?
        .flatten()
        .unwrap_or_default();
    let suggestion = suggest_kind(original.as_ref(), root, files);
    let (kind, kind_from) = match (request.kind, &record.kind) {
        (Some(kind), _) => (Some(kind.to_owned()), Some("flag")),
        (None, Some(kind)) => (Some(kind.clone()), Some("record")),
        (None, None) => (
            suggestion.as_ref().map(|(kind, _)| (*kind).to_owned()),
            suggestion.as_ref().map(|_| "suggestion"),
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
        file: PathBuf::from(PYPROJECT),
        changes: Vec::new(),
        conflicts: Vec::new(),
        exceptions: Vec::new(),
        locks: Vec::new(),
        diff: String::new(),
        before: before.clone(),
        after: String::new(),
        owned: Vec::new(),
    };
    let facts = pytest_facts(root, original.as_ref());
    let values = profile.values(plan.kind.as_deref());
    plan.locations(root, original.as_ref(), profile);
    plan.layout(original.as_ref(), profile, files);
    let accepted = plan.judge(&values, &profile.forbid, facts, &record.exceptions);
    let mut edits = Edits {
        doc: &mut doc,
        original: original.as_ref(),
        file: &file,
        catalog,
        plan: &mut plan,
    };
    edits.ruff(&values, &accepted);
    edits.ty(&values, &accepted);
    let dependencies = edits.pytest(&values, &profile.forbid, facts, &accepted);
    if request.stamp {
        edits.stamp(profile, &record);
    }
    edits.verify(&values, &accepted);
    plan.after = doc.to_string();
    plan.owned_files(root, profile)?;
    plan.locks(root, profile, files, dependencies);
    let mut text = diff(PYPROJECT, before.as_deref(), &plan.after);
    for owned in &plan.owned {
        let name = owned.path.display().to_string();
        text.push_str(&diff(&name, owned.before.as_deref(), &owned.after));
    }
    plan.diff = text;
    Ok(plan)
}

/// The pytest release a fresh checkout of the project at `root` gets: the one `uv.lock` locks,
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
    let release = if let Some(locked) = pytest::locked(root) {
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
    }
}

impl Plan {
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
            .map(|lock| format!("{}: {}", lock.file.display(), lock.state));
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
                (Presence::Forbidden, true) => self.conflict(
                    PYPROJECT,
                    None,
                    format!("The kind {name} keeps {PYPROJECT} to tool settings, and it has {part}. Move the metadata into the inline metadata of the scripts, or name the kind that fits."),
                ),
                _ => {}
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

    /// The owned files of the kind that differ from their templates, each with a change.
    fn owned_files(&mut self, root: &Path, profile: &Profile) -> Result<()> {
        for owned in profile.owned(self.kind.as_deref()) {
            let path = PathBuf::from(&owned.path);
            let file = root.join(&path);
            let before = match fs::read_to_string(&file) {
                Ok(text) => Some(text),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(error).with_context(|| format!("cannot read {}", file.display()));
                }
            };
            if before.as_deref() == Some(owned.text) {
                continue;
            }
            self.changes.push(Change {
                tool: "xpd",
                id: owned.name.clone(),
                item: format!("owned file {}", owned.path),
                path: owned.path.clone(),
                line: None,
                old: if before.is_some() { "differs" } else { "missing" }.to_owned(),
                new: "the profile's copy".to_owned(),
                reason: "The profile owns this file, so the project keeps an exact copy of its template.".to_owned(),
                source: DOCS.to_owned(),
            });
            self.owned.push(OwnedFile {
                path,
                before,
                after: owned.text.to_owned(),
            });
        }
        Ok(())
    }

    /// The files that keep a tool's settings outside the tool's table in `pyproject.toml`.
    fn locations(&mut self, root: &Path, doc: Option<&Toml>, profile: &Profile) {
        for name in [".ruff.toml", "ruff.toml"] {
            if root.join(name).is_file() {
                self.conflict(
                    name,
                    None,
                    format!("Ruff reads {name} instead of {PYPROJECT}. Move its settings into [tool.ruff] of {PYPROJECT} and delete it."),
                );
            }
        }
        if root.join("ty.toml").is_file() {
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
            if holds {
                self.conflict(
                    name,
                    None,
                    format!("{name} holds pytest settings. Move them into the pytest table of {PYPROJECT} and remove them from {name}."),
                );
            }
        }
        if let Some(doc) = doc {
            if let Some(base) = toml::get(doc, &["tool", "ruff", "extend"]).and_then(Item::as_str) {
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
        for nested in nested_ruff(root) {
            let shown = nested.strip_prefix(root).unwrap_or(&nested).to_path_buf();
            self.conflict(
                shown.clone(),
                None,
                format!("Ruff reads {} for the files below it instead of {PYPROJECT} at the root. Move its settings into [tool.ruff] of the root {PYPROJECT}, with per-file-ignores for its files.", shown.display()),
            );
        }
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
        forbid: &profile::PytestForbid,
        facts: PytestFacts,
        exceptions: &[Exception],
    ) -> Vec<Exception> {
        let mut accepted: Vec<Exception> = Vec::new();
        for exception in exceptions {
            let (status, message) = match verdict(values, forbid, facts, exception, &accepted) {
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
                });
            }
            LockStyle::Script => {
                for script in scripts(files) {
                    let shown = script.strip_prefix(root).unwrap_or(&script).to_path_buf();
                    let mut lock = shown.clone().into_os_string();
                    lock.push(".lock");
                    let file = PathBuf::from(lock);
                    let state = if root.join(&file).is_file() {
                        "present"
                    } else {
                        "missing"
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
                    });
                }
            }
        }
    }

    /// Marks each present lock that `uv lock --check` finds out of date as one to refresh. The
    /// rest of the plan reads files only, so this is the one step that runs a tool.
    ///
    /// # Errors
    ///
    /// Returns an error if uv cannot run or cannot tell whether a lock is up to date.
    pub(crate) fn check_locks(&mut self, root: &Path) -> Result<()> {
        for lock in self.locks.iter_mut().filter(|lock| lock.state == "present") {
            let command = lock.command.join(" ");
            let (program, args) = lock.command.split_first().context("a lock has a command")?;
            let output = Command::new(program)
                .args(args)
                .arg("--check")
                .current_dir(root)
                .stdin(Stdio::null())
                .output()
                .with_context(|| format!("cannot run {command} --check"))?;
            if output.status.success() {
                continue;
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("needs to be updated") {
                bail!(
                    "{command} --check could not check {}: {}",
                    lock.file.display(),
                    stderr.trim()
                );
            }
            lock.state = "refresh";
        }
        Ok(())
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
        let _ = writeln!(out, "Profile revision {}, {kind}", self.revision);
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
                        "  {} is out of date. --apply runs {command}.",
                        lock.file.display()
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
        let _ = if !self.drifts() {
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
    forbid: &profile::PytestForbid,
    facts: PytestFacts,
    exception: &Exception,
    accepted: &[Exception],
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
    if tool == Tool::Pytest && forbid.forbids(id) {
        return invalid(format!(
            "the profile removes {id} from every project, and no exception keeps it"
        ));
    }
    match tool {
        Tool::Ruff if !values.ruff_rules.iter().any(|code| code == id) => {
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

/// The Ruff configuration files below the root, outside virtual environments.
fn nested_ruff(root: &Path) -> Vec<PathBuf> {
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
        .filter(
            |path| match path.file_name().and_then(|name| name.to_str()) {
                Some("ruff.toml" | ".ruff.toml") => true,
                Some(PYPROJECT) => toml::read(path)
                    .ok()
                    .flatten()
                    .is_some_and(|doc| toml::get(&doc, &["tool", "ruff"]).is_some()),
                _ => false,
            },
        )
        .collect()
}

/// The unified diff of the file `name` from `before` to `after`, empty when they are the same.
fn diff(name: &str, before: Option<&str>, after: &str) -> String {
    let old = before.unwrap_or_default();
    if before.is_some() && old == after {
        return String::new();
    }
    let old_name = if before.is_some() { name } else { "/dev/null" };
    similar::TextDiff::from_lines(old, after)
        .unified_diff()
        .context_radius(3)
        .header(old_name, name)
        .to_string()
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

    /// Every rule of the profile runs, outside the files and rules that exceptions name.
    fn ruff(&mut self, values: &Values, accepted: &[Exception]) {
        let ruff_exceptions = || accepted.iter().filter(|exception| exception.tool == "ruff");
        let whole: BTreeSet<&str> = ruff_exceptions()
            .filter(|exception| exception.files.is_empty())
            .map(|exception| exception.id.as_str())
            .collect();
        let required: Vec<&String> = values
            .ruff_rules
            .iter()
            .filter(|code| !whole.contains(code.as_str()))
            .collect();
        let (enabled, ignored) = self.resolve();
        let mut off = Vec::new();
        for code in &required {
            if enabled.contains(*code) {
                continue;
            }
            let removed = self.unselect(code, &["ignore", "extend-ignore"]);
            let old = ignored
                .get(*code)
                .map_or_else(|| "off".to_owned(), |at| format!("off, by {}", at.text));
            off.push(((*code).clone(), old, removed));
        }
        let (enabled, _) = self.resolve();
        for (code, old, removed) in off {
            let path: Vec<String> = if enabled.contains(&code) {
                removed.into_iter().next().unwrap_or_default()
            } else {
                if let Some(array) = edit::array(self.doc, RUFF_LINT, "extend-select") {
                    edit::push(array, &code);
                }
                ["tool", "ruff", "lint", "extend-select"]
                    .map(str::to_owned)
                    .to_vec()
            };
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            let item = self.rule_name(&code);
            let explained = self.explain(Tool::Ruff, &code);
            self.change(
                ("ruff", &code),
                item,
                &path,
                old,
                "on".to_owned(),
                explained,
            );
        }
        self.per_file(&required, accepted);
        for exception in ruff_exceptions().filter(|exception| exception.files.is_empty()) {
            let (enabled, _) = self.resolve();
            if !enabled.contains(&exception.id) {
                continue;
            }
            let removed = self.unselect(&exception.id, &["select", "extend-select"]);
            let (enabled, _) = self.resolve();
            let path: Vec<String> = if enabled.contains(&exception.id) {
                if let Some(array) = edit::array(self.doc, RUFF_LINT, "ignore") {
                    edit::push(array, &exception.id);
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

    /// Per-file ignores of the profile's rules go, except those that exceptions record, and each
    /// exception for some files gets its per-file ignore.
    fn per_file(&mut self, required: &[&String], accepted: &[Exception]) {
        let scoped: BTreeSet<(&str, &str)> = accepted
            .iter()
            .filter(|exception| exception.tool == "ruff")
            .flat_map(|exception| {
                exception
                    .files
                    .iter()
                    .map(|pattern| (exception.id.as_str(), pattern.as_str()))
            })
            .collect();
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
                    let drop = self.per_file_pattern(
                        &path,
                        &pattern,
                        &selectors,
                        required,
                        &scoped,
                        &mut covered,
                    );
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
        for exception in accepted.iter().filter(|exception| exception.tool == "ruff") {
            for pattern in &exception.files {
                if covered.contains(&(exception.id.clone(), pattern.clone())) {
                    continue;
                }
                let mut path: Vec<&str> = RUFF_LINT.to_vec();
                path.push("per-file-ignores");
                if let Some(array) = edit::array(self.doc, &path, pattern) {
                    edit::push(array, &exception.id);
                }
                path.push(pattern);
                let item = self.rule_name(&exception.id);
                let explained = (exception.reason.clone(), DOCS.to_owned());
                self.change(
                    ("ruff", &exception.id),
                    item,
                    &path,
                    "on".to_owned(),
                    format!("off for {pattern}, by an exception"),
                    explained,
                );
            }
        }
    }

    /// What the per-file ignore `pattern` of the table at `path`, with its `selectors`, does to
    /// the `required` rules. Returns the indexes of the selectors to drop, records a conflict for
    /// a selector that turns a rule off as part of a group, and adds each pattern that an
    /// exception keeps to `covered`.
    fn per_file_pattern(
        &mut self,
        path: &[&str],
        pattern: &str,
        selectors: &[(usize, String)],
        required: &[&String],
        scoped: &BTreeSet<(&str, &str)>,
        covered: &mut BTreeSet<(String, String)>,
    ) -> Vec<usize> {
        let mut drop = Vec::new();
        let mut full = path.to_vec();
        full.push(pattern);
        for code in required {
            let Some(rule) = self.catalog.ruff_rules().find(|rule| rule.key.id == **code) else {
                continue;
            };
            let matching: Vec<&(usize, String)> = selectors
                .iter()
                .filter(|(_, selector)| {
                    self.catalog
                        .redirect(selector)
                        .is_some_and(|selector| ruff::selects(selector, rule))
                })
                .collect();
            if matching.is_empty() {
                continue;
            }
            if scoped.contains(&(code.as_str(), pattern)) {
                covered.insert(((*code).clone(), pattern.to_owned()));
                continue;
            }
            let mut exact = false;
            for (index, selector) in matching {
                if self.catalog.redirect(selector) == Some(code.as_str()) {
                    drop.push(*index);
                    exact = true;
                } else {
                    let line = self.line(&full);
                    self.plan.conflict(
                        PYPROJECT,
                        line,
                        format!("{} turns {code} off for {pattern} through {selector}. Record a Ruff exception for {code} and {pattern}, or name codes that leave {code} on.", dotted(path)),
                    );
                }
            }
            if exact {
                let item = self.rule_name(code);
                let explained = self.explain(Tool::Ruff, code);
                self.change(
                    ("ruff", code),
                    item,
                    &full,
                    format!("off for {pattern}"),
                    "on".to_owned(),
                    explained,
                );
            }
        }
        drop
    }

    /// Every ty rule of the profile has at least its level, or the level its exception keeps.
    fn ty(&mut self, values: &Values, accepted: &[Exception]) {
        let prefix = ["tool", "ty"];
        let state = ty::state(self.catalog, self.file, &prefix, Some(&self.current()));
        for (rule, level) in &values.ty_rules {
            let kept = accepted
                .iter()
                .find(|exception| exception.tool == "ty" && exception.id == *rule)
                .and_then(|exception| exception.value.as_deref().and_then(Level::parse));
            let set = state.rules.get(rule).or(state.all.as_ref());
            let effective = set.map(|set| set.level).or_else(|| {
                self.catalog
                    .ty_rule(rule)
                    .and_then(|record| record.default.as_deref())
                    .and_then(Level::parse)
            });
            let satisfied = match (kept, effective) {
                (Some(kept), Some(effective)) => effective == kept,
                (None, Some(effective)) => effective >= *level,
                (_, None) => false,
            };
            let target = kept.unwrap_or(*level);
            if !satisfied {
                edit::set(self.doc, TY_RULES, rule, Value::from(target.name()));
                let old = match (effective, set) {
                    (Some(effective), Some(_)) => effective.name().to_owned(),
                    (Some(effective), None) => format!("{} by default", effective.name()),
                    (None, _) => "unknown".to_owned(),
                };
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
    }

    /// Raises each level below `target` that ty's override tables give `rule`, by its name or by
    /// `all`. A rule's own entry wins over `all` in ty, so an override that lowers `all` gets an
    /// entry for the rule at `target`.
    fn overrides(&mut self, rule: &str, target: Level) {
        let catalog = self.catalog;
        let mut raised = Vec::new();
        if let Some(tables) = edit::existing(self.doc, &["tool", "ty"])
            .and_then(|table| table.get_mut("overrides"))
            .and_then(Item::as_array_of_tables_mut)
        {
            for (index, table) in tables.iter_mut().enumerate() {
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
        forbid: &profile::PytestForbid,
        facts: PytestFacts,
        accepted: &[Exception],
    ) -> Lists {
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
                self.overridden(&settings, values, forbid, &excepted);
                let prefix = settings.prefix();
                for (key, value) in &wanted.settings {
                    if excepted(key) || !settings.missing(key) {
                        continue;
                    }
                    let old = settings
                        .flag(key)
                        .map_or_else(|| "unset".to_owned(), |flag| flag.to_string());
                    edit::set(self.doc, prefix, key, Value::from(*value));
                    let mut path = prefix.to_vec();
                    path.push(key);
                    let explained = self.explain(Tool::Pytest, key);
                    self.change(
                        ("pytest", key),
                        format!("pytest {key}"),
                        &path,
                        old,
                        value.to_string(),
                        explained,
                    );
                }
                let add: Vec<&str> = wanted
                    .addopts
                    .iter()
                    .filter(|arg| !excepted(arg) && settings.missing(arg))
                    .map(String::as_str)
                    .collect();
                let forbidden: Vec<&str> = forbid.addopts.iter().map(String::as_str).collect();
                let removed = addopts(self.doc, prefix, &add, &forbidden);
                let mut path = prefix.to_vec();
                path.push("addopts");
                for (flag, text) in removed {
                    let explained = self.explain(Tool::Pytest, &flag);
                    self.change(
                        ("pytest", &flag),
                        format!("pytest {flag}"),
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
        self.plugins(values, forbid, facts, &excepted)
    }

    /// A conflict for each argument of `addopts` that overrides a setting the profile owns, with
    /// `-o` or `--override-ini`, or that turns off a plugin the profile adds, with `-p no:`.
    fn overridden(
        &mut self,
        settings: &pytest::Settings,
        values: &Values,
        forbid: &profile::PytestForbid,
        excepted: &dyn Fn(&str) -> bool,
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
            let attached = |short: &str, long: &str| {
                arg.strip_prefix(long)
                    .and_then(|rest| rest.strip_prefix('='))
                    .or_else(|| arg.strip_prefix(short).filter(|rest| !rest.is_empty()))
            };
            let (value, used) = if matches!(arg, "-o" | "--override-ini" | "-p") {
                (args.get(index + 1).copied(), 2)
            } else {
                (
                    attached("-o", "--override-ini").or_else(|| attached("-p", "-p")),
                    1,
                )
            };
            let plugin_flag = arg == "-p" || (arg.starts_with("-p") && !arg.starts_with("--"));
            if let Some(value) = value {
                let shown = if used == 2 {
                    format!("{arg} {value}")
                } else {
                    arg.to_owned()
                };
                let message = if plugin_flag {
                    value
                        .strip_prefix("no:")
                        .filter(|name| disabled.contains(name))
                        .map(|name| format!("addopts turns off the plugin {name} with {shown}, and the profile adds it. Remove {shown}, or record a pytest exception for the plugin."))
                } else {
                    let key = value.split_once('=').map_or(value, |(key, _)| key).trim();
                    owned.contains(key).then(|| format!("addopts overrides {key} with {shown}, and the profile sets {key}. Remove {shown}, and record an exception where the profile takes one."))
                };
                if let Some(message) = message {
                    self.plan.conflict(PYPROJECT, line, message);
                }
            }
            index += used;
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
    /// from every list. Returns whether a list changed.
    fn plugins(
        &mut self,
        values: &Values,
        forbid: &profile::PytestForbid,
        facts: PytestFacts,
        excepted: &dyn Fn(&str) -> bool,
    ) -> Lists {
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
        let has = |name: &str| lists.iter().any(|group| group.has(name));
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
        let additions: Vec<String> = if facts.uses {
            values
                .plugins
                .iter()
                .filter(|name| !excepted(name) && !has(name))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let holder = groups
            .iter()
            .chain(runtime.iter())
            .find(|group| group.has("pytest"))
            .map(|group| group.path.as_slice());
        let target: Vec<String> = holder
            .unwrap_or_else(|| pytest::Project::plugin_group(&groups))
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
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
        let path: Vec<&str> = target.iter().map(String::as_str).collect();
        let (key, table) = path.split_last().expect("a dependency list has a key");
        for name in additions {
            if let Some(array) = edit::array(self.doc, table, key) {
                edit::push(array, &name);
            }
            let explained = self.explain(Tool::Pytest, &name);
            self.change(
                ("pytest", &name),
                format!("pytest plugin {name}"),
                &path,
                "absent".to_owned(),
                format!("{name:?}"),
                explained,
            );
        }
        changed
    }

    /// Records the revision and kind in `[tool.xpd.profile]`.
    fn stamp(&mut self, profile: &Profile, record: &profile::Record) {
        let Some(kind) = self.plan.kind.clone() else {
            return;
        };
        if record.revision == Some(profile.revision)
            && record.kind.as_deref() == Some(kind.as_str())
        {
            return;
        }
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
        let old = match (record.revision, &record.kind) {
            (None, None) => "none".to_owned(),
            (revision, kind) => format!(
                "revision {}, kind {}",
                revision.map_or_else(|| "none".to_owned(), |revision| revision.to_string()),
                kind.as_deref().unwrap_or("none")
            ),
        };
        let new = format!("revision {}, kind {kind}", profile.revision);
        let reason = "Records the profile revision and kind that the settings follow, so a newer xpd can show the next change.".to_owned();
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
    fn verify(&mut self, values: &Values, accepted: &[Exception]) {
        let (enabled, ignored) = self.resolve();
        let whole: BTreeSet<&str> = accepted
            .iter()
            .filter(|exception| exception.tool == "ruff" && exception.files.is_empty())
            .map(|exception| exception.id.as_str())
            .collect();
        for code in &values.ruff_rules {
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

/// Removes each flag of `remove`, with its value, from `addopts` of the table at `prefix`, and
/// appends `add`. Returns each removed flag with the text it had.
fn addopts(
    doc: &mut DocumentMut,
    prefix: &[&str],
    add: &[&str],
    remove: &[&str],
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
        let (indexes, removed) = flags(&tokens, remove);
        edit::remove_items(array, &indexes);
        for arg in add {
            edit::push(array, arg);
        }
        return removed;
    }
    let Some(text) = value.as_str() else {
        return Vec::new();
    };
    let Some(tokens) = shlex::split(text) else {
        return Vec::new();
    };
    let (indexes, removed) = flags(&tokens, remove);
    let kept = tokens
        .iter()
        .enumerate()
        .filter(|(index, _)| !indexes.contains(index))
        .map(|(_, token)| token.as_str())
        .chain(add.iter().copied());
    let joined = command_line(kept);
    let decor = value.decor().clone();
    *value = Value::from(joined);
    *value.decor_mut() = decor;
    removed
}

/// The indexes of the `flags` in `tokens`, each with the value after it, before a `--`, and each
/// flag with the text it had.
fn flags(tokens: &[String], flags: &[&str]) -> (Vec<usize>, Vec<(String, String)>) {
    let mut indexes = Vec::new();
    let mut removed = Vec::new();
    let mut index = 0;
    while index < tokens.len() && tokens[index] != "--" {
        let token = &tokens[index];
        if let Some(flag) = flags.iter().find(|flag| *token == **flag) {
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
            token
                .strip_prefix(**flag)
                .is_some_and(|rest| rest.starts_with('='))
        }) {
            indexes.push(index);
            removed.push(((*flag).to_owned(), token.clone()));
        }
        index += 1;
    }
    (indexes, removed)
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

fn dotted(path: &[&str]) -> String {
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
