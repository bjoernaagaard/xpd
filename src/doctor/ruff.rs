//! Ruff's effective settings for each group of files that one configuration file governs.
//!
//! Ruff uses the closest `.ruff.toml`, `ruff.toml` or `pyproject.toml` with a `[tool.ruff]`
//! table for each file, and a configuration inherits only through its `extend` key. When Ruff
//! runs here, `ruff check --show-settings FILE` gives the rules and settings it resolved.
//! Without Ruff, doctor resolves them from the files and the pinned release's defaults, and
//! labels the advice unverified.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::catalog::{self, Catalog, Decision, Key, Kind, Mode, Record, Surface, Tool};
use super::evidence::Location;
use super::placement::Destination;
use super::probe::Checker;
use crate::toml::{self, Observed, Toml};

/// The files that one configuration governs, and what Ruff runs on them.
#[derive(Debug)]
pub(crate) struct Scope {
    /// The governing file. `None` when no file configures Ruff, so its defaults apply.
    pub(crate) config: Option<PathBuf>,
    pub(crate) destination: Destination,
    /// Sorted.
    pub(crate) files: Vec<PathBuf>,
    pub(crate) state: Result<State, String>,
}

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) enabled: BTreeSet<String>,
    /// The rules an `ignore` selector of the project turns off, each with that selector.
    pub(crate) ignored: BTreeMap<String, Location>,
    /// The rules `per-file-ignores` turns off for some files, each with the patterns.
    pub(crate) per_file_ignored: BTreeMap<String, Vec<Location>>,
    pub(crate) preview: bool,
    /// The value of each setting that a suggestion names.
    pub(crate) settings: BTreeMap<String, Observed>,
    /// The file Ruff resolved the settings for, when Ruff ran.
    pub(crate) verified_with: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct Ruff {
    pub(crate) checker: Checker,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) notes: Vec<String>,
    /// The installed release's rules, code to name, when it is not the pinned release.
    pub(crate) installed_rules: Option<BTreeMap<String, String>>,
    /// Python files that Ruff leaves out, by `ruff check --show-files`.
    pub(crate) skipped: Vec<Location>,
}

/// Ruff's analysis of the `files` under `root`.
pub(crate) fn analyze(root: &Path, files: &[PathBuf], catalog: &Catalog, checker: Checker) -> Ruff {
    let keys: Vec<&str> = catalog.setting_keys(Tool::Ruff).collect();
    let mut groups: BTreeMap<Option<PathBuf>, Vec<PathBuf>> = BTreeMap::new();
    let mut governing: HashMap<PathBuf, Option<PathBuf>> = HashMap::new();
    for file in files {
        let dir = file.parent().unwrap_or(root).to_path_buf();
        let config = governing
            .entry(dir.clone())
            .or_insert_with(|| config_for(&dir))
            .clone();
        groups.entry(config).or_default().push(file.clone());
    }
    let mut notes = Vec::new();
    let scopes = groups
        .into_iter()
        .map(|(config, mut files)| {
            files.sort();
            let destination = destination(root, config.as_deref());
            let state = state(catalog, &checker, root, config.as_deref(), &files, &keys);
            Scope {
                config,
                destination,
                files,
                state,
            }
        })
        .collect::<Vec<_>>();
    let installed_rules = differences(catalog, &checker, root, &scopes, &mut notes);
    let skipped = skipped(&checker, root, files, &mut notes);
    Ruff {
        checker,
        scopes,
        notes,
        installed_rules,
        skipped,
    }
}

/// The closest file in or above `dir` that configures Ruff.
fn config_for(dir: &Path) -> Option<PathBuf> {
    for dir in dir.ancestors() {
        for name in [".ruff.toml", "ruff.toml"] {
            let path = dir.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        let pyproject = dir.join("pyproject.toml");
        if pyproject.is_file() {
            match toml::read(&pyproject) {
                Ok(Some(doc)) if toml::get(&doc, &["tool", "ruff"]).is_none() => {}
                // A pyproject that does not parse stops Ruff too, so its error is shown.
                _ => return Some(pyproject),
            }
        }
    }
    None
}

/// The table inside `file` that holds Ruff's settings.
fn prefix(file: &Path) -> &'static [&'static str] {
    if file
        .file_name()
        .is_some_and(|name| name == "pyproject.toml")
    {
        &["tool", "ruff"]
    } else {
        &[]
    }
}

