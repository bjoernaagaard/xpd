//! Weighs each suggestion of the catalog against the project's effective settings and its code,
//! and prints what is left: the edits worth making, the project's deliberate choices that
//! doctor keeps, and what it could not establish.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::catalog::{Catalog, Kind, Record, Satisfied, Suggestion, Target, Tool, Trigger, When};
use super::evidence::{Evidence, Location};
use super::placement::{Destination, Placement};
use super::probe::Checker;
use super::pytest::{self, Project, Pytest};
use super::ruff::{self, Ruff};
use super::ty::{self, Ty};
use crate::toml::Observed;

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) tools: Vec<ToolStatus>,
    pub(crate) suggestions: Vec<Advice>,
    pub(crate) policies: Vec<Policy>,
    pub(crate) notes: Vec<Note>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ToolStatus {
    tool: &'static str,
    pinned: &'static str,
    installed: Option<String>,
    path: Option<PathBuf>,
}

/// One edit to make, or one command to run.
#[derive(Debug, Serialize)]
pub(crate) struct Advice {
    record: String,
    tool: &'static str,
    id: String,
    name: Option<String>,
    decision: &'static str,
    /// The file to edit, which may not exist yet. `None` for a command.
    file: Option<PathBuf>,
    file_exists: bool,
    table: String,
    table_line: Option<usize>,
    key: String,
    key_line: Option<usize>,
    value: String,
    current: Option<String>,
    edit: String,
    /// The configuration whose files the suggestion affects.
    scope: String,
    files: usize,
    reason: String,
    /// Where the reviewed decision's rule or setting is documented for the pinned release.
    source: String,
    /// The release that gave a rule its current status.
    since: Option<String>,
    condition: Option<String>,
    evidence: Vec<Location>,
    status: String,
    #[serde(skip)]
    arguments: Option<(Placement, pytest::ArgumentStyle)>,
    #[serde(skip)]
    assign: Assign,
}

/// A deliberate choice of the project that doctor keeps.
#[derive(Debug, Serialize)]
pub(crate) struct Policy {
    record: String,
    tool: &'static str,
    id: String,
    detail: String,
    file: PathBuf,
    line: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct Note {
    tool: &'static str,
    message: String,
}

/// The facts one suggestion is weighed against.
struct Context<'a> {
    root: &'a Path,
    files: &'a [PathBuf],
    evidence: &'a Evidence,
    ruff: &'a Ruff,
    ruff_state: Option<&'a ruff::State>,
    ty: &'a Ty,
}

