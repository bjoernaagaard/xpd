//! Pytest's TOML settings and the installed version from the project's virtual environment.

use std::path::{Path, PathBuf};

use super::catalog::{Catalog, Record, Suggestion, Target, Tool};
use super::toml::{self, Destination};
use crate::venv::Venv;

const STANDALONE: &[&str] = &["pytest"];
const NATIVE: &[&str] = &["tool", "pytest"];
const LEGACY: &[&str] = &["tool", "pytest", "ini_options"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Release {
    BeforeNine,
    NineOrLater,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ArgumentStyle {
    Native,
    Ini,
}

pub(crate) struct Pytest {
    pub(crate) installed: Option<(String, PathBuf)>,
    pub(crate) settings: Result<Option<Settings>, String>,
    pub(crate) notes: Vec<String>,
}

pub(crate) struct Settings {
    pub(crate) destination: Destination,
    pub(crate) config: Option<PathBuf>,
    args: Vec<String>,
    release: Release,
}

pub(crate) fn analyze(root: &Path) -> Pytest {
    let version = Venv::find_from(root).and_then(|venv| {
        let distributions = venv.distributions();
        let version = distributions.version("pytest")?.clone();
        Some((version, venv.root().to_path_buf()))
    });
    let nine = "9.0".parse().expect("9.0 is a release number");
    let release = if version.as_ref().is_some_and(|(version, _)| *version < nine) {
        Release::BeforeNine
    } else {
        Release::NineOrLater
    };
    let mut notes = ["pytest.ini", ".pytest.ini", "tox.ini", "setup.cfg"]
        .into_iter()
        .filter(|name| root.join(name).is_file())
        .map(|name| format!("{name} exists. Doctor did not read it because it reads TOML only."))
        .collect::<Vec<_>>();
    let settings = settings(root, release, &mut notes);
    Pytest {
        installed: version.map(|(version, path)| (version.to_string(), path)),
        settings,
        notes,
    }
}

fn settings(
    root: &Path,
    release: Release,
    notes: &mut Vec<String>,
) -> Result<Option<Settings>, String> {
    if release == Release::NineOrLater {
        for name in ["pytest.toml", ".pytest.toml"] {
            let file = root.join(name);
            if let Some(doc) = toml::read(&file)? {
                let config = Some(file.clone());
                return Settings::new(
                    Destination {
                        file,
                        prefix: STANDALONE,
                        doc: Some(doc),
                    },
                    config,
                    release,
                )
                .map(Some);
            }
        }
    }
    if ["pytest.ini", ".pytest.ini"]
        .into_iter()
        .any(|name| root.join(name).is_file())
    {
        return Ok(None);
    }
    let file = root.join("pyproject.toml");
    let doc = toml::read(&file)?;
    if release == Release::NineOrLater
        && let Some(table) = doc
            .as_ref()
            .and_then(|doc| toml::get(doc, NATIVE))
            .and_then(toml_edit::Item::as_table_like)
        && table.len() > 1
        && table
            .get("ini_options")
            .and_then(toml_edit::Item::as_table_like)
            .is_some_and(|ini| !ini.is_empty())
    {
        return Err(format!(
            "{}: [tool.pytest] and [tool.pytest.ini_options] both contain settings",
            file.display()
        ));
    }
    for prefix in [NATIVE, LEGACY] {
        let setting = doc.as_ref().and_then(|doc| toml::get(doc, prefix));
        if prefix == NATIVE
            && (release == Release::BeforeNine
                || setting.is_some_and(|item| {
                    item.as_table_like()
                        .is_some_and(|table| table.len() == 1 && table.contains_key("ini_options"))
                }))
        {
            continue;
        }
        if setting.is_some_and(toml_edit::Item::is_table_like) {
            let config = Some(file.clone());
            return Settings::new(Destination { file, prefix, doc }, config, release).map(Some);
        }
    }
    if ["tox.ini", "setup.cfg"]
        .into_iter()
        .any(|name| root.join(name).is_file())
    {
        notes.push("Doctor cannot tell whether a new pytest table would replace an existing configuration.".to_owned());
        return Ok(None);
    }
    let prefix = if release == Release::BeforeNine {
        LEGACY
    } else {
        NATIVE
    };
    Settings::new(Destination { file, prefix, doc }, None, release).map(Some)
}

impl Settings {
    fn new(
        destination: Destination,
        config: Option<PathBuf>,
        release: Release,
    ) -> Result<Self, String> {
        let Destination { file, prefix, doc } = &destination;
        let args = match doc.as_ref().and_then(|doc| {
            let path: Vec<&str> = prefix.iter().copied().chain(["addopts"]).collect();
            toml::get(doc, &path)
        }) {
            None => Some(Vec::new()),
            Some(item) if item.is_array() => item
                .as_array()
                .into_iter()
                .flatten()
                .map(|arg| arg.as_str().map(str::to_owned))
                .collect(),
            Some(item) if *prefix == LEGACY => item.as_str().and_then(shlex::split),
            Some(_) => None,
        }
        .ok_or_else(|| {
            format!(
                "{}: [{}] has invalid addopts",
                file.display(),
                prefix.join(".")
            )
        })?;
        Ok(Self {
            destination,
            config,
            args,
            release,
        })
    }

    pub(crate) fn suggestions<'a>(
        &'a self,
        catalog: &'a Catalog,
    ) -> impl Iterator<Item = (&'a Record, &'a Suggestion)> {
        catalog
            .suggestions()
            .filter(|(record, _)| record.key.tool == Tool::Pytest)
            .filter(|(record, _)| self.missing(&record.key.id))
    }

    fn missing(&self, id: &str) -> bool {
        match id {
            "strict" => self.release == Release::NineOrLater && !self.strict(),
            "--strict-config" | "--strict-markers" => {
                self.release == Release::BeforeNine
                    && !self.has_arg(id)
                    && !(id == "--strict-markers" && self.has_arg("--strict"))
            }
            "xfail_strict" => {
                self.release == Release::BeforeNine && self.flag("xfail_strict") != Some(true)
            }
            "-ra" => !self
                .arg_value("--report-chars", Some("-r"))
                .is_some_and(|value| {
                    value
                        .rsplit('N')
                        .next()
                        .is_some_and(|chars| chars.contains(['a', 'A']))
                }),
            "--import-mode=importlib" => self.arg_value("--import-mode", None) != Some("importlib"),
            _ => false,
        }
    }

    fn strict(&self) -> bool {
        let strict = self.flag("strict").unwrap_or(false) || self.has_arg("--strict");
        strict
            || [
                self.flag("strict_config").unwrap_or(false) || self.has_arg("--strict-config"),
                self.flag("strict_markers").unwrap_or(false) || self.has_arg("--strict-markers"),
                self.flag("strict_parametrization_ids").unwrap_or(false),
                self.flag("strict_xfail")
                    .or_else(|| self.flag("xfail_strict"))
                    .unwrap_or(false),
            ]
            .into_iter()
            .all(|enabled| enabled)
    }

    pub(crate) fn has_arg(&self, flag: &str) -> bool {
        self.args
            .iter()
            .take_while(|arg| *arg != "--")
            .any(|arg| arg == flag)
    }

    pub(crate) fn arg_value(&self, long: &str, short: Option<&str>) -> Option<&str> {
        let mut args = self
            .args
            .iter()
            .map(String::as_str)
            .take_while(|arg| *arg != "--");
        let mut value = None;
        while let Some(arg) = args.next() {
            if arg == long || Some(arg) == short {
                value = args.next();
            } else if let Some(found) = arg
                .strip_prefix(long)
                .and_then(|rest| rest.strip_prefix('='))
            {
                value = Some(found);
            } else if let Some(found) = short.and_then(|short| arg.strip_prefix(short)) {
                value = Some(found.trim_start_matches('='));
            }
        }
        value
    }

    pub(crate) fn flag(&self, key: &str) -> Option<bool> {
        let item = self.get(key)?;
        item.as_bool().or_else(|| {
            if self.destination.prefix != LEGACY {
                return None;
            }
            match item.as_str()?.to_ascii_lowercase().as_str() {
                "y" | "yes" | "t" | "true" | "on" | "1" => Some(true),
                "n" | "no" | "f" | "false" | "off" | "0" => Some(false),
                _ => None,
            }
        })
    }

    pub(crate) fn get(&self, key: &str) -> Option<&toml_edit::Item> {
        let doc = self.destination.doc.as_ref()?;
        let path: Vec<&str> = self
            .destination
            .prefix
            .iter()
            .copied()
            .chain([key])
            .collect();
        toml::get(doc, &path)
    }
}

/// How to edit addopts while preserving any other arguments.
pub(crate) fn add_arguments(
    placement: &toml::Placement,
    values: &[&str],
    style: ArgumentStyle,
) -> String {
    let joined = values.join(", ");
    if let Some(line) = placement.key_line {
        return if placement.array {
            format!("add {joined} to the existing addopts array at line {line}")
        } else {
            format!(
                "append {} inside the addopts string at line {line}",
                command_line(values)
            )
        };
    }
    if matches!(style, ArgumentStyle::Ini) {
        format!("addopts = {:?}", command_line(values))
    } else {
        format!("addopts = [{joined}]")
    }
}

pub(crate) fn argument_style(destination: &Destination) -> ArgumentStyle {
    if destination.prefix == LEGACY {
        ArgumentStyle::Ini
    } else {
        ArgumentStyle::Native
    }
}

pub(crate) fn is_addopts(tool: Tool, target: &Target) -> bool {
    tool == Tool::Pytest && matches!(target, Target::Setting { key, .. } if key == "addopts")
}

fn command_line(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| value.trim_matches('"'))
        .collect::<Vec<_>>()
        .join(" ")
}
