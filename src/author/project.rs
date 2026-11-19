//! A rule project on disk: `sgconfig.yml` for ast-grep, `xpd.yml` for xpd's packs, and the
//! folders they name. Every path `xpd new` reads or writes stays inside the project folder.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use ast_grep_config::{DeserializeEnv, GlobalRules, SerializableGlobalRule};
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::document::{self, Name, RuleDoc, RuleId};

const SGCONFIG: &str = "sgconfig.yml";
const SGCONFIG_YAML: &str = "sgconfig.yaml";
const XPD_CONFIG: &str = "xpd.yml";
const RULES: &str = "rules";
const TESTS: &str = "rule-tests";
const UTILS: &str = "utils";

/// What xpd reads of `sgconfig.yml`. ast-grep owns the file, so other keys are kept as they are.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SgConfig {
    #[serde(default)]
    rule_dirs: Vec<String>,
    #[serde(default)]
    util_dirs: Vec<String>,
    #[serde(default)]
    test_configs: Vec<TestConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestConfig {
    test_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewSgConfig<'a> {
    rule_dirs: [&'a str; 1],
    test_configs: [TestConfig; 1],
    util_dirs: [&'a str; 1],
}

/// What `xpd new` reads of `xpd.yml`.
#[derive(Debug, Default, Deserialize)]
struct XpdConfig {
    #[serde(default)]
    packs: Vec<PackConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackConfig {
    module: String,
    #[serde(default)]
    rule_dirs: Vec<String>,
    #[serde(default)]
    every_file: bool,
}

/// A pack entry of `xpd.yml` as `xpd new` writes one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackEntry<'a> {
    module: &'a str,
    rule_dirs: &'a [String],
    every_file: bool,
    aliases: BTreeMap<String, String>,
    members: BTreeMap<String, String>,
    packages: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct NewXpdConfig<'a> {
    packs: Vec<PackEntry<'a>>,
}

/// Which Python files a pack's rules run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::author) enum Gate {
    /// The files that import the pack's module.
    Import,
    EveryFile,
}

impl From<bool> for Gate {
    /// The gate of a pack whose `everyFile` is `every_file`.
    fn from(every_file: bool) -> Self {
        if every_file {
            Self::EveryFile
        } else {
            Self::Import
        }
    }
}

impl fmt::Display for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Import => "the files that import the module",
            Self::EveryFile => "every Python file",
        })
    }
}

/// A rule pack of the project: rules about one Python module.
#[derive(Debug, Clone)]
pub(in crate::author) struct Pack {
    pub(in crate::author) module: String,
    rule_dirs: Vec<String>,
    gate: Gate,
    /// Whether `xpd.yml` already lists the pack.
    declared: bool,
}

impl Pack {
    /// A pack for a new project, with its rules in `rules/<module>`.
    pub(in crate::author) fn new(module: String, gate: Gate) -> Self {
        Self {
            rule_dirs: vec![format!("{RULES}/{module}")],
            module,
            gate,
            declared: false,
        }
    }

    fn entry(&self) -> PackEntry<'_> {
        PackEntry {
            module: &self.module,
            rule_dirs: &self.rule_dirs,
            every_file: self.gate == Gate::EveryFile,
            aliases: BTreeMap::new(),
            members: BTreeMap::new(),
            packages: Vec::new(),
        }
    }
}

/// A rule of the project, as its file names it.
#[derive(Debug)]
struct ExistingRule {
    id: String,
    public_id: Option<String>,
}

/// The files one `xpd new` writes, all checked before the first is written.
#[derive(Debug, Default)]
pub(in crate::author) struct Changes {
    create: Vec<(PathBuf, String)>,
    replace: Vec<(PathBuf, String)>,
}

impl Changes {
    fn stage(&mut self, path: PathBuf, text: String, previous: Option<&String>) {
        match previous {
            Some(_) => self.replace.push((path, text)),
            None => self.create.push((path, text)),
        }
    }

    /// Writes the files, and returns the created ones and the modified ones. Fails before
    /// writing anything when a file to create exists, and never follows a link to create one.
    pub(in crate::author) fn apply(self) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
        for (path, _) in &self.create {
            ensure!(
                fs::symlink_metadata(path).is_err(),
                "{} already exists",
                path.display()
            );
        }
        let mut created = Vec::new();
        for (path, text) in self.create {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create {}", parent.display()))?;
            }
            let write = || -> std::io::Result<()> {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                file.write_all(text.as_bytes())
            };
            write().with_context(|| format!("cannot create {}", path.display()))?;
            created.push(path);
        }
        let mut modified = Vec::new();
        for (path, text) in self.replace {
            fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
            modified.push(path);
        }
        Ok((created, modified))
    }
}