impl Context<'_> {
    /// What shows that `trigger` applies to the files of this context.
    fn found(&self, trigger: Trigger, requires: &[String]) -> Vec<Location> {
        let state = self.ruff_state;
        match trigger {
            Trigger::PreviewBroadSelect => state
                .and_then(|s| s.broad_preview.clone())
                .into_iter()
                .collect(),
            Trigger::RuleActive => state
                .map(|s| {
                    requires
                        .iter()
                        .filter(|code| s.enabled.contains(*code))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .filter(|active| !active.is_empty())
                .map(|active| Location {
                    file: self.files.first().cloned().unwrap_or_default(),
                    line: 1,
                    text: format!("Ruff runs {} here", active.join(", ")),
                })
                .into_iter()
                .collect(),
            Trigger::DeclaredDependencies => self.ty.dependencies.clone().into_iter().collect(),
            Trigger::RuffAvailable => state
                .and_then(|s| s.verified_with.as_ref())
                .map(|file| Location {
                    file: self.root.join(file),
                    line: 1,
                    text: "doctor read Ruff's settings for this file".to_owned(),
                })
                .into_iter()
                .collect(),
            Trigger::RuffSkipsPythonFile => self
                .ruff
                .skipped
                .iter()
                .filter(|location| self.files.binary_search(&location.file).is_ok())
                .cloned()
                .collect(),
            code => self.evidence.within(code, self.files).cloned().collect(),
        }
    }

    /// The evidence for a suggestion, or `None` when its trigger or its required rules are
    /// missing here.
    fn applies(&self, suggestion: &Suggestion) -> Option<Vec<Location>> {
        if !suggestion.requires.is_empty() {
            let enabled = &self.ruff_state?.enabled;
            if !suggestion
                .requires
                .iter()
                .any(|code| enabled.contains(code))
            {
                return None;
            }
        }
        match &suggestion.when {
            When::Always => Some(Vec::new()),
            When::If { trigger, .. } => {
                let found = self.found(*trigger, &suggestion.requires);
                (!found.is_empty()).then_some(found)
            }
        }
    }
}

/// What one record comes to in one scope.
enum Outcome {
    /// Already in effect, needs preview mode that is off, or deliberately turned off.
    Nothing,
    /// The installed release lacks the rule.
    Unsupported(String),
    /// Suggest it. The value in effect now, when there is one.
    Suggest(Option<Observed>),
}

struct Verdict {
    outcome: Outcome,
    /// The project's deliberate choices that touch the record, each with its place.
    kept: Vec<(String, Location)>,
}

impl Verdict {
    fn nothing() -> Self {
        Self::of(Outcome::Nothing)
    }

    fn of(outcome: Outcome) -> Self {
        Self {
            outcome,
            kept: Vec::new(),
        }
    }
}

/// The facts of the scope that a suggestion is written for.
#[derive(Clone)]
struct Scope<'a> {
    destination: &'a Destination,
    name: String,
    files: usize,
    status: String,
    /// The file `ruff check --show-settings` ran for.
    representative: Option<&'a Path>,
    placement: Option<&'a Placement>,
}

pub(crate) fn evaluate(
    root: &Path,
    files: &[PathBuf],
    catalog: &Catalog,
    evidence: &Evidence,
    ruff: &Ruff,
    ty: &Ty,
) -> Report {
    let mut report = Report {
        tools: [&ruff.checker, &ty.checker]
            .into_iter()
            .map(tool_status)
            .collect(),
        suggestions: Vec::new(),
        policies: Vec::new(),
        notes: Vec::new(),
    };
    let mut unsupported = BTreeSet::new();
    for scope in &ruff.scopes {
        let state = match &scope.state {
            Ok(state) => state,
            Err(error) => {
                let files = scope.files.len();
                let message = format!("cannot resolve Ruff's settings for {files} files: {error}");
                report.note(Tool::Ruff, message);
                continue;
            }
        };
        let context = Context {
            root,
            files: &scope.files,
            evidence,
            ruff,
            ruff_state: Some(state),
            ty,
        };
        let facts = Scope {
            destination: &scope.destination,
            name: scope_name(root, scope.config.as_deref(), "Ruff's defaults"),
            files: scope.files.len(),
            status: match state.verified_with {
                Some(_) => ruff.checker.verified(),
                None => ruff.checker.unverified(),
            },
            representative: state.verified_with.as_deref(),
            placement: None,
        };
        let verdict = |record: &Record, suggestion: &Suggestion| {
            ruff_verdict(record, suggestion, state, ruff)
        };
        report.weigh(
            catalog,
            Tool::Ruff,
            &context,
            &facts,
            verdict,
            &mut unsupported,
        );
    }
    match &ty.state {
        Err(error) => report.note(Tool::Ty, format!("cannot read ty's settings: {error}")),
        Ok(state) => {
            let context = Context {
                root,
                files,
                evidence,
                ruff,
                ruff_state: None,
                ty,
            };
            let facts = Scope {
                destination: &ty.destination,
                name: scope_name(root, ty.config.as_deref(), "ty's defaults"),
                files: files.len(),
                status: match ty.installed {
                    Some(_) => ty.checker.verified(),
                    None => ty.checker.unverified(),
                },
                representative: None,
                placement: None,
            };
            let verdict = |record: &Record, suggestion: &Suggestion| {
                ty_verdict(catalog, record, suggestion, state, ty)
            };
            report.weigh(
                catalog,
                Tool::Ty,
                &context,
                &facts,
                verdict,
                &mut unsupported,
            );
        }
    }
    report.finish(unsupported, ruff, ty);
    report
}

