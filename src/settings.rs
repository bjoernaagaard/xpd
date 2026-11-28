//! The settings of xpd for one project: the `[tool.xpd]` table of the closest `pyproject.toml`
//! that has one, and below that the user's own file, `$XDG_CONFIG_HOME/xpd/config.toml`.
//!
//! ```toml
//! # pyproject.toml
//! [tool.xpd]
//! style = "strict"
//! ```
//!
//! The user's file holds the same keys at its top, as in `style = "strict"`. A key that the
//! project file sets wins over the user's file. The project file also holds the record of
//! `xpd doctor`'s profile, the `[tool.xpd.profile]` table, which doctor reads and checks.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use toml_edit::TableLike;

use crate::style::Style;
use crate::toml;

/// The file of a project that can hold xpd's settings.
const PROJECT_FILE: &str = "pyproject.toml";
/// The table of [`PROJECT_FILE`] that holds them.
const PROJECT_TABLE: [&str; 2] = ["tool", "xpd"];
/// The key of the house style.
const STYLE: &str = "style";
/// The key of the profile record, which only the project file holds.
const PROFILE: &str = "profile";
/// The table of local project library indexes, which only the project file holds.
const LIBRARIES: &str = "libraries";

/// What the settings files say for one project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) style: Style,
    pub(crate) libraries: BTreeMap<String, PathBuf>,
}

impl Settings {
    /// The settings for the project in the folder `project`. A file that does not parse, a key
    /// that is no setting and a value that a setting does not take are errors that name the
    /// file.
    pub(crate) fn find(project: &Path) -> Result<Self> {
        Self::find_in(project, Path::new(""))
    }

    pub(crate) fn find_in(project: &Path, invocation_dir: &Path) -> Result<Self> {
        let (project_style, libraries) = match project_table(project)? {
            Some((file, doc)) => {
                let table = toml::get(&doc, &PROJECT_TABLE).and_then(|item| item.as_table_like());
                match table {
                    Some(table) => {
                        let parsed_style = style(table, &file, &[STYLE, PROFILE, LIBRARIES])?;
                        let parsed_libraries = libraries(table, &file)?;
                        (parsed_style, parsed_libraries)
                    }
                    None => (None, BTreeMap::new()),
                }
            }
            None => (None, BTreeMap::new()),
        };
        let style = match project_style {
            Some(style) => Some(style),
            None => user_style(invocation_dir)?,
        };
        Ok(Self {
            style: style.unwrap_or_default(),
            libraries,
        })
    }
}

/// Parses `[tool.xpd.libraries]` entries as `package = "path/to/index.json"`, resolved against
/// the folder containing `file`.
fn libraries(table: &dyn TableLike, file: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let Some(item) = table.get(LIBRARIES) else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = item.as_table_like() else {
        let found = toml::Observed::of_item(item)
            .map_or_else(|| "an array of tables".to_owned(), |value| value.show());
        bail!(
            "{}: {LIBRARIES} is {found}, and it must be a table mapping package names to index file paths",
            file.display()
        );
    };
    let root = file.parent().unwrap_or_else(|| Path::new(""));
    let mut configured = BTreeMap::new();
    for (package, value) in entries.iter() {
        let Some(raw_path) = value.as_str() else {
            let found = toml::Observed::of_item(value)
                .map_or_else(|| "a table".to_owned(), |obs| obs.show());
            bail!(
                "{}: {LIBRARIES}.{package} is {found}, and it must be a string path to an index JSON file",
                file.display()
            );
        };
        if raw_path.trim().is_empty() {
            bail!(
                "{}: {LIBRARIES}.{package} must not be empty",
                file.display()
            );
        }
        let path = PathBuf::from(raw_path);
        let resolved = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        configured.insert(package.to_owned(), resolved);
    }
    Ok(configured)
}

/// The closest `pyproject.toml` at or above `project` that has a `[tool.xpd]` table.
fn project_table(project: &Path) -> Result<Option<(PathBuf, toml::Toml)>> {
    let bound = crate::project::boundary();
    let project = std::fs::canonicalize(project).or_else(|_| std::path::absolute(project))?;
    for folder in project
        .ancestors()
        .take_while(|dir| bound.as_deref().is_none_or(|root| dir.starts_with(root)))
    {
        let file = folder.join(PROJECT_FILE);
        if let Some(doc) = toml::read(&file).map_err(|error| anyhow!(error))?
            && toml::get(&doc, &PROJECT_TABLE).is_some()
        {
            return Ok(Some((file, doc)));
        }
    }
    Ok(None)
}

/// The style that the user's own file sets. `None` without such a file.
fn user_style(invocation_dir: &Path) -> Result<Option<Style>> {
    let Some(file) = user_file() else {
        return Ok(None);
    };
    match toml::read(&invocation_dir.join(&file)).map_err(|error| anyhow!(error))? {
        Some(doc) => style(doc.as_table(), &file, &[STYLE]),
        None => Ok(None),
    }
}

/// The user's settings file: `xpd/config.toml` in `$XDG_CONFIG_HOME`, or else in `.config` in
/// the home folder. On Windows the folder is `%APPDATA%`. `None` when the variables name no
/// folder.
fn user_file() -> Option<PathBuf> {
    let var = |name| env::var_os(name).filter(|value| !value.is_empty());
    let config = if let Some(config) = var("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else if cfg!(windows) {
        PathBuf::from(var("APPDATA")?)
    } else {
        PathBuf::from(var("HOME")?).join(".config")
    };
    Some(config.join("xpd").join("config.toml"))
}

/// The style that `table`, the settings table of `file`, sets. `None` when it sets none. A key
/// outside `keys` is an error.
fn style(table: &dyn TableLike, file: &Path, keys: &[&str]) -> Result<Option<Style>> {
    let file = file.display();
    if let Some((key, _)) = table.iter().find(|(key, _)| !keys.contains(key)) {
        bail!(
            "{file}: xpd has no setting named {key}. It reads {}",
            keys.join(" and ")
        );
    }
    let Some(item) = table.get(STYLE) else {
        return Ok(None);
    };
    let named = item
        .as_str()
        .and_then(|name| Style::ALL.into_iter().find(|style| style.as_str() == name));
    if let Some(style) = named {
        return Ok(Some(style));
    }
    let found = toml::Observed::of_item(item)
        .map_or_else(|| "an array of tables".to_owned(), |value| value.show());
    let allowed: Vec<String> = (Style::ALL.iter())
        .map(|style| format!("{:?}", style.as_str()))
        .collect();
    bail!(
        "{file}: {STYLE} is {found}, and the allowed values are {}",
        allowed.join(" and ")
    )
}