/// A rule project, found from its `sgconfig.yml`.
#[derive(Debug)]
pub(in crate::author) struct Project {
    /// The project folder, with every link resolved.
    root: PathBuf,
    config: SgConfig,
    packs: Vec<Pack>,
    /// The text of `xpd.yml`, when the project has one.
    xpd_config: Option<String>,
}

impl Project {
    /// The `sgconfig.yml` in `dir` or the nearest folder above it.
    pub(in crate::author) fn config_at_or_above(dir: &Path) -> Option<PathBuf> {
        dir.ancestors()
            .flat_map(|dir| [dir.join(SGCONFIG), dir.join(SGCONFIG_YAML)])
            .find(|path| path.is_file())
    }

    /// The project whose `sgconfig.yml` is in `dir`.
    pub(in crate::author) fn find(dir: &Path) -> Result<Self> {
        let Some(config_path) = [SGCONFIG, SGCONFIG_YAML]
            .iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
        else {
            bail!(
                "{} has no {SGCONFIG}. Create a project with xpd new project",
                dir.display()
            );
        };
        let root = dir
            .canonicalize()
            .with_context(|| format!("cannot read {}", dir.display()))?;
        let text = fs::read_to_string(&config_path)
            .with_context(|| format!("cannot read {}", config_path.display()))?;
        let config: SgConfig = ast_grep_config::from_str(&text)
            .with_context(|| format!("{} does not parse", config_path.display()))?;
        let xpd_path = root.join(XPD_CONFIG);
        let xpd_config = xpd_path
            .is_file()
            .then(|| fs::read_to_string(&xpd_path))
            .transpose()
            .with_context(|| format!("cannot read {}", xpd_path.display()))?;
        let packs: XpdConfig = match &xpd_config {
            Some(text) => ast_grep_config::from_str::<Option<XpdConfig>>(text)
                .with_context(|| format!("{} does not parse", xpd_path.display()))?
                .unwrap_or_default(),
            None => XpdConfig::default(),
        };
        let packs = packs
            .packs
            .into_iter()
            .map(|pack| Pack {
                module: pack.module,
                rule_dirs: pack.rule_dirs,
                gate: Gate::from(pack.every_file),
                declared: true,
            })
            .collect();
        Ok(Self {
            root,
            config,
            packs,
            xpd_config,
        })
    }

    /// The files of a new project in `dir`, with the packs in `xpd.yml`.
    pub(in crate::author) fn create(dir: &Path, packs: &[Pack]) -> Result<Changes> {
        for name in [SGCONFIG, SGCONFIG_YAML, XPD_CONFIG] {
            ensure!(
                fs::symlink_metadata(dir.join(name)).is_err(),
                "{} already has {name}",
                dir.display()
            );
        }
        let sgconfig = serde_saphyr::to_string(&NewSgConfig {
            rule_dirs: [RULES],
            test_configs: [TestConfig {
                test_dir: TESTS.to_owned(),
            }],
            util_dirs: [UTILS],
        })?;
        let read: SgConfig = ast_grep_config::from_str(&sgconfig)?;
        ensure!(
            read.rule_dirs == [RULES]
                && read.util_dirs == [UTILS]
                && read
                    .test_configs
                    .iter()
                    .map(|test| test.test_dir.as_str())
                    .eq([TESTS]),
            "the new {SGCONFIG} does not keep its folders"
        );
        let xpd_config = serde_saphyr::to_string(&NewXpdConfig {
            packs: packs.iter().map(Pack::entry).collect(),
        })?;
        let read: XpdConfig = ast_grep_config::from_str(&xpd_config)?;
        ensure!(
            read.packs
                .iter()
                .map(|pack| &pack.module)
                .eq(packs.iter().map(|pack| &pack.module)),
            "the new {XPD_CONFIG} does not keep its packs"
        );
        let mut changes = Changes::default();
        changes.stage(inside(dir, SGCONFIG)?, sgconfig, None);
        changes.stage(inside(dir, XPD_CONFIG)?, xpd_config, None);
        let folders = [RULES, TESTS, UTILS]
            .map(str::to_owned)
            .into_iter()
            .chain(packs.iter().flat_map(|pack| pack.rule_dirs.clone()));
        for folder in folders {
            let keep = inside(dir, &format!("{folder}/.gitkeep"))?;
            if fs::symlink_metadata(&keep).is_err() {
                changes.stage(keep, String::new(), None);
            }
        }
        Ok(changes)
    }

    pub(in crate::author) fn packs(&self) -> &[Pack] {
        &self.packs
    }

    /// The declared pack for `module`, or a new one in the first rules folder.
    pub(in crate::author) fn pack_or_new(&self, module: &str, gate: Gate) -> Pack {
        if let Some(pack) = self.packs.iter().find(|pack| pack.module == module) {
            return pack.clone();
        }
        let base = self.config.rule_dirs.first().map_or(RULES, String::as_str);
        Pack {
            module: module.to_owned(),
            rule_dirs: vec![format!("{}/{module}", base.trim_end_matches('/'))],
            gate,
            declared: false,
        }
    }