/// The configuration file a scope's advice is for, or the defaults that apply without one.
fn scope_name(root: &Path, config: Option<&Path>, defaults: &str) -> String {
    config.map_or_else(
        || format!("{defaults}, with no configuration"),
        |config| shown(root, config).display().to_string(),
    )
}

fn ruff_verdict(
    record: &Record,
    suggestion: &Suggestion,
    state: &ruff::State,
    ruff: &Ruff,
) -> Verdict {
    match &suggestion.target {
        Target::Select { code, .. } => {
            if ruff
                .installed_rules
                .as_ref()
                .is_some_and(|installed| !installed.contains_key(code))
            {
                return Verdict::of(Outcome::Unsupported(code.clone()));
            }
            if record.is_preview() && !state.preview {
                return Verdict::nothing();
            }
            if state.enabled.contains(code) {
                let per_file = state.per_file_ignored.get(code).into_iter().flatten();
                let kept = per_file.map(|at| {
                    (
                        format!("{code} runs, except where {} turns it off", at.text),
                        at.clone(),
                    )
                });
                return Verdict {
                    outcome: Outcome::Nothing,
                    kept: kept.collect(),
                };
            }
            match state.ignored.get(code) {
                Some(at) => Verdict {
                    outcome: Outcome::Nothing,
                    kept: vec![(
                        format!("{code} stays off, because the project sets {}", at.text),
                        at.clone(),
                    )],
                },
                None => Verdict::of(Outcome::Suggest(None)),
            }
        }
        Target::Setting {
            key,
            value,
            satisfied,
        } => setting_verdict(&state.settings, key, value, *satisfied),
        Target::Command(_) => Verdict::of(Outcome::Suggest(None)),
        Target::Level { .. } | Target::Remove { .. } => Verdict::nothing(),
    }
}

fn ty_verdict(
    catalog: &Catalog,
    record: &Record,
    suggestion: &Suggestion,
    state: &ty::State,
    project: &Ty,
) -> Verdict {
    let level = match &suggestion.target {
        Target::Level { level, .. } => *level,
        Target::Setting {
            key,
            value,
            satisfied,
        } => return setting_verdict(&state.settings, key, value, *satisfied),
        Target::Select { .. } | Target::Command(_) | Target::Remove { .. } => {
            return Verdict::nothing();
        }
    };
    let rule = &record.key.id;
    if project
        .installed
        .as_ref()
        .is_some_and(|installed| !installed.contains_key(rule))
    {
        return Verdict::of(Outcome::Unsupported(rule.clone()));
    }
    // An override that turns the rule down for some files stays visible, whether or not the
    // rest of the project gets a suggestion.
    let mut kept: Vec<(String, Location)> = state
        .overrides
        .iter()
        .filter_map(|scoped| {
            let set = scoped.rules.get(rule).filter(|set| set.level < level)?;
            let detail = format!(
                "{rule} is {} for {} by an override",
                set.level.name(),
                scoped.include.join(", ")
            );
            Some((detail, set.at.clone()))
        })
        .collect();
    let outcome = match project.level(catalog, rule) {
        None => Outcome::Nothing,
        Some((effective, _)) if effective >= level => Outcome::Nothing,
        Some((effective, Some(set))) => {
            kept.push((
                format!(
                    "{rule} stays {}, because the project sets {}",
                    effective.name(),
                    set.at.text
                ),
                set.at.clone(),
            ));
            Outcome::Nothing
        }
        Some((effective, None)) => {
            Outcome::Suggest(Some(Observed::Text(effective.name().to_owned())))
        }
    };
    Verdict { outcome, kept }
}

