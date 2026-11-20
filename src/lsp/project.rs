//! Which project a document belongs to, and the rules for that project's venv.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use lsp_types::Uri;

use super::uri;
use crate::catalog::{Catalog, LocalProject};
use crate::pack;
use crate::package_check::{self, Calls};
use crate::report::Issue;
use crate::settings::Settings;
use crate::style::Style;
use crate::venv::{Distributions, Venv};
use crate::version::Version;

/// Where a document is, as `xpd check` would see it when run in the project's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Place {
    /// The project's folder: the workspace folder that holds the document, or else the folder
    /// the document is in. The search for the venv starts here.
    pub(crate) root: PathBuf,
    /// The document's path below `root`, which the `files` and `ignores` globs of a rule match.
    pub(crate) path: PathBuf,
}

impl Place {
    /// Where the document is on disk, from which a rule follows its imports.
    pub(crate) fn disk(&self) -> PathBuf {
        self.root.join(&self.path)
    }
}

/// What the server knows about one project folder.
#[derive(Debug)]
pub(crate) struct Project {
    /// The rules for the library versions in the project's venv.
    pub(crate) catalog: Rc<Catalog>,
    pub(crate) load_error: Option<String>,
    /// The project's venv, with what it holds.
    venv: Option<(Venv, Distributions)>,
}

impl Project {
    /// One warning for each package the venv lacks, as `xpd check` adds them to a run. The
    /// project is in `root`, and `calls` is what the checks' `onlyWhere` shapes matched in the
    /// project's open documents.
    pub(crate) fn package_issues(&self, root: &Path, calls: &Calls) -> Vec<Issue> {
        match &self.venv {
            Some((venv, installed)) => {
                package_check::missing(&self.catalog.packs, venv, installed, root, calls)
            }
            None => Vec::new(),
        }
    }
}

/// The client's workspace folders and the projects of the open documents.
#[derive(Debug, Default)]
pub(crate) struct Workspace {
    folders: Vec<PathBuf>,
    /// The packs for each set of installed library versions and each style setting. Parsing
    /// the rules takes about half a second, so projects with the same versions and the same
    /// setting share one set.
    loaded: HashMap<(Vec<Option<Version>>, Style), Rc<Catalog>>,
    projects: HashMap<PathBuf, Project>,
}

impl Workspace {
    pub(crate) fn add_folder(&mut self, folder: &Uri) {
        if let Some(path) = uri::to_path(folder).filter(|path| !self.folders.contains(path)) {
            self.folders.push(path);
        }
    }

    pub(crate) fn remove_folder(&mut self, folder: &Uri) {
        if let Some(path) = uri::to_path(folder) {
            self.folders.retain(|known| *known != path);
        }
    }

    /// Where the document at `uri` is. A document that is not a file, such as an unsaved new
    /// one, counts as a file in the server's working directory.
    pub(crate) fn place(&self, uri: &Uri) -> Place {
        let Some(file) = uri::to_path(uri) else {
            return Place {
                root: env::current_dir().unwrap_or_default(),
                path: PathBuf::from(uri.path().as_str()),
            };
        };
        let folder = self
            .folders
            .iter()
            .filter(|folder| file.starts_with(folder))
            .max_by_key(|folder| folder.components().count());
        let root = folder.map_or_else(|| file.parent().unwrap_or(&file), PathBuf::as_path);
        Place {
            root: root.to_path_buf(),
            path: file.strip_prefix(root).unwrap_or(&file).to_path_buf(),
        }
    }

    /// The project in `root`, read from its venv when the server has not seen it yet.
    pub(crate) fn project(&mut self, root: &Path) -> Result<&Project> {
        if !self.projects.contains_key(root) {
            self.refresh(root)?;
        }
        self.projects
            .get(root)
            .context("the project was not loaded")
    }

    /// Reads the venv of the project in `root` again. Returns whether a project the server
    /// already knew got other rules, which happens when a library in its venv changed version.
    pub(crate) fn refresh(&mut self, root: &Path) -> Result<bool> {
        let venv = Venv::find_from(root);
        let installed = venv.as_ref().map(Venv::distributions);
        let versions = pack::installed_versions(installed.as_ref());
        // A project with its own rules reads them again each time, and the others share the
        // embedded rules for their versions.
        // A settings file that cannot be read is a load error, as a broken rule file is.
        let loaded = match (Settings::find(root), LocalProject::find(root)?) {
            (Err(error), _) => Err(error),
            (Ok(settings), Some(config)) => {
                Catalog::load_config(&config, installed.as_ref(), settings.style).map(Rc::new)
            }
            (Ok(settings), None) => {
                let key = (versions, settings.style);
                if let Some(catalog) = self.loaded.get(&key) {
                    Ok(Rc::clone(catalog))
                } else {
                    let catalog = Rc::new(Catalog::embedded(installed.as_ref(), settings.style)?);
                    self.loaded.insert(key, Rc::clone(&catalog));
                    Ok(catalog)
                }
            }
        };
        let (catalog, load_error) = match loaded {
            Ok(catalog) => (catalog, None),
            Err(error) => {
                let catalog = match self.projects.get(root) {
                    Some(previous) => Rc::clone(&previous.catalog),
                    None => Rc::new(Catalog::embedded(installed.as_ref(), Style::Default)?),
                };
                (catalog, Some(format!("{error:#}")))
            }
        };
        let project = Project {
            catalog,
            load_error,
            venv: venv.zip(installed),
        };
        let before = self.projects.insert(root.to_path_buf(), project);
        let now = &self.projects[root];
        Ok(before.is_some_and(|before| {
            !Rc::ptr_eq(&before.catalog, &now.catalog) || before.load_error != now.load_error
        }))
    }

    /// Drops the project in `root`, after its last document closed.
    pub(crate) fn forget(&mut self, root: &Path) {
        self.projects.remove(root);
    }
}