    pub(in crate::author) fn has_tests(&self) -> bool {
        !self.config.test_configs.is_empty()
    }

    pub(in crate::author) fn test_dirs(&self) -> Vec<String> {
        let tests = self.config.test_configs.iter();
        tests.map(|test| test.test_dir.clone()).collect()
    }

    pub(in crate::author) fn util_dirs(&self) -> &[String] {
        &self.config.util_dirs
    }

    fn inside(&self, relative: &str) -> Result<PathBuf> {
        inside(&self.root, relative)
    }

    /// The .yml and .yaml files under the folders, each with its text.
    fn yaml_files(&self, dirs: &[String]) -> Result<Vec<(PathBuf, String)>> {
        let mut files = Vec::new();
        for dir in dirs {
            let dir = self.inside(dir)?;
            if !dir.is_dir() {
                continue;
            }
            for entry in ignore::WalkBuilder::new(&dir).parents(false).build() {
                let path = entry?.into_path();
                let yaml = path
                    .extension()
                    .is_some_and(|extension| extension == "yml" || extension == "yaml");
                if yaml && path.is_file() {
                    let real = path.canonicalize()?;
                    ensure!(
                        real.starts_with(&self.root),
                        "{} leads outside the project folder through a link",
                        path.display()
                    );
                    let text = fs::read_to_string(&path)
                        .with_context(|| format!("cannot read {}", path.display()))?;
                    files.push((path, text));
                }
            }
        }
        Ok(files)
    }

    fn existing_rules(&self) -> Result<Vec<ExistingRule>> {
        let mut rules = Vec::new();
        for (path, text) in self.yaml_files(&self.config.rule_dirs)? {
            let documents: Vec<Value> = ast_grep_config::from_multiple(&text)
                .with_context(|| format!("{} does not parse", path.display()))?;
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            for document in documents {
                let id = document.get("id").and_then(Value::as_str).unwrap_or(stem);
                let public_id = document
                    .pointer("/metadata/publicId")
                    .and_then(Value::as_str);
                rules.push(ExistingRule {
                    id: id.to_owned(),
                    public_id: public_id.map(str::to_owned),
                });
            }
        }
        Ok(rules)
    }