/// The file a suggestion for the scope goes to: its configuration, or else the nearest
/// `pyproject.toml`, which may not exist yet.
fn destination(root: &Path, config: Option<&Path>) -> Destination {
    let file = config.map_or_else(
        || {
            root.ancestors()
                .map(|dir| dir.join("pyproject.toml"))
                .find(|path| path.is_file())
                .unwrap_or_else(|| root.join("pyproject.toml"))
        },
        Path::to_path_buf,
    );
    Destination {
        prefix: prefix(&file),
        doc: toml::read(&file).ok().flatten(),
        file,
    }
}

/// One configuration file of an `extend` chain.
struct Layer {
    file: PathBuf,
    doc: Toml,
    prefix: &'static [&'static str],
}

impl Layer {
    /// The lint option `key`, from the `lint` table or else from its old place at the top.
    fn lint<'a>(&'a self, key: &'a str) -> Option<(&'a toml_edit::Item, Vec<&'a str>)> {
        let canonical: Vec<&str> = self
            .prefix
            .iter()
            .copied()
            .chain(std::iter::once("lint"))
            .chain(key.split('.'))
            .collect();
        let legacy: Vec<&str> = self.prefix.iter().copied().chain(key.split('.')).collect();
        [canonical, legacy]
            .into_iter()
            .find_map(|path| toml::get(&self.doc, &path).map(|item| (item, path)))
    }

    fn selectors(&self, key: &str) -> Option<Vec<(String, Location)>> {
        let (item, path) = self.lint(key)?;
        let line = toml::line(&self.doc, &path).unwrap_or(1);
        Some(
            item.as_array()?
                .iter()
                .filter_map(|value| value.as_str())
                .map(|selector| {
                    let text = format!("{} = [\"{selector}\"]", path.join("."));
                    (selector.to_owned(), self.at(line, text))
                })
                .collect(),
        )
    }

    fn at(&self, line: usize, text: String) -> Location {
        Location {
            file: self.file.clone(),
            line,
            text,
        }
    }
}

/// The configuration at `config` and the files it extends, the base first.
fn chain(config: &Path) -> Result<Vec<Layer>, String> {
    let mut layers = Vec::new();
    let mut next = Some(config.to_path_buf());
    while let Some(file) = next.take() {
        if layers.iter().any(|layer: &Layer| layer.file == file) || layers.len() > 32 {
            return Err(format!("{}: the extend chain loops", file.display()));
        }
        let doc = toml::read(&file)?.ok_or_else(|| format!("{}: no such file", file.display()))?;
        let prefix = prefix(&file);
        let path: Vec<&str> = prefix.iter().copied().chain(["extend"]).collect();
        if let Some(base) = toml::get(&doc, &path).and_then(|item| item.as_str()) {
            let base = shellexpand(base);
            next = Some(
                file.parent()
                    .map_or_else(|| base.clone(), |dir| dir.join(&base)),
            );
        }
        layers.push(Layer { file, doc, prefix });
    }
    layers.reverse();
    Ok(layers)
}

/// `path` with a leading `~` for the home folder expanded, as Ruff does for `extend`.
fn shellexpand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// The linter part of a rule's selector prefix: `PL` for Pylint's `PLC`, `PLE`, `PLR` and `PLW`.
fn linter(prefix: &str) -> &str {
    if prefix.len() == 3 && prefix.starts_with("PL") {
        "PL"
    } else {
        prefix
    }
}