fn setting_verdict(
    settings: &BTreeMap<String, Observed>,
    key: &str,
    value: &str,
    satisfied: Satisfied,
) -> Verdict {
    let current = settings.get(key);
    if is_satisfied(satisfied, current, value) {
        Verdict::nothing()
    } else {
        Verdict::of(Outcome::Suggest(current.cloned()))
    }
}

/// Whether `current` already has the effect of the TOML value `suggested`.
fn is_satisfied(satisfied: Satisfied, current: Option<&Observed>, suggested: &str) -> bool {
    let (Some(current), Some(suggested)) = (current, Observed::parse(suggested.trim_matches('`')))
    else {
        return false;
    };
    match (satisfied, current, &suggested) {
        (Satisfied::Equals, current, suggested) => current == suggested,
        (Satisfied::NonEmpty, current, _) => !current.is_empty(),
        (Satisfied::Contains, Observed::Items(have), Observed::Items(want)) => want.is_subset(have),
        (Satisfied::Contains, _, _) => false,
    }
}

fn tool_status(checker: &Checker) -> ToolStatus {
    ToolStatus {
        tool: checker.tool.name(),
        pinned: checker.tool.pinned(),
        installed: checker.installed.as_ref().map(|i| i.version.clone()),
        path: checker.installed.as_ref().map(|i| i.path.clone()),
    }
}

