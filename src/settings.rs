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
//! project file sets wins over the user's file.

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

/// What the settings files say for one project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) style: Style,
}

impl Settings {
    /// The settings for the project in the folder `project`. A file that does not parse, a key
    /// that is no setting and a value that a setting does not take are errors that name the
    /// file.
    pub(crate) fn find(project: &Path) -> Result<Self> {
        Self::find_in(project, Path::new(""))
    }

    pub(crate) fn find_in(project: &Path, invocation_dir: &Path) -> Result<Self> {
        let project_style = match project_table(project)? {
            Some((file, doc)) => {
                let table = toml::get(&doc, &PROJECT_TABLE).and_then(|item| item.as_table_like());
                table
                    .map(|table| style(table, &file))
                    .transpose()?
                    .flatten()
            }
            None => None,
        };
        let style = match project_style {
            Some(style) => Some(style),
            None => user_style(invocation_dir)?,
        };
        Ok(Self {
            style: style.unwrap_or_default(),
        })
    }
}

/// The closest `pyproject.toml` at or above `project` that has a `[tool.xpd]` table.
fn project_table(project: &Path) -> Result<Option<(PathBuf, toml::Toml)>> {
    let project = std::path::absolute(project)?;
    for folder in project.ancestors() {
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
        Some(doc) => style(doc.as_table(), &file),
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

/// The style that `table`, the settings table of `file`, sets. `None` when it sets none.
fn style(table: &dyn TableLike, file: &Path) -> Result<Option<Style>> {
    let file = file.display();
    if let Some((key, _)) = table.iter().find(|(key, _)| *key != STYLE) {
        bail!("{file}: xpd has no setting named {key}. The setting it reads is {STYLE}");
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
