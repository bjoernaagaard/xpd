//! Package checks: packages a library works better with, reported once per project when the
//! project's venv has the library but not the package.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ast_grep_config::{GlobalRules, SerializableRule, Severity};
use ast_grep_language::SupportLang;
use serde::Deserialize;

use crate::pack::{ManifestRule, Pack};
use crate::report::Issue;
use crate::scan::Root;
use crate::venv::{Distributions, Venv};

/// One entry under `packages` in a pack's `pack.yml`, as it is written.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageCheckSource {
    id: String,
    package: String,
    #[serde(default)]
    only_with: Vec<String>,
    only_where: Option<SerializableRule>,
    message: String,
}

/// One package check of a pack.
#[derive(Debug)]
pub(crate) struct PackageCheck {
    /// Rule name in the report, such as `missing-pyarrow`.
    pub(crate) id: String,
    /// Distribution name, such as `pandas-stubs`.
    package: String,
    when: Condition,
    /// May name `{count}` and `{first}` when the check has `onlyWhere`.
    pub(crate) message: String,
}

/// What a project that lacks the package must also show for the check to report.
#[derive(Debug)]
struct Condition {
    /// When not empty, the project uses one of these tools.
    only_with: Vec<String>,
    /// When set, a file that the run checked with the pack holds code of this shape.
    only_where: Option<ManifestRule>,
}

/// The placeholders a message may name when its check has `onlyWhere`.
const PLACEHOLDERS: [&str; 2] = ["{count}", "{first}"];

impl PackageCheck {
    /// Reads `onlyWhere` against the pack's utility rules in `globals`.
    pub(crate) fn new(source: PackageCheckSource, globals: &GlobalRules) -> Result<Self> {
        let PackageCheckSource {
            id,
            package,
            only_with,
            only_where,
            message,
        } = source;
        let only_where = only_where
            .map(|rule| ManifestRule::read(rule, globals))
            .transpose()
            .with_context(|| format!("onlyWhere of {id}"))?;
        if only_where.is_none()
            && let Some(placeholder) = PLACEHOLDERS.iter().find(|p| message.contains(*p))
        {
            bail!("the package check {id} names {placeholder} in its message without onlyWhere");
        }
        Ok(Self {
            id,
            package,
            when: Condition {
                only_with,
                only_where,
            },
            message,
        })
    }

    /// The warning's message when the venv lacks the package and the project meets the
    /// check's condition. `configured` holds the tools with a table in the project's
    /// `pyproject.toml`, and `seen` holds what the run found of the check's `onlyWhere`.
    fn report(
        &self,
        installed: &Distributions,
        configured: &[&str],
        seen: Option<&Seen>,
    ) -> Option<String> {
        if installed.version(&self.package).is_some() {
            return None;
        }
        let uses = |tool: &String| {
            configured.contains(&tool.as_str()) || installed.version(tool).is_some()
        };
        let Condition {
            only_with,
            only_where,
        } = &self.when;
        if !only_with.is_empty() && !only_with.iter().any(uses) {
            return None;
        }
        match (only_where, seen) {
            (None, _) => Some(self.message.clone()),
            (Some(_), None) => None,
            (Some(_), Some(seen)) => Some(
                self.message
                    .replace("{count}", &seen.count.to_string())
                    .replace("{first}", &seen.first()),
            ),
        }
    }
}

/// What the `onlyWhere` shapes of the package checks matched in the files of one run.
#[derive(Debug, Default, Clone)]
pub(crate) struct Calls(BTreeMap<(String, String), Seen>);

/// The matches of one check's `onlyWhere`.
#[derive(Debug, Clone)]
struct Seen {
    count: usize,
    /// The file and line of the first match, by path and then by line.
    first: (PathBuf, usize),
}

impl Seen {
    fn first(&self) -> String {
        let (path, line) = &self.first;
        format!("{}:{line}", path.display())
    }
}