fn shown(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

impl Report {
    pub(crate) fn pytest(&mut self, root: &Path, files: usize, catalog: &Catalog, pytest: &Pytest) {
        let tool = Tool::Pytest;
        let (installed, path) = pytest.installed.clone().unzip();
        let pinned = tool.pinned();
        let status = installed.as_ref().map_or_else(
            || format!("unverified locally (decisions for pytest {pinned})"),
            |version| format!("pytest {version} installed (decisions reviewed for {pinned})"),
        );
        self.tools.push(ToolStatus {
            tool: tool.name(),
            pinned,
            installed,
            path,
        });
        for note in &pytest.notes {
            self.note(tool, note.clone());
        }
        let timeout = pytest.project.as_ref().and_then(|project| {
            self.pytest_plugins(
                root,
                files,
                catalog,
                project,
                pytest.installed.as_ref(),
                &status,
            )
        });
        let settings = match &pytest.settings {
            Ok(Some(settings)) => settings,
            Ok(None) => return,
            Err(error) => {
                self.note(tool, format!("cannot read pytest's settings: {error}"));
                return;
            }
        };
        let facts = Scope {
            destination: &settings.destination,
            name: scope_name(root, settings.config.as_deref(), "pytest's defaults"),
            files,
            status,
            representative: None,
            placement: None,
        };
        for (record, suggestion) in catalog
            .suggestions()
            .filter(|(record, _)| record.key.tool == tool && record.key.kind != Kind::Dependency)
        {
            match (&suggestion.target, &suggestion.when) {
                (Target::Remove { key, .. }, _) => {
                    for (item, placement) in settings.removals(record, key) {
                        self.suggest(
                            root,
                            record,
                            &Suggestion {
                                target: Target::Remove {
                                    key: key.clone(),
                                    item,
                                },
                                ..suggestion.clone()
                            },
                            placement.current.clone(),
                            Vec::new(),
                            &Scope {
                                placement: Some(&placement),
                                ..facts.clone()
                            },
                        );
                    }
                }
                (
                    _,
                    When::If {
                        trigger: Trigger::TimeoutPlugin,
                        ..
                    },
                ) if settings.needs_timeout() => {
                    if let Some(location) = &timeout {
                        self.suggest(
                            root,
                            record,
                            suggestion,
                            None,
                            vec![location.clone()],
                            &facts,
                        );
                    }
                }
                (_, When::Always) if settings.missing(&record.key.id) => {
                    self.suggest(root, record, suggestion, None, Vec::new(), &facts);
                }
                _ => {}
            }
        }
    }

    fn pytest_plugins(
        &mut self,
        root: &Path,
        files: usize,
        catalog: &Catalog,
        project: &Project,
        installed: Option<&(String, PathBuf)>,
        status: &str,
    ) -> Option<Location> {
        let groups = project.groups();
        let runtime = project.runtime_dependencies();
        let has = |name| {
            groups
                .iter()
                .chain(runtime.iter())
                .any(|group| group.has(name))
        };
        let has_pytest = installed.is_some() || has("pytest");
        let path = Project::plugin_group(&groups);
        let destination = &project.destination;
        let facts = Scope {
            destination,
            name: scope_name(root, Some(&destination.file), "pytest's dependencies"),
            files,
            status: status.to_owned(),
            representative: None,
            placement: None,
        };
        let add = project.place_group(path);
        for (record, suggestion) in catalog.suggestions().filter(|(record, _)| {
            record.key.tool == Tool::Pytest && record.key.kind == Kind::Dependency
        }) {
            let name = &record.key.id;
            match &suggestion.target {
                Target::Remove { key, .. } => {
                    for group in &groups {
                        for entry in group.entries(name) {
                            let placement = group.place(project, entry);
                            let suggestion = Suggestion {
                                target: Target::Remove {
                                    key: key.clone(),
                                    item: entry.as_str().map(str::to_owned),
                                },
                                ..suggestion.clone()
                            };
                            self.suggest(
                                root,
                                record,
                                &suggestion,
                                None,
                                Vec::new(),
                                &Scope {
                                    placement: Some(&placement),
                                    ..facts.clone()
                                },
                            );
                        }
                    }
                }
                Target::Setting { .. } if has_pytest && !has(name) => {
                    self.suggest(
                        root,
                        record,
                        suggestion,
                        None,
                        Vec::new(),
                        &Scope {
                            placement: Some(&add),
                            ..facts.clone()
                        },
                    );
                    if project.uv_lock && path == ["dependency-groups", "dev"] {
                        let command = Suggestion {
                            target: Target::Command(format!("uv add --dev {name}")),
                            ..suggestion.clone()
                        };
                        self.suggest(root, record, &command, None, Vec::new(), &facts);
                    }
                }
                _ => {}
            }
        }
        groups.iter().chain(runtime.iter()).find_map(|group| {
            let entry = group.entries("pytest-timeout").next()?;
            group.location(project, entry)
        })
    }

    fn note(&mut self, tool: Tool, message: String) {
        self.notes.push(Note {
            tool: tool.name(),
            message,
        });
    }

    fn policy(&mut self, root: &Path, record: &Record, detail: String, at: &Location) {
        self.policies.push(Policy {
            record: record.key.to_string(),
            tool: record.key.tool.name(),
            id: record.key.id.clone(),
            detail,
            file: shown(root, &at.file),
            line: at.line,
        });
    }

    /// Adds a note for each recommended rule the installed release lacks, and the tools' notes.
    fn finish(&mut self, unsupported: BTreeSet<(Tool, String)>, ruff: &Ruff, ty: &Ty) {
        for (tool, id) in unsupported {
            let checker = match tool {
                Tool::Ruff => &ruff.checker,
                Tool::Ty => &ty.checker,
                Tool::Pytest => continue,
            };
            let version = checker
                .installed
                .as_ref()
                .map_or("", |i| i.version.as_str());
            let name = tool.name();
            let pinned = tool.pinned();
            let message = format!(
                "{id} is recommended for {name} {pinned} but {name} {version} lacks it, so doctor leaves it out"
            );
            self.note(tool, message);
        }
        for message in &ruff.notes {
            self.note(Tool::Ruff, message.clone());
        }
        for message in &ty.notes {
            self.note(Tool::Ty, message.clone());
        }
    }

    /// Weighs each suggestion of `tool` in one scope, by the evidence of `context` and the
    /// verdict on the project's settings.
    fn weigh(
        &mut self,
        catalog: &Catalog,
        tool: Tool,
        context: &Context<'_>,
        facts: &Scope<'_>,
        verdict: impl Fn(&Record, &Suggestion) -> Verdict,
        unsupported: &mut BTreeSet<(Tool, String)>,
    ) {
        for (record, suggestion) in catalog.suggestions().filter(|(r, _)| r.key.tool == tool) {
            if let Some(found) = context.applies(suggestion) {
                let verdict = verdict(record, suggestion);
                self.apply(
                    context.root,
                    record,
                    suggestion,
                    verdict,
                    found,
                    facts,
                    unsupported,
                );
            }
        }
    }

    /// Records what `verdict` says about `record` in the scope of `facts`.
    #[expect(
        clippy::too_many_arguments,
        reason = "a verdict joins the record, its evidence and its scope"
    )]
    fn apply(
        &mut self,
        root: &Path,
        record: &Record,
        suggestion: &Suggestion,
        verdict: Verdict,
        evidence: Vec<Location>,
        facts: &Scope<'_>,
        unsupported: &mut BTreeSet<(Tool, String)>,
    ) {
        for (detail, at) in verdict.kept {
            self.policy(root, record, detail, &at);
        }
        match verdict.outcome {
            Outcome::Nothing => {}
            Outcome::Unsupported(id) => {
                unsupported.insert((record.key.tool, id));
            }
            Outcome::Suggest(current) => {
                self.suggest(root, record, suggestion, current, evidence, facts);
            }
        }
    }

    fn suggest(
        &mut self,
        root: &Path,
        record: &Record,
        suggestion: &Suggestion,
        current: Option<Observed>,
        evidence: Vec<Location>,
        facts: &Scope<'_>,
    ) {
        let condition = match &suggestion.when {
            When::If { condition, .. } => Some(condition.clone()),
            When::Always => None,
        };
        let (key, value, assign) = match &suggestion.target {
            Target::Select { key, code } => (key, format!("\"{code}\""), Assign::Item),
            Target::Level { key, level } => (key, format!("\"{}\"", level.name()), Assign::Value),
            Target::Setting { key, value, .. } => (
                key,
                value.clone(),
                if record.key.kind == Kind::Dependency {
                    Assign::Item
                } else {
                    Assign::Value
                },
            ),
            Target::Command(command) => (command, String::new(), Assign::Command),
            Target::Remove {
                key,
                item: Some(item),
            } => (key, item.clone(), Assign::RemoveItem),
            Target::Remove { key, item: None } => (
                key,
                facts.destination.literal(key).unwrap_or_default(),
                Assign::RemoveKey,
            ),
        };
        let placement = (assign != Assign::Command).then(|| {
            facts
                .placement
                .cloned()
                .unwrap_or_else(|| facts.destination.place(key))
        });
        let arguments = placement
            .as_ref()
            .filter(|_| pytest::is_addopts(record.key.tool, &suggestion.target))
            .map(|placement| (placement.clone(), pytest::argument_style(facts.destination)));
        let (value, edit) = if let Some(placement) = &placement {
            let file = shown(root, &facts.destination.file);
            let edit = if let Some((placement, style)) = &arguments {
                pytest::add_arguments(placement, &[&value], *style)
            } else {
                edit(placement, &value, assign, &file)
            };
            (value, edit)
        } else {
            let command = match facts.representative {
                Some(file) if record.key.id == "--show-settings" => {
                    format!("{key} {}", file.display())
                }
                _ => key.clone(),
            };
            (command.clone(), format!("run {command}"))
        };
        self.suggestions.push(Advice {
            record: record.key.to_string(),
            tool: record.key.tool.name(),
            id: record.key.id.clone(),
            name: record.name.clone(),
            decision: record.decision.word(),
            file: placement
                .as_ref()
                .map(|_| shown(root, &facts.destination.file)),
            file_exists: placement.as_ref().is_some_and(|p| p.file_exists),
            table: placement
                .as_ref()
                .map(|p| p.table.clone())
                .unwrap_or_default(),
            table_line: placement.as_ref().and_then(|p| p.table_line),
            key: placement
                .as_ref()
                .map(|p| p.key.clone())
                .unwrap_or_default(),
            key_line: placement.as_ref().and_then(|p| p.key_line),
            value,
            current: current.map(|c| c.show()),
            edit,
            scope: facts.name.clone(),
            files: facts.files,
            reason: record.plain_reason(),
            source: record.source.clone(),
            since: record.since.clone(),
            condition,
            evidence: relative(root, evidence),
            status: facts.status.clone(),
            arguments,
            assign,
        });
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub(crate) fn to_text(&self) -> String {
        let mut out = String::new();
        for tool in &self.tools {
            let _ = match (&tool.installed, &tool.path) {
                (Some(version), Some(path)) => writeln!(
                    out,
                    "{} {version} at {}; decisions reviewed for {} {}",
                    tool.tool,
                    path.display(),
                    tool.tool,
                    tool.pinned
                ),
                _ => writeln!(
                    out,
                    "{} not found; its advice is for {} {} and unverified locally",
                    tool.tool, tool.tool, tool.pinned
                ),
            };
        }
        self.write_edits(&mut out);
        let commands: Vec<&Advice> = self
            .suggestions
            .iter()
            .filter(|a| a.file.is_none())
            .collect();
        if !commands.is_empty() {
            out.push_str("\nCommands\n");
            for advice in commands {
                let _ = writeln!(out, "  {}", advice.value);
                advice.explain(&mut out);
            }
        }
        if !self.policies.is_empty() {
            out.push_str("\nProject choices doctor keeps\n");
            for policy in &self.policies {
                let _ = writeln!(
                    out,
                    "  {}:{}: {}",
                    policy.file.display(),
                    policy.line,
                    policy.detail
                );
            }
        }
        if !self.notes.is_empty() {
            out.push_str("\nNotes\n");
            for note in &self.notes {
                let _ = writeln!(out, "  {}: {}", note.tool, note.message);
            }
        }
        out
    }

    /// The edits, grouped by the table they go in. Codes for one selector array share a line.
    fn write_edits(&self, out: &mut String) {
        let mut groups: Vec<Vec<&Advice>> = Vec::new();
        for advice in self.suggestions.iter().filter(|a| a.file.is_some()) {
            let same_table = |group: &&mut Vec<&Advice>| {
                let first = group[0];
                (&first.file, &first.table, &first.scope, &first.status)
                    == (&advice.file, &advice.table, &advice.scope, &advice.status)
            };
            match groups.iter_mut().find(same_table) {
                Some(group) => group.push(advice),
                None => groups.push(vec![advice]),
            }
        }
        if groups.is_empty() {
            out.push_str("\nNo settings to change.\n");
        }
        for group in groups {
            let first = group[0];
            let file = first.file.as_deref().unwrap_or(Path::new("")).display();
            let place = match (first.file_exists, first.table.is_empty(), first.table_line) {
                (false, _, _) => format!("{file} (new file), table [{}]", first.table),
                (true, true, _) => format!("{file}, top of the file"),
                (true, false, Some(line)) => {
                    format!("{file}, table [{}] at line {line}", first.table)
                }
                (true, false, None) => format!("{file}, new table [{}]", first.table),
            };
            let files = match first.files {
                1 => "1 Python file".to_owned(),
                count => format!("{count} Python files"),
            };
            let _ = writeln!(
                out,
                "\n{place}\n  For {files} under {}, {}.",
                first.scope, first.status
            );
            let mut keys: Vec<&str> = Vec::new();
            for advice in &group {
                if advice.decision == "remove" {
                    let _ = writeln!(out, "  {}", advice.edit);
                    advice.explain(out);
                    continue;
                }
                if !keys.contains(&advice.key.as_str()) {
                    keys.push(&advice.key);
                }
            }
            for key in keys {
                let same: Vec<&Advice> = group
                    .iter()
                    .filter(|m| m.key == key && m.decision != "remove")
                    .copied()
                    .collect();
                let values: Vec<&str> = same.iter().map(|m| m.value.as_str()).collect();
                let _ = match (same.as_slice(), same[0].key_line) {
                    _ if let Some((placement, style)) = &same[0].arguments => {
                        writeln!(
                            out,
                            "  {}",
                            pytest::add_arguments(placement, &values, *style)
                        )
                    }
                    ([single], Some(_)) => writeln!(out, "  {}", single.edit),
                    ([single], None)
                        if single.tool == "pytest" && single.assign == Assign::Item =>
                    {
                        writeln!(out, "  {key} = [{}]", single.value)
                    }
                    ([single], None) => writeln!(out, "  {key} = {}", single.value),
                    (_, Some(line)) => writeln!(
                        out,
                        "  add {} to the existing {key} array at line {line}",
                        values.join(", ")
                    ),
                    (_, None) => writeln!(out, "  {key} = [{}]", values.join(", ")),
                };
                for member in same {
                    member.explain(out);
                }
            }
        }
    }
}