/// Whether `selector` selects `rule`, as a code, a prefix of codes in its linter, a legacy
/// group, `ALL`, or the rule's name, which Ruff accepts in preview mode.
pub(crate) fn selects(selector: &str, rule: &Record) -> bool {
    if selector == "ALL" || rule.name.as_deref() == Some(selector) {
        return true;
    }
    let Some(prefix) = rule.prefix.as_deref() else {
        return selector == rule.key.id;
    };
    match selector {
        "C" => matches!(prefix, "C4" | "C90"),
        "T" => matches!(prefix, "T10" | "T20"),
        _ => selector.starts_with(linter(prefix)) && rule.key.id.starts_with(selector),
    }
}

/// How Ruff runs preview rules.
#[derive(Debug, Clone, Copy)]
struct Preview {
    mode: Mode,
    /// `explicit-preview-rules`: a preview rule runs only when a selector names its code.
    explicit: bool,
}

/// The rules that run before the first layer's selectors apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Baseline {
    /// The pinned release's default selection, which another release may change.
    Defaults,
    /// None, so that only the configuration's own selectors count.
    Empty,
}

/// Resolves selection layer by layer: a `select` starts over, and within a layer the more
/// specific selector wins, an ignore over a select of the same length.
fn selection(
    catalog: &Catalog,
    layers: &[Layer],
    preview: Preview,
    baseline: Baseline,
) -> (BTreeSet<String>, BTreeMap<String, Location>) {
    let rules: Vec<&Record> = catalog
        .ruff_rules()
        .filter(|rule| !rule.is_removed() && (preview.mode == Mode::Preview || !rule.is_preview()))
        .collect();
    let mut enabled: BTreeSet<String> = rules
        .iter()
        .filter(|rule| baseline == Baseline::Defaults && rule.ruff_default(preview.mode))
        .map(|rule| rule.key.id.clone())
        .collect();
    let mut ignored = BTreeMap::new();
    for layer in layers {
        let select = layer.selectors("select");
        if select.is_some() {
            enabled.clear();
        }
        let mut entries: Vec<(usize, bool, String, Location)> = Vec::new();
        for (selector, location) in select
            .into_iter()
            .flatten()
            .chain(layer.selectors("extend-select").into_iter().flatten())
        {
            entries.push((specificity(&selector), false, selector, location));
        }
        for key in ["ignore", "extend-ignore"] {
            for (selector, location) in layer.selectors(key).into_iter().flatten() {
                entries.push((specificity(&selector), true, selector, location));
            }
        }
        entries.sort_by_key(|(specificity, ignore, _, _)| (*specificity, *ignore));
        for (_, ignore, selector, location) in entries {
            let Some(selector) = catalog.redirect(&selector) else {
                continue;
            };
            for rule in &rules {
                let exact = rule.key.id == selector;
                if !selects(selector, rule) || (rule.is_preview() && preview.explicit && !exact) {
                    continue;
                }
                if ignore {
                    enabled.remove(&rule.key.id);
                    ignored.insert(rule.key.id.clone(), location.clone());
                } else {
                    enabled.insert(rule.key.id.clone());
                    ignored.remove(&rule.key.id);
                }
            }
        }
    }
    (enabled, ignored)
}

/// The rules that the selectors of `doc`, the configuration file `file`, turn on, and the
/// selector that turns off each rule the file ignores. A rule that only a default selection
/// runs does not count, because the defaults differ between Ruff releases.
pub(crate) fn resolve(
    catalog: &Catalog,
    file: &Path,
    doc: Toml,
) -> (BTreeSet<String>, BTreeMap<String, Location>) {
    let layers = [Layer {
        file: file.to_path_buf(),
        prefix: prefix(file),
        doc,
    }];
    let flag = |key| matches!(lint_value(&layers, key), Some((Observed::Bool(true), _)));
    let mode = if flag("preview") {
        Mode::Preview
    } else {
        Mode::Stable
    };
    let explicit = flag("explicit-preview-rules");
    selection(
        catalog,
        &layers,
        Preview { mode, explicit },
        Baseline::Empty,
    )
}

fn specificity(selector: &str) -> usize {
    if selector == "ALL" { 0 } else { selector.len() }
}

