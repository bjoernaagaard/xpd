use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::catalog::{self, Catalog, Filters};
use crate::package_check;
use crate::report::Report;
use crate::rule::Applicability;
use crate::scan;
use crate::settings::Settings;
use crate::venv::Venv;

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
    let venv = Venv::find();
    let installed = venv.as_ref().map(Venv::distributions);
    let project = std::env::current_dir()?;
    let style = Settings::find(&project)?.style;
    let mut catalog = match config {
        Some(config) => Catalog::load_config(config, installed.as_ref(), style)?,
        None => Catalog::load(&project, installed.as_ref(), style)?,
    };
    catalog.select(filters)?;
    let files = scan::collect_files(paths, &catalog.packs)?;
    let scanned = match mode {
        FixMode::Apply(fix) => scan::scan(&files, &catalog.packs, fix)?,
        FixMode::Review(allowed) => crate::review::scan(&files, &catalog.packs, allowed)?,
    };
    let mut issues = scanned.issues;
    if let (Scope::Project, Some(venv), Some(installed)) = (scope, &venv, &installed) {
        issues.extend(package_check::missing(
            &catalog.packs,
            venv,
            installed,
            &project,
            &scanned.calls,
        ));
    }
    if matches!(scope, Scope::Project) {
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
        fixed: scanned.fixed,
        skipped: scanned.skipped,
    })
}

/// How much a lint run looks at.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scope {
    /// The given files only. The hook runs after every edit, so it stops here.
    Files,
    /// The files, and the packages the project's venv lacks.
    Project,
}