impl Advice {
    fn explain(&self, out: &mut String) {
        let name = self
            .name
            .as_deref()
            .map(|n| format!(" {n}"))
            .unwrap_or_default();
        let _ = writeln!(out, "    {}{name}. {}", self.id, self.reason);
        if let Some(condition) = &self.condition {
            let _ = writeln!(out, "    Condition: {condition}");
        }
        let _ = writeln!(out, "    Source: {}", self.source);
        for location in self.evidence.iter().take(3) {
            let _ = writeln!(
                out,
                "    Evidence: {}:{} {}",
                location.file.display(),
                location.line,
                location.text
            );
        }
        if self.evidence.len() > 3 {
            let _ = writeln!(out, "    Evidence: {} more places", self.evidence.len() - 3);
        }
    }
}

fn relative(root: &Path, evidence: Vec<Location>) -> Vec<Location> {
    evidence
        .into_iter()
        .map(|location| Location {
            file: shown(root, &location.file),
            ..location
        })
        .collect()
}

/// How a suggestion changes its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Assign {
    /// Add an item to the key's array.
    Item,
    /// Set the key to the value.
    Value,
    /// No key: run a command.
    Command,
    /// Remove a key and its value.
    RemoveKey,
    /// Remove part of a key's value.
    RemoveItem,
}

