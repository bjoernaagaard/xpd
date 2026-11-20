use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::catalog::{self, Filters};
use crate::fix_safety::Applicability;
use crate::report::Report;
use crate::scan;
use crate::workspace::Workspace;

/// Whether a lint run writes fixes, and up to which level.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FixMode {
    Apply(Option<Applicability>),
    Review(Applicability),
}

/// The issues of the files under `paths`, with the rules and the settings of the project in the
/// working directory.
pub(crate) fn lint(
    paths: &[PathBuf],
    mode: FixMode,
    scope: Scope,
    filters: &Filters,
    config: Option<&Path>,
) -> Result<Report> {
    let workspace = Workspace::open(std::env::current_dir()?, config, filters)?;
    workspace.check(paths, mode, scope)
}

impl Workspace {
    pub(crate) fn check(&self, paths: &[PathBuf], mode: FixMode, scope: Scope) -> Result<Report> {
        let catalog = &self.catalog;
        let files = scan::collect_files(paths, &catalog.packs)?;
        let (scanned, fixed) = match mode {
            FixMode::Apply(fix) => {
                let scanned = scan::scan(&files, &catalog.packs, fix)?;
                let fixed = scanned.fixed;
                (scanned, fixed)
            }
            FixMode::Review(allowed) => {
                let fixed = crate::review::apply(&files, self, allowed)?;
                (scan::scan(&files, &catalog.packs, None)?, fixed)
            }
        };
        let mut issues = scanned.issues;
        if matches!(scope, Scope::Project) {
            issues.extend(self.package_issues(&scanned.calls));
            for warning in catalog
                .project
                .iter()
                .flat_map(catalog::LocalProject::warnings)
            {
                eprintln!("xpd: {warning}");
            }
        }
        Ok(Report {
            issues,
            fixed,
            skipped: scanned.skipped,
        })
    }
}

/// How much a lint run looks at.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scope {
    /// The given files only. The hook runs after every edit, so it stops here.
    Files,
    /// The files, and the packages the project's venv lacks.
    Project,
}