/// The last value that a layer sets for the lint option `key`, with the place it is set.
fn lint_value(layers: &[Layer], key: &str) -> Option<(Observed, Location)> {
    layers.iter().rev().find_map(|layer| {
        let (item, path) = layer.lint(key)?;
        let observed = Observed::of_item(item)?;
        let line = toml::line(&layer.doc, &path).unwrap_or(1);
        let text = format!("{} = {}", path.join("."), observed.show());
        Some((observed, layer.at(line, text)))
    })
}

fn state(
    catalog: &Catalog,
    checker: &Checker,
    root: &Path,
    config: Option<&Path>,
    files: &[PathBuf],
    keys: &[&str],
) -> Result<State, String> {
    let layers = match config {
        Some(config) => chain(config)?,
        None => Vec::new(),
    };
    let flag = |key| matches!(lint_value(&layers, key), Some((Observed::Bool(true), _)));
    // `lint.preview`, or else the top-level `preview`, which also applies to the linter.
    let preview = flag("preview");
    let mode = if preview { Mode::Preview } else { Mode::Stable };
    let explicit = flag("explicit-preview-rules");
    let (mut enabled, ignored) = selection(
        catalog,
        &layers,
        Preview { mode, explicit },
        Baseline::Defaults,
    );
    let mut settings = BTreeMap::new();
    for key in keys {
        let lint_key = key.strip_prefix("lint.").unwrap_or(key);
        let observed = lint_value(&layers, lint_key)
            .map(|(observed, _)| observed)
            .or_else(|| default(catalog, key));
        if let Some(observed) = observed {
            settings.insert((*key).to_owned(), observed);
        }
    }
    let mut verified_with = None;
    let representative = files.first().ok_or("a scope without files")?;
    let relative = representative.strip_prefix(root).unwrap_or(representative);
    if let Some(output) = checker.run(
        &[
            std::ffi::OsStr::new("check"),
            "--show-settings".as_ref(),
            relative.as_os_str(),
        ],
        root,
    ) {
        let shown = output.map_err(|error| {
            format!("ruff check --show-settings {}: {error}", relative.display())
        })?;
        let mut resolved = ShowSettings::parse(&shown);
        enabled = std::mem::take(&mut resolved.enabled);
        for key in keys {
            if let Some(observed) = resolved.get(key) {
                settings.insert((*key).to_owned(), observed);
            }
        }
        verified_with = Some(relative.to_path_buf());
    }
    let per_file_ignored = per_file_ignores(catalog, &layers);
    Ok(State {
        enabled,
        ignored,
        per_file_ignored,
        preview,
        settings,
        verified_with,
    })
}

/// The documented default of the setting `key` in the pinned release.
fn default(catalog: &Catalog, key: &str) -> Option<Observed> {
    let record = catalog.get(&Key::pinned(
        Tool::Ruff,
        Surface::Project,
        Kind::Option,
        key,
    ))?;
    Observed::parse(record.default.as_deref()?.trim_matches('`'))
}

/// The rules that `per-file-ignores` and `extend-per-file-ignores` turn off, with each pattern.
fn per_file_ignores(catalog: &Catalog, layers: &[Layer]) -> BTreeMap<String, Vec<Location>> {
    let mut ignored: BTreeMap<String, Vec<Location>> = BTreeMap::new();
    for layer in layers {
        for key in ["per-file-ignores", "extend-per-file-ignores"] {
            let Some((item, path)) = layer.lint(key) else {
                continue;
            };
            let line = toml::line(&layer.doc, &path).unwrap_or(1);
            let Some(table) = item.as_table_like() else {
                continue;
            };
            for (pattern, selectors) in table.iter() {
                let selectors = selectors
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|value| value.as_str());
                for selector in selectors {
                    let Some(selector) = catalog.redirect(selector) else {
                        continue;
                    };
                    for rule in catalog.ruff_rules().filter(|rule| selects(selector, rule)) {
                        let text = format!("{} \"{pattern}\" = [\"{selector}\"]", path.join("."));
                        ignored
                            .entry(rule.key.id.clone())
                            .or_default()
                            .push(layer.at(line, text));
                    }
                }
            }
        }
    }
    ignored
}

