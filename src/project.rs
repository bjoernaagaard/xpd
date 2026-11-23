//! The layout of a local rule project, before paths are resolved or files are compiled.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};

pub(crate) const SGCONFIG: &str = "sgconfig.yml";
pub(crate) const XPD_CONFIG: &str = "xpd.yml";
pub(crate) const PROJECT_FILES: [&str; 2] = [SGCONFIG, "sgconfig.yaml"];
pub(crate) const PROJECT_ROOT_ENV: &str = "XPD_PROJECT_ROOT";

pub(crate) fn boundary() -> Option<PathBuf> {
    let raw = env::var_os(PROJECT_ROOT_ENV).filter(|value| !value.is_empty())?;
    let path = PathBuf::from(raw);
    Some(
        fs::canonicalize(&path)
            .or_else(|_| std::path::absolute(&path))
            .unwrap_or(path),
    )
}

pub(crate) fn config_in(dir: &Path) -> Option<PathBuf> {
    PROJECT_FILES
        .map(|name| dir.join(name))
        .into_iter()
        .find(|path| path.is_file())
}

pub(crate) fn ancestors_within<'a>(
    from: &'a Path,
    boundary: Option<&'a Path>,
) -> impl Iterator<Item = ProjectFolder<'a>> {
    from.ancestors()
        .take_while(move |root| boundary.is_none_or(|bound| root.starts_with(bound)))
        .map(|root| ProjectFolder {
            root,
            config: config_in(root),
        })
}

pub(crate) fn nearest_config(from: &Path) -> Option<PathBuf> {
    let bound = boundary();
    let canonical = fs::canonicalize(from)
        .or_else(|_| std::path::absolute(from))
        .unwrap_or_else(|_| from.to_path_buf());
    ancestors_within(&canonical, bound.as_deref()).find_map(|folder| folder.config)
}

pub(crate) struct ProjectFolder<'a> {
    root: &'a Path,
    pub(crate) config: Option<PathBuf>,
}

impl ProjectFolder<'_> {
    pub(crate) fn manifest(&self) -> PathBuf {
        self.root.join(XPD_CONFIG)
    }
}

/// Authoring uses the declared folders verbatim. Loading additionally accepts null folders
/// and reads the optional snapshot path from each test entry.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleFolders<Rules, Utilities, Tests> {
    #[serde(default)]
    pub(crate) rule_dirs: Rules,
    #[serde(default)]
    pub(crate) util_dirs: Utilities,
    #[serde(default)]
    pub(crate) test_configs: Tests,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(bound(deserialize = "Directory: Deserialize<'de>, Snapshot: Deserialize<'de>"))]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestFolder<Directory = PathBuf, Snapshot = PathBuf> {
    pub(crate) test_dir: Directory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) snapshot_dir: Option<Snapshot>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlineFolders {
    #[serde(default)]
    pub(crate) outline_rules: HashMap<SupportLang, PathBuf>,
    #[serde(default)]
    pub(crate) outline_dirs: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "Pack: Deserialize<'de>"))]
pub(crate) struct PackList<Pack> {
    #[serde(default)]
    pub(crate) packs: Vec<Pack>,
}

impl<Pack> Default for PackList<Pack> {
    fn default() -> Self {
        Self { packs: Vec::new() }
    }
}

macro_rules! pack_entry {
    (
        $(#[$entry_attr:meta])* $entry:ident {
            $(#[$dirs_attr:meta])* $dirs:ty,
            $($details:tt)*
        }
    ) => {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        $(#[$entry_attr])*
        pub(crate) struct $entry {
            pub(crate) module: String,
            $(#[$dirs_attr])*
            pub(crate) rule_dirs: $dirs,
            #[serde(default)]
            pub(crate) every_file: bool,
            $($details)*
        }
    };
}

pub(crate) use pack_entry;