/// The edit that sets `key` to `value` at `placement`, or adds `value` to its array.
fn edit(placement: &Placement, value: &str, assign: Assign, file: &Path) -> String {
    let key = &placement.key;
    let assignment = match assign {
        Assign::RemoveKey | Assign::RemoveItem => {
            let line = placement
                .key_line
                .map_or_else(String::new, |line| format!(" at line {line}"));
            return match assign {
                Assign::RemoveKey => format!("remove {key} = {value}{line}"),
                _ if key == "addopts" => format!("remove {value} from {key}{line}"),
                _ => format!("remove {value:?} from {key}{line}"),
            };
        }
        Assign::Item => format!("{key} = [{value}]"),
        Assign::Value | Assign::Command => format!("{key} = {value}"),
    };
    if let Some(line) = placement.key_line {
        return match (
            &placement.current,
            assign == Assign::Item && placement.array,
        ) {
            (_, true) => format!("add {value} to the existing {key} array at line {line}"),
            (Some(current), false) => format!(
                "change {key} from {} to {value} at line {line}",
                current.show()
            ),
            (None, false) => format!("set {assignment} at line {line}"),
        };
    }
    let place = match (
        placement.file_exists,
        placement.table.is_empty(),
        placement.table_line,
    ) {
        (false, _, _) => format!(
            "in a new [{}] table of a new {}",
            placement.table,
            file.display()
        ),
        (true, true, _) if placement.top_before_tables => {
            format!("at the top of {}, before its first table", file.display())
        }
        (true, true, _) => format!("at the top of {}", file.display()),
        (true, false, Some(line)) => format!("under [{}] at line {line}", placement.table),
        (true, false, None) => format!("in a new [{}] table", placement.table),
    };
    format!("add {assignment} {place}")
}