impl Calls {
    /// The matches in the parsed file `root` at `path`, of `src` and `language`. A check's
    /// shape runs only on a file that its pack's rules run on.
    pub(crate) fn find(
        path: &Path,
        root: &Root,
        src: &str,
        language: SupportLang,
        packs: &[Pack],
    ) -> Self {
        let mut calls = Self::default();
        for pack in packs {
            let checks = pack.package_checks.iter();
            let mut shaped = checks
                .filter_map(|check| Some((check, check.when.only_where.as_ref()?)))
                .peekable();
            if shaped.peek().is_none() || !pack.applies(root, src, language) {
                continue;
            }
            for (check, only_where) in shaped {
                let lines: Vec<usize> = (root.root().find_all(&only_where.shape))
                    .map(|found| found.start_pos().line() + 1)
                    .collect();
                if let Some(&line) = lines.iter().min() {
                    let seen = Seen {
                        count: lines.len(),
                        first: (path.to_path_buf(), line),
                    };
                    calls.add((pack.module.clone(), check.id.clone()), seen);
                }
            }
        }
        calls
    }

    /// Adds the matches of `other`, from other files of the same run.
    pub(crate) fn merge(&mut self, other: Self) {
        for (key, seen) in other.0 {
            self.add(key, seen);
        }
    }

    fn add(&mut self, key: (String, String), seen: Seen) {
        let Some(known) = self.0.get_mut(&key) else {
            self.0.insert(key, seen);
            return;
        };
        known.count += seen.count;
        if seen.first < known.first {
            known.first = seen.first;
        }
    }

    fn get(&self, pack: &Pack, check: &PackageCheck) -> Option<&Seen> {
        self.0.get(&(pack.module.clone(), check.id.clone()))
    }
}

/// Every package check reports at this severity.
pub(crate) const SEVERITY: Severity = Severity::Warning;

/// One warning for each package that a pack asks for and the venv lacks. A pack is checked only
/// when the venv has its library. `installed` is what `venv` holds, `project` is the folder the
/// run starts in, which is the working directory for the command line, and `calls` is what the
/// run found of the checks' `onlyWhere` shapes.
pub(crate) fn missing(
    packs: &[Pack],
    venv: &Venv,
    installed: &Distributions,
    project: &Path,
    calls: &Calls,
) -> Vec<Issue> {
    let pyproject = nearest_pyproject(project)
        .and_then(|path| fs::read_to_string(&path).ok().map(|text| (path, text)));
    let configured = pyproject.as_ref().map(|(_, text)| configured_tools(text));
    let configured = configured.as_deref().unwrap_or_default();
    // The issue points at the file where the dependency is added, or else at the venv.
    let file = pyproject.as_ref().map_or(venv.root(), |(path, _)| path);
    let file = shown(file, project);
    packs
        .iter()
        .filter(|pack| installed.version(&pack.module).is_some())
        .flat_map(|pack| pack.package_checks.iter().map(move |check| (pack, check)))
        .filter_map(|(pack, check)| {
            let message = check.report(installed, configured, calls.get(pack, check))?;
            Some(Issue {
                package: pack.module.clone(),
                rule: check.id.clone(),
                file: file.clone(),
                position: None,
                severity: SEVERITY,
                message,
                fix: None,
                help: pack.page(&check.id),
            })
        })
        .collect()
}

/// The nearest `pyproject.toml` in or above `project`, where the search for the venv also
/// starts. The venv may live elsewhere, such as in `$VIRTUAL_ENV`.
fn nearest_pyproject(project: &Path) -> Option<PathBuf> {
    project
        .ancestors()
        .map(|dir| dir.join("pyproject.toml"))
        .find(|path| path.is_file())
}

/// The names in the `[tool.<name>]` and `[tool.<name>.<part>]` table headers of a
/// `pyproject.toml`.
fn configured_tools(pyproject: &str) -> Vec<&str> {
    pyproject
        .lines()
        .filter_map(|line| line.trim().strip_prefix("[tool."))
        .filter_map(|rest| rest.split(['.', ']']).next())
        .collect()
}

/// `path` relative to `project` when it is inside it, as the other issues show their files.
fn shown(path: &Path, project: &Path) -> PathBuf {
    path.strip_prefix(project).unwrap_or(path).to_path_buf()
}
