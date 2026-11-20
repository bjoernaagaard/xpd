use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::catalog::{Catalog, Filters, LocalProject};
use crate::fix_safety::Applicability;
use crate::package_check::{self, Calls};
use crate::report::Issue;
use crate::scan::{self, Edits, File, Linted, Scanned};
use crate::settings::Settings;
use crate::venv::{Distributions, Venv};

#[derive(Debug)]
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) settings: Settings,
    pub(crate) catalog: Catalog,
    environment: Option<(Venv, Distributions)>,
}

impl Workspace {
    pub(crate) fn open(root: PathBuf, config: Option<&Path>, filters: &Filters) -> Result<Self> {
        let settings = Settings::find(&root)?;
        let venv = Venv::find_from(&root);
        Self::load(root, config, settings, venv, filters)
    }

    pub(crate) fn load(
        root: PathBuf,
        config: Option<&Path>,
        settings: Settings,
        venv: Option<Venv>,
        filters: &Filters,
    ) -> Result<Self> {
        let installed = venv.as_ref().map(Venv::distributions);
        let mut catalog = match config {
            Some(config) => Catalog::load_config(config, installed.as_ref(), settings.style)?,
            None => Catalog::load(&root, installed.as_ref(), settings.style)?,
        };
        catalog.select(filters)?;
        Ok(Self {
            root,
            settings,
            catalog,
            environment: venv.zip(installed),
        })
    }

    pub(crate) fn collect_files(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        scan::collect_files(&self.root, paths, &self.catalog.packs)
    }

    pub(crate) fn scan_files(
        &self,
        files: &[PathBuf],
        fix: Option<Applicability>,
    ) -> Result<Scanned> {
        scan::scan(&self.root, files, &self.catalog.packs, fix)
    }

    pub(crate) fn edited_files(root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
        // Only a local rule project can opt into a language other than Python.
        let has_rules = LocalProject::find(root).is_ok_and(|config| config.is_some());
        paths
            .iter()
            .filter(|file| scan::is_python(file) || (has_rules && scan::names_language(file)))
            .cloned()
            .collect()
    }

    pub(crate) fn analyze(&self, file: File<'_>, source: &str) -> Linted {
        scan::analyze(file, source, &self.catalog.packs, Edits::Report)
    }

    pub(crate) fn package_issues(&self, calls: &Calls) -> Vec<Issue> {
        self.environment
            .as_ref()
            .map_or_else(Vec::new, |(venv, installed)| {
                package_check::missing(&self.catalog.packs, venv, installed, &self.root, calls)
            })
    }
}
