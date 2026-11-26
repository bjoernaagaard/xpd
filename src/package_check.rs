//! Package checks: packages a library works better with, reported once per project when the
//! project's venv has the library but not the package.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::pack::{Pack, PackageCheck, PackageCheckCondition};
use crate::report::Issue;
use crate::rule_id::RuleId;
use crate::severity::Severity;
use crate::syntax::Root;
use crate::venv::{Distributions, Venv};

/// The warning's message when the venv lacks the package and the project meets the
/// check's condition. `configured` holds the tools with a table in the project's
/// `pyproject.toml`, and `seen` holds what the run found of the check's `onlyWhere`.
fn report(
    check: &PackageCheck,
    installed: &Distributions,
    configured: &[&str],
    seen: Option<&Seen>,
) -> Option<String> {
    if installed.version(&check.package).is_some() {
        return None;
    }
    let uses =
        |tool: &String| configured.contains(&tool.as_str()) || installed.version(tool).is_some();
    let PackageCheckCondition {
        only_with,
        only_where,
    } = &check.when;
    if !only_with.is_empty() && !only_with.iter().any(uses) {
        return None;
    }
    match (only_where, seen) {
        (None, _) => Some(check.message.clone()),
        (Some(_), None) => None,
        (Some(_), Some(seen)) => Some(
            check
                .message
                .replace("{count}", &seen.count.to_string())
                .replace("{first}", &seen.first()),
        ),
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
    pub(crate) fn find(path: &Path, root: &Root, applicable: &[&Pack]) -> Self {
        let mut calls = Self::default();
        for pack in applicable {
            let shaped = pack
                .package_checks
                .iter()
                .filter_map(|check| Some((check, check.when.only_where.as_ref()?)));
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
) -> Result<Vec<Issue>> {
    let pyproject = nearest_pyproject(project)
        .map(|path| {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("cannot read {}", path.display()))?;
            Ok::<_, anyhow::Error>((path, text))
        })
        .transpose()?;
    let configured = pyproject.as_ref().map(|(_, text)| configured_tools(text));
    let configured = configured.as_deref().unwrap_or_default();
    let file = pyproject.as_ref().map_or(venv.root(), |(path, _)| path);
    let file = shown(file, project);
    Ok(packs
        .iter()
        .filter(|pack| installed.version(&pack.module).is_some())
        .flat_map(|pack| pack.package_checks.iter().map(move |check| (pack, check)))
        .filter_map(|(pack, check)| {
            let message = report(check, installed, configured, calls.get(pack, check))?;
            Some(Issue {
                id: RuleId::qualified(&pack.module, &check.id),
                category: crate::category::RuleCategory::Default(pack.default_category),
                file: file.clone(),
                position: None,
                severity: SEVERITY,
                message,
                fix: None,
                help: pack.page(&check.id),
            })
        })
        .collect())
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
