//! Deterministic file intent zones and `pyproject.toml` entry points for `xpd untangle`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use toml_edit::{Document, Item};

use crate::source;

/// Intent zone of a Python source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Zone {
    Production,
    Test,
    Config,
    Generated,
    Script,
    Vendor,
}

impl Zone {
    /// Whether this file belongs to generated or vendored code that should not be refactored.
    pub(crate) fn is_generated_or_vendor(self) -> bool {
        matches!(self, Self::Generated | Self::Vendor)
    }
}

/// Classifies `path` into an intent zone using path components and conventional filenames.
pub(crate) fn classify(path: &Path) -> Zone {
    let name = path
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or("");
    let has_dir = |names: &[&str]| {
        path.components()
            .filter_map(|part| part.as_os_str().to_str())
            .any(|part| names.contains(&part))
    };

    if has_dir(&["vendor", "third_party", "vendored"]) {
        return Zone::Vendor;
    }
    if has_dir(&["generated", "__generated__", "migrations"])
        || name.ends_with("_pb2.py")
        || name.ends_with("_pb2_grpc.py")
    {
        return Zone::Generated;
    }
    if source::is_test(path)
        || has_dir(&[
            "test",
            "fixtures",
            "factories",
            "samples",
            "testdata",
            "snapshots",
        ])
    {
        return Zone::Test;
    }
    if matches!(
        name,
        "setup.py"
            | "manage.py"
            | "wsgi.py"
            | "asgi.py"
            | "settings.py"
            | "config.py"
            | "noxfile.py"
            | "fabfile.py"
    ) {
        return Zone::Config;
    }
    if name == "__main__.py"
        || has_dir(&[
            "scripts",
            "bin",
            "tools",
            "commands",
            "examples",
            "benchmarks",
            "experiments",
        ])
    {
        return Zone::Script;
    }
    Zone::Production
}

/// Dotted module names and callables declared as package entry points in `pyproject.toml`.
#[derive(Debug, Default)]
pub(crate) struct EntryPoints {
    modules: HashSet<String>,
    callables: HashMap<String, HashSet<String>>,
}

impl EntryPoints {
    /// Parses `[project.scripts]`, `[project.gui-scripts]`, `[project.entry-points]`, and
    /// `[tool.poetry.scripts]` from `pyproject.toml` text. Invalid TOML yields an empty set.
    pub(crate) fn from_pyproject(text: &str) -> Self {
        let Ok(doc) = text.parse::<Document<String>>() else {
            return Self::default();
        };
        let mut entry_points = Self::default();
        if let Some(project) = doc.get("project").and_then(Item::as_table_like) {
            collect_script_table(project.get("scripts"), &mut entry_points);
            collect_script_table(project.get("gui-scripts"), &mut entry_points);
            if let Some(groups) = project.get("entry-points").and_then(Item::as_table_like) {
                for (_, group) in groups.iter() {
                    collect_script_table(Some(group), &mut entry_points);
                }
            }
        }
        if let Some(poetry) = doc
            .get("tool")
            .and_then(Item::as_table_like)
            .and_then(|tool| tool.get("poetry"))
            .and_then(Item::as_table_like)
        {
            collect_script_table(poetry.get("scripts"), &mut entry_points);
        }
        entry_points
    }

    /// Whether `dotted` matches a declared entry-point module.
    pub(crate) fn contains(&self, dotted: &str) -> bool {
        self.modules.contains(dotted)
    }

    /// Whether `name` in module `dotted` is declared as an entry-point callable.
    pub(crate) fn is_callable(&self, dotted: &str, name: &str) -> bool {
        self.callables
            .get(dotted)
            .is_some_and(|names| names.contains(name))
    }
}

/// Extracts dotted module names and optional `:attr` callables from a script table.
fn collect_script_table(item: Option<&Item>, entry_points: &mut EntryPoints) {
    let Some(table) = item.and_then(Item::as_table_like) else {
        return;
    };
    for (_, value) in table.iter() {
        let Some(spec) = value.as_str() else {
            continue;
        };
        let spec = spec.split('[').next().unwrap_or("").trim();
        let (module, attr) = match spec.split_once(':') {
            Some((module, attr)) => (module.trim(), Some(attr.trim())),
            None => (spec, None),
        };
        if module.is_empty() {
            continue;
        }
        entry_points.modules.insert(module.to_owned());
        if let Some(attr) = attr
            && let Some(root_attr) = attr.split('.').next()
            && !root_attr.is_empty()
        {
            entry_points
                .callables
                .entry(module.to_owned())
                .or_default()
                .insert(root_attr.to_owned());
        }
    }
}
