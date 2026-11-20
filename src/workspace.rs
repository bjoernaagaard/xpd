use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::catalog::{Catalog, Filters, LocalProject};
use crate::fix_safety::Applicability;
use crate::pack;
use crate::package_check::{self, Calls};
use crate::report::Issue;
use crate::scan::{self, Edits, Fixed, Linted, Scanned};
use crate::settings::Settings;
use crate::source::{self, File};
use crate::style::Style;
use crate::venv::{Distributions, Venv};
use crate::version::Version;

#[derive(Debug)]
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) settings: Settings,
    pub(crate) catalog: Arc<Catalog>,
    environment: Inventory,
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
        let environment = inventory(venv);
        let config = Catalog::config(&root, config)?;
        let mut catalog = Catalog::load(
            config.as_deref(),
            distributions(&environment),
            settings.style,
        )?;
        catalog.select(filters)?;
        Ok(Self {
            root,
            settings,
            catalog: Arc::new(catalog),
            environment,
        })
    }

    pub(crate) fn bundled(style: Style) -> Result<Self> {
        Ok(Self {
            root: PathBuf::new(),
            settings: Settings { style },
            catalog: Arc::new(Catalog::load(None, None, style)?),
            environment: None,
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
            .filter(|file| source::is_python(file) || (has_rules && source::names_language(file)))
            .cloned()
            .collect()
    }

    pub(crate) fn analyze(&self, file: File<'_>, source: &str) -> Linted {
        scan::analyze(file, source, &self.catalog.packs, Edits::Report)
    }

    pub(crate) fn fix(&self, file: File<'_>, source: String, allowed: Applicability) -> Fixed {
        scan::fix_source(file, source, &self.catalog.packs, allowed)
    }

    pub(crate) fn package_issues(&self, calls: &Calls) -> Vec<Issue> {
        self.environment
            .as_ref()
            .map_or_else(Vec::new, |(venv, installed)| {
                package_check::missing(&self.catalog.packs, venv, installed, &self.root, calls)
            })
    }
}

type Inventory = Option<(Venv, Distributions)>;

fn inventory(venv: Option<Venv>) -> Inventory {
    venv.map(|venv| {
        let installed = venv.distributions();
        (venv, installed)
    })
}

fn distributions(environment: &Inventory) -> Option<&Distributions> {
    environment.as_ref().map(|(_, installed)| installed)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EmbeddedRules {
    versions: Vec<Option<Version>>,
    style: Style,
}

#[derive(Debug)]
pub(crate) struct LoadedWorkspace {
    pub(crate) workspace: Arc<Workspace>,
    pub(crate) error: Option<String>,
    embedded: Option<EmbeddedRules>,
}

/// Reuse embedded rule compilations across open projects. Local rules are read afresh.
#[derive(Debug, Default)]
pub(crate) struct WorkspaceCache {
    embedded: HashMap<EmbeddedRules, Arc<Catalog>>,
}

impl WorkspaceCache {
    pub(crate) fn reload(
        &mut self,
        root: &Path,
        previous: Option<&LoadedWorkspace>,
    ) -> Result<(LoadedWorkspace, bool)> {
        let environment = inventory(Venv::find_from(root));
        let installed = distributions(&environment);
        let settings = Settings::find(root);
        let config = Catalog::config(root, None)?;
        let loaded = match (settings, config.as_deref()) {
            (Err(error), _) => Err(error),
            (Ok(settings), Some(config)) => Catalog::load(Some(config), installed, settings.style)
                .map(|catalog| (Arc::new(catalog), settings, None)),
            (Ok(settings), None) => {
                let key = EmbeddedRules {
                    versions: pack::installed_versions(installed),
                    style: settings.style,
                };
                let catalog = self.embedded(&key, installed)?;
                Ok((catalog, settings, Some(key)))
            }
        };
        let (catalog, settings, embedded, error, changed) = match loaded {
            Ok((catalog, settings, embedded)) => {
                let changed = previous.is_some_and(|old| {
                    embedded.is_none() || embedded != old.embedded || old.error.is_some()
                });
                (catalog, settings, embedded, None, changed)
            }
            Err(error) => {
                let message = format!("{error:#}");
                let (catalog, settings, embedded, changed) = if let Some(old) = previous {
                    (
                        Arc::clone(&old.workspace.catalog),
                        old.workspace.settings,
                        old.embedded.clone(),
                        old.error.as_deref() != Some(&message),
                    )
                } else {
                    (
                        Arc::new(Catalog::load(None, installed, Style::Default)?),
                        Settings::default(),
                        None,
                        false,
                    )
                };
                (catalog, settings, embedded, Some(message), changed)
            }
        };
        let workspace = Arc::new(Workspace {
            root: root.to_path_buf(),
            settings,
            catalog,
            environment,
        });
        Ok((
            LoadedWorkspace {
                workspace,
                error,
                embedded,
            },
            changed,
        ))
    }

    fn embedded(
        &mut self,
        key: &EmbeddedRules,
        installed: Option<&Distributions>,
    ) -> Result<Arc<Catalog>> {
        if let Some(catalog) = self.embedded.get(key) {
            return Ok(Arc::clone(catalog));
        }
        let catalog = Arc::new(Catalog::load(None, installed, key.style)?);
        self.embedded.insert(key.clone(), Arc::clone(&catalog));
        Ok(catalog)
    }
}
