use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::catalog::{Catalog, Filters};
use crate::package_check::{self, Calls};
use crate::report::Issue;
use crate::settings::Settings;
use crate::venv::{Distributions, Venv};

#[derive(Debug)]
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
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
            catalog,
            environment: venv.zip(installed),
        })
    }

    pub(crate) fn package_issues(&self, calls: &Calls) -> Vec<Issue> {
        self.environment
            .as_ref()
            .map_or_else(Vec::new, |(venv, installed)| {
                package_check::missing(&self.catalog.packs, venv, installed, &self.root, calls)
            })
    }
}
