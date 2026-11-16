//! Which project a document belongs to, and the rules for that project's venv.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use lsp_types::Uri;

use super::uri;
use crate::pack::{self, Pack};
use crate::package_check;
use crate::report::Issue;
use crate::venv::Venv;
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

/// What the server knows about one project folder.
#[derive(Debug)]
pub(crate) struct Project {
    /// The rules for the library versions in the project's venv.
    pub(crate) packs: Rc<[Pack]>,
    /// One warning for each package the venv lacks, as `xpd check` adds them to a run.
    pub(crate) package_issues: Vec<Issue>,
}

/// The client's workspace folders and the projects of the open documents.
#[derive(Debug, Default)]
pub(crate) struct Workspace {
    folders: Vec<PathBuf>,
    /// The packs for each set of installed library versions. Parsing the rules takes about
    /// half a second, so projects with the same versions share one set.
    loaded: HashMap<Vec<Option<Version>>, Rc<[Pack]>>,
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
        let packs = if let Some(packs) = self.loaded.get(&versions) {
            Rc::clone(packs)
        } else {
            let packs: Rc<[Pack]> = pack::load(installed.as_ref())
                .context("invalid embedded rules")?
                .into();
            self.loaded.insert(versions, Rc::clone(&packs));
            packs
        };
        let package_issues = match (&venv, &installed) {
            (Some(venv), Some(installed)) => package_check::missing(&packs, venv, installed, root),
            _ => Vec::new(),
        };
        let project = Project {
            packs,
            package_issues,
        };
        let before = self.projects.insert(root.to_path_buf(), project);
        let now = &self.projects[root];
        Ok(before.is_some_and(|before| !Rc::ptr_eq(&before.packs, &now.packs)))
    }

    /// Drops the project in `root`, after its last document closed.
    pub(crate) fn forget(&mut self, root: &Path) {
        self.projects.remove(root);
    }
}
