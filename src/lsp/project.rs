//! Which project a document belongs to, and the rules for that project's venv.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lsp_types::Uri;

use super::uri;
use crate::workspace::{LoadedWorkspace, WorkspaceCache};

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

#[derive(Debug, Default)]
pub(crate) struct OpenProjects {
    folders: Vec<PathBuf>,
    cache: WorkspaceCache,
    projects: HashMap<PathBuf, LoadedWorkspace>,
}

impl OpenProjects {
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
    pub(crate) fn project(&mut self, root: &Path) -> Result<&LoadedWorkspace> {
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
        let (loaded, changed) = self.cache.reload(root, self.projects.get(root))?;
        self.projects.insert(root.to_path_buf(), loaded);
        Ok(changed)
    }

    /// Drops the project in `root`, after its last document closed.
    pub(crate) fn forget(&mut self, root: &Path) {
        self.projects.remove(root);
    }
}