/// What `ruff check --show-settings` printed: its enabled rules and its settings, by name.
#[derive(Debug, Default)]
struct ShowSettings {
    enabled: BTreeSet<String>,
    values: HashMap<String, String>,
}

impl ShowSettings {
    fn parse(text: &str) -> Self {
        let mut shown = Self::default();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            let Some((name, value)) = line.split_once(" = ") else {
                continue;
            };
            let value = if matches!(value, "[" | "{") {
                let close = if value == "[" { "]" } else { "}" };
                let block: Vec<&str> = lines.by_ref().take_while(|l| *l != close).collect();
                format!("{value}\n{}\n{close}", block.join("\n"))
            } else {
                value.to_owned()
            };
            if name == "linter.rules.enabled" {
                shown.enabled = value
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim().trim_end_matches(',');
                        let code = line
                            .rsplit_once(" (")
                            .map_or(line, |(_, code)| code.trim_end_matches(')'));
                        (!code.is_empty() && !matches!(code, "[" | "]")).then(|| code.to_owned())
                    })
                    .collect();
            }
            shown.values.insert(name.to_owned(), value);
        }
        shown
    }

    /// The value of the TOML setting `key`, such as `lint.flake8-tidy-imports.ban-lazy`.
    fn get(&self, key: &str) -> Option<Observed> {
        let name = format!(
            "linter.{}",
            key.strip_prefix("lint.").unwrap_or(key).replace('-', "_")
        );
        let value = self.values.get(&name)?;
        Some(match value.as_str() {
            "true" | "enabled" => Observed::Bool(true),
            "false" | "disabled" => Observed::Bool(false),
            block if block.starts_with('[') || block.starts_with('{') => Observed::Items(
                block
                    .lines()
                    .skip(1)
                    .filter(|line| !matches!(line.trim(), "]" | "}" | ""))
                    .map(|line| {
                        let item = line.trim().trim_end_matches(',');
                        let item = item.split_once(" = ").map_or(item, |(key, _)| key);
                        item.trim_matches('"').to_owned()
                    })
                    .collect(),
            ),
            text => Observed::Text(text.trim_matches('"').to_owned()),
        })
    }
}

/// The installed release's rules, code to name, by `ruff rule --all`. A rule without a code
/// is keyed by its name.
fn installed_rules(
    checker: &Checker,
    root: &Path,
) -> Option<Result<BTreeMap<String, String>, String>> {
    let json = match checker.run(&["rule", "--all", "--output-format", "json"], root)? {
        Ok(json) => json,
        Err(error) => return Some(Err(error)),
    };
    let rules: Vec<serde_json::Value> = match serde_json::from_str(&json) {
        Ok(rules) => rules,
        Err(error) => return Some(Err(error.to_string())),
    };
    Some(Ok(rules
        .iter()
        .map(|rule| {
            let name = rule["name"].as_str().unwrap_or_default().to_owned();
            let code = rule["code"]
                .as_str()
                .filter(|code| !code.is_empty())
                .map_or_else(|| name.clone(), str::to_owned);
            (code, name)
        })
        .collect()))
}