    /// The id and public id of every rule, which a test case may name.
    pub(in crate::author) fn rule_ids(&self) -> Result<Vec<String>> {
        let mut ids: Vec<String> = self
            .existing_rules()?
            .into_iter()
            .flat_map(|rule| std::iter::once(rule.id).chain(rule.public_id))
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    /// Fails when a rule already has the id, as its own id or as a public id.
    pub(in crate::author) fn check_new_rule_id(&self, id: &RuleId) -> Result<()> {
        let id = id.to_string();
        let taken = self
            .existing_rules()?
            .iter()
            .any(|rule| rule.id == id || rule.public_id.as_deref() == Some(id.as_str()));
        ensure!(!taken, "the project already has a rule {id}");
        Ok(())
    }

    fn utils(&self) -> Result<GlobalRules> {
        let mut utils = Vec::new();
        for (path, text) in self.yaml_files(&self.config.util_dirs)? {
            let util: SerializableGlobalRule<SupportLang> = ast_grep_config::from_str(&text)
                .with_context(|| format!("{} does not parse", path.display()))?;
            utils.push(util);
        }
        DeserializeEnv::parse_global_utils(utils).context("the project's utilities do not compile")
    }

    /// Adds the rule's file, its Markdown beside it and, for a new pack, the pack's `xpd.yml`
    /// entry.
    pub(in crate::author) fn add_rule(
        &self,
        doc: &RuleDoc,
        pack: Option<&Pack>,
        changes: &mut Changes,
    ) -> Result<()> {
        let text = doc.checked_yaml(&self.utils()?)?;
        let dirs = pack.map_or(&self.config.rule_dirs, |pack| &pack.rule_dirs);
        let dir = dirs.first().with_context(|| match pack {
            Some(pack) => format!("pack {} in {XPD_CONFIG} has no ruleDirs", pack.module),
            None => format!("{SGCONFIG} has no ruleDirs"),
        })?;
        let path = self.inside(&format!("{dir}/{}.yml", doc.id.name))?;
        changes.stage(path, text, None);
        let markdown = self.inside(&format!("{dir}/{}.md", doc.id.name))?;
        changes.stage(markdown, doc.markdown(), None);
        if let Some(pack) = pack.filter(|pack| !pack.declared) {
            let text = with_pack(self.xpd_config.as_deref(), pack)?;
            changes.stage(self.inside(XPD_CONFIG)?, text, self.xpd_config.as_ref());
        }
        Ok(())
    }

    /// Adds an empty test case for the rule `id`, in `dir` or the first test folder.
    pub(in crate::author) fn add_test(
        &self,
        id: &str,
        dir: Option<&str>,
        changes: &mut Changes,
    ) -> Result<()> {
        let dirs = self.test_dirs();
        let dir = dir
            .or(dirs.first().map(String::as_str))
            .with_context(|| format!("{SGCONFIG} has no testConfigs"))?;
        let path = self.inside(&format!("{dir}/{id}-test.yml"))?;
        changes.stage(path, document::empty_test(id)?, None);
        Ok(())
    }

    /// Adds a global utility, in `dir` or the first utility folder.
    pub(in crate::author) fn add_util(
        &self,
        name: &Name,
        language: SupportLang,
        language_name: &str,
        pattern: &str,
        dir: Option<&str>,
        changes: &mut Changes,
    ) -> Result<()> {
        let dir = dir
            .or(self.config.util_dirs.first().map(String::as_str))
            .with_context(|| format!("{SGCONFIG} has no utilDirs"))?;
        let others: Vec<(String, String)> = self
            .yaml_files(&self.config.util_dirs)?
            .into_iter()
            .map(|(path, text)| (path.display().to_string(), text))
            .collect();
        for (path, text) in &others {
            let id = ast_grep_config::from_str::<Value>(text)
                .ok()
                .and_then(|util| util.get("id").and_then(Value::as_str).map(str::to_owned));
            ensure!(
                id.as_deref() != Some(&name.to_string()),
                "{path} already has the utility {name}"
            );
        }
        let text = document::checked_util(name, language, language_name, pattern, &others)?;
        let path = self.inside(&format!("{dir}/{name}.yml"))?;
        changes.stage(path, text, None);
        Ok(())
    }
}

/// `relative` in the folder `root`. Fails when it leads outside the folder, by `..`, an absolute
/// path or a link.
fn inside(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !relative.contains('\\')
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "{relative:?} leads outside the project folder {}",
        root.display()
    );
    let full = root.join(path);
    if fs::symlink_metadata(root).is_err() {
        return Ok(full);
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("cannot read {}", root.display()))?;
    let existing = full
        .ancestors()
        .find(|ancestor| fs::symlink_metadata(ancestor).is_ok())
        .unwrap_or(&root);
    let real = existing
        .canonicalize()
        .with_context(|| format!("{} is a broken link", existing.display()))?;
    ensure!(
        real.starts_with(&root),
        "{relative:?} leads outside the project folder {} through a link",
        root.display()
    );
    Ok(full)
}

/// The text of `xpd.yml` with the pack added, and every other line of the file kept.
fn with_pack(text: Option<&str>, pack: &Pack) -> Result<String> {
    let entry = pack.entry();
    let Some(text) = text else {
        return Ok(serde_saphyr::to_string(&NewXpdConfig {
            packs: vec![entry],
        })?);
    };
    let item = serde_saphyr::to_string(&[&entry])?;
    let mut expected: Value =
        ast_grep_config::from_str(text).with_context(|| format!("{XPD_CONFIG} does not parse"))?;
    let entry_value = serde_json::to_value(&entry)?;
    match &mut expected {
        Value::Null => expected = json!({ "packs": [entry_value] }),
        Value::Object(config) => config
            .entry("packs")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .with_context(|| format!("packs in {XPD_CONFIG} is not a list"))?
            .push(entry_value),
        _ => bail!("{XPD_CONFIG} is not a mapping"),
    }
    let base = if text.is_empty() || text.ends_with('\n') {
        text.to_owned()
    } else {
        format!("{text}\n")
    };
    let lines: Vec<&str> = base.lines().collect();
    let indent = lines
        .iter()
        .skip_while(|line| !line.starts_with("packs:"))
        .skip(1)
        .find(|line| line.trim_start().starts_with("- "))
        .map(|line| &line[..line.len() - line.trim_start().len()])
        .unwrap_or_default();
    let mut indented = String::new();
    for line in item.lines() {
        indented.push_str(indent);
        indented.push_str(line);
        indented.push('\n');
    }
    let flow_empty = lines.iter().position(|line| line.trim_end() == "packs: []");
    let candidates = [
        Some(format!("{base}{indented}")),
        flow_empty.map(|at| {
            let mut lines: Vec<String> = lines.iter().map(|line| format!("{line}\n")).collect();
            lines[at] = format!("packs:\n{item}");
            lines.concat()
        }),
        Some(format!("{base}packs:\n{item}")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| ast_grep_config::from_str::<Value>(candidate).ok().as_ref() == Some(&expected))
        .with_context(|| {
            format!("cannot add pack {} to {XPD_CONFIG} without rewriting it. Add this entry to its packs:\n{item}", pack.module)
        })
}