/// When the installed Ruff is another release: its rules, and notes on what changed.
fn differences(
    catalog: &Catalog,
    checker: &Checker,
    root: &Path,
    scopes: &[Scope],
    notes: &mut Vec<String>,
) -> Option<BTreeMap<String, String>> {
    if checker.installed.is_none() || checker.is_pinned() {
        return None;
    }
    let version = &checker.installed.as_ref()?.version;
    let installed = match installed_rules(checker, root)? {
        Ok(installed) => installed,
        Err(error) => {
            notes.push(format!(
                "ruff rule --all failed, so its rules are unknown: {error}"
            ));
            return None;
        }
    };
    let pinned: BTreeMap<&str, &Record> = catalog
        .ruff_rules()
        .map(|rule| (rule.key.id.as_str(), rule))
        .collect();
    let newer: Vec<String> = installed
        .keys()
        .filter(|code| !pinned.contains_key(code.as_str()))
        .map(|code| {
            let live = catalog.get(&Key {
                release: version.clone(),
                ..Key::pinned(Tool::Ruff, Surface::Rules, Kind::Newer, code)
            });
            match live.map(|record| &record.decision) {
                Some(Decision::Leave) => format!("{code} (reviewed for {version}: leave)"),
                _ => code.clone(),
            }
        })
        .collect();
    if !newer.is_empty() {
        notes.push(format!(
            "ruff {version} has {} rules that ruff {} lacks, which doctor leaves alone: {}",
            newer.len(),
            catalog::RUFF_RELEASE,
            newer.join(", ")
        ));
    }
    let renamed: Vec<String> = installed
        .iter()
        .filter_map(|(code, name)| {
            let rule = pinned.get(code.as_str())?;
            (rule.name.as_deref() != Some(name)).then(|| {
                format!(
                    "{code} is {name} in {version}, {} in {}",
                    rule.name.as_deref().unwrap_or_default(),
                    catalog::RUFF_RELEASE
                )
            })
        })
        .collect();
    if !renamed.is_empty() {
        notes.push(format!("renamed rules: {}", renamed.join("; ")));
    }
    let missing: Vec<&str> = pinned
        .iter()
        .filter(|(code, rule)| !rule.is_removed() && !installed.contains_key(**code))
        .map(|(code, _)| *code)
        .collect();
    if !missing.is_empty() {
        notes.push(format!(
            "ruff {version} lacks {} rules of {}: {}",
            missing.len(),
            catalog::RUFF_RELEASE,
            missing.join(", ")
        ));
    }
    if let Some(file) = scopes.iter().find_map(|scope| scope.files.first()) {
        let changed = changed_defaults(checker, root, file, &pinned, &installed);
        if !changed.is_empty() {
            notes.push(format!(
                "default selection changed in ruff {version}: {}",
                changed.join(", ")
            ));
        }
    }
    Some(installed)
}

/// The rules whose stable default selection differs between the installed release, by
/// `ruff check --isolated --show-settings`, and the pinned one.
fn changed_defaults(
    checker: &Checker,
    root: &Path,
    file: &Path,
    pinned: &BTreeMap<&str, &Record>,
    installed: &BTreeMap<String, String>,
) -> Vec<String> {
    let relative = file.strip_prefix(root).unwrap_or(file);
    let args = [
        std::ffi::OsStr::new("check"),
        "--isolated".as_ref(),
        "--show-settings".as_ref(),
        relative.as_os_str(),
    ];
    let Some(Ok(shown)) = checker.run(&args, root) else {
        return Vec::new();
    };
    let defaults = ShowSettings::parse(&shown).enabled;
    pinned
        .iter()
        .filter(|(code, _)| installed.contains_key(**code))
        .filter_map(|(code, rule)| {
            let before = rule.ruff_default(Mode::Stable);
            (before != defaults.contains(*code))
                .then(|| format!("{code} {}", if before { "off" } else { "on" }))
        })
        .collect()
}

/// The Python files that xpd checks and `ruff check --show-files` leaves out.
fn skipped(
    checker: &Checker,
    root: &Path,
    files: &[PathBuf],
    notes: &mut Vec<String>,
) -> Vec<Location> {
    let Some(output) = checker.run(&["check", "--show-files", "."], root) else {
        return Vec::new();
    };
    let listed: BTreeSet<PathBuf> = match output {
        Ok(text) => text.lines().map(PathBuf::from).collect(),
        Err(error) => {
            notes.push(format!("ruff check --show-files failed: {error}"));
            return Vec::new();
        }
    };
    files
        .iter()
        .filter(|file| !listed.contains(*file))
        .map(|file| Location {
            file: file.clone(),
            line: 1,
            text: "ruff check --show-files . does not list this file".to_owned(),
        })
        .collect()
}
