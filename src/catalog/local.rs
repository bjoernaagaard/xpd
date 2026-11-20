//! The rules a project adds to xpd: an ast-grep project, which is a folder with `sgconfig.yml`,
//! and the optional `xpd.yml` beside it that makes some of its rule folders packs.
//!
//! `sgconfig.yml` is ast-grep's own file, and xpd reads `ruleDirs`, `utilDirs` and
//! `testConfigs` from it as ast-grep does, each a path below the folder of the file. A rule file
//! under `ruleDirs` is a plain ast-grep rule. `xpd.yml` lists packs, each with the Python module
//! it is for and its own `ruleDirs`, and a rule file under one of those is a rule of that pack,
//! with the import gate, the library versions and the value proofs of an embedded pack:
//!
//! ```yaml
//! packs:
//!   - module: team
//!     ruleDirs: [rules/team]
//!     everyFile: true
//!     aliases: { tm: team }
//!     packages: [{ id: missing-extra, package: team-extra, message: Add team-extra. }]
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use ast_grep_config::GlobalRules;
use ast_grep_language::config_file_type;
use serde::Deserialize;

use crate::pack::{self, AlsoOnSource, Manifest, Pack};
use crate::package_check::PackageCheckSource;
use crate::rule::{Origin, Rule};
use crate::scan::cannot_read;

/// The names ast-grep finds its project file by, in the order it tries them.
const CONFIG_NAMES: [&str; 2] = ["sgconfig.yml", "sgconfig.yaml"];
/// The file beside the project file that lists the project's packs.
const MANIFEST_NAME: &str = "xpd.yml";
/// The manifest of an embedded pack, which a local rule folder must not hold.
const EMBEDDED_MANIFEST: &str = "pack.yml";

/// What xpd reads from `sgconfig.yml`. ast-grep allows more keys, and a new one must not stop
/// xpd, so an unknown key is not an error.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SgConfig {
    #[serde(default)]
    rule_dirs: Vec<PathBuf>,
    util_dirs: Option<Vec<PathBuf>>,
    test_configs: Option<Vec<TestConfigSource>>,
    custom_languages: Option<serde_json::Value>,
    language_globs: Option<serde_json::Value>,
    language_injections: Option<serde_json::Value>,
}

impl SgConfig {
    fn tests(&self, root: &Path) -> Result<Vec<TestConfig>> {
        (self.test_configs.iter().flatten())
            .map(|test| {
                let test_dir = absolute(&root.join(&test.test_dir));
                ensure!(
                    test_dir.starts_with(root)
                        && !test
                            .test_dir
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "testDir must stay inside the project root"
                );
                Ok(TestConfig {
                    test_dir,
                    snapshot_dir: test.snapshot_dir.clone(),
                })
            })
            .collect()
    }

    /// The keys that are set and that change which files ast-grep reads as which language,
    /// which xpd does not follow.
    fn unsupported(&self) -> Vec<&'static str> {
        [
            ("customLanguages", &self.custom_languages),
            ("languageGlobs", &self.language_globs),
            ("languageInjections", &self.language_injections),
        ]
        .into_iter()
        .filter(|(_, value)| value.is_some())
        .map(|(key, _)| key)
        .collect()
    }
}

/// One entry of `testConfigs`, as ast-grep writes it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestConfigSource {
    test_dir: PathBuf,
    snapshot_dir: Option<PathBuf>,
}

/// Where the project keeps the test cases of its rules.
#[derive(Debug, Clone)]
pub(crate) struct TestConfig {
    pub(crate) test_dir: PathBuf,
    /// The folder of the snapshots, when the project names one. ast-grep reads the name below
    /// `test_dir`.
    pub(crate) snapshot_dir: Option<PathBuf>,
}

/// `xpd.yml`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct XpdConfig {
    #[serde(default)]
    packs: Vec<PackEntry>,
}

/// One entry under `packs` in `xpd.yml`: the manifest of a pack, with its module and folders.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackEntry {
    module: String,
    rule_dirs: Vec<PathBuf>,
    #[serde(default)]
    aliases: BTreeMap<String, String>,
    #[serde(default)]
    members: BTreeMap<String, String>,
    #[serde(default)]
    packages: Vec<PackageCheckSource>,
    #[serde(default)]
    every_file: bool,
    also_on: Option<AlsoOnSource>,
}

/// A pack of `xpd.yml` as the project places it: its module and its rule folders.
#[derive(Debug)]
struct LocalPack {
    module: String,
    rule_dirs: Vec<PathBuf>,
}

/// One project's own rules, as the files said when the catalog was loaded. Every path is
/// absolute.
pub(crate) struct LocalProject {
    /// The folder of the project file.
    pub(crate) root: PathBuf,
    /// The project file, `sgconfig.yml`.
    pub(crate) config: PathBuf,
    /// `xpd.yml`, when the project has one.
    pub(crate) manifest: Option<PathBuf>,
    /// The folders whose YAML files are rules: those of `sgconfig.yml`, then those of the packs.
    pub(crate) rule_dirs: Vec<PathBuf>,
    /// The folders whose YAML files are utility rules.
    pub(crate) util_dirs: Vec<PathBuf>,
    pub(crate) test_configs: Vec<TestConfig>,
    /// The keys of `sgconfig.yml` that are set and that xpd does not follow.
    unsupported: Vec<&'static str>,
    custom_languages: BTreeSet<String>,
    packs: Vec<LocalPack>,
    /// The project's utility rules, which each of its rules can name with `matches`.
    globals: GlobalRules,
}

impl std::fmt::Debug for LocalProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalProject")
            .field("config", &self.config)
            .field("manifest", &self.manifest)
            .field("rule_dirs", &self.rule_dirs)
            .field("util_dirs", &self.util_dirs)
            .field("test_configs", &self.test_configs)
            .field("packs", &self.packs)
            .finish_non_exhaustive()
    }
}

/// `path` without symbolic links, or as an absolute path when no file is there yet.
fn absolute(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// The folders that `key` of the file `source` names, each below `root`. Each must exist, so
/// that a misspelled name is not a project without rules.
fn folders(root: &Path, dirs: &[PathBuf], key: &str, source: &Path) -> Result<Vec<PathBuf>> {
    dirs.iter()
        .map(|dir| {
            let path = root.join(dir);
            ensure!(
                path.is_dir(),
                "{}: {key} names {}, which is not a folder",
                source.display(),
                dir.display()
            );
            fs::canonicalize(&path).with_context(|| cannot_read(&path))
        })
        .collect()
}

/// Every YAML file under `dirs`, each once, in path order. A folder that is named is read
/// whatever an ignore file above it says, as ast-grep reads it.
fn yaml_files(dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for dir in dirs {
        let walk = ignore::WalkBuilder::new(dir)
            .parents(false)
            .types(config_file_type())
            .build();
        for entry in walk {
            let entry = entry.with_context(|| cannot_read(dir))?;
            if entry.file_type().is_some_and(|kind| kind.is_file()) {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Whether `module` can be the module of a pack. It goes into the pattern of the import gate,
/// so only an identifier is allowed.
pub(crate) fn is_module_name(module: &str) -> bool {
    let mut chars = module.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl XpdConfig {
    /// Reads the text of the `xpd.yml` at `path`.
    fn parse(path: &Path, text: &str) -> Result<Self> {
        let config: Self =
            ast_grep_config::from_str(text).with_context(|| path.display().to_string())?;
        for entry in &config.packs {
            ensure!(
                is_module_name(&entry.module),
                "{}: the module {:?} of a pack must be a Python module name",
                path.display(),
                entry.module
            );
        }
        Ok(config)
    }
}

impl LocalProject {
    /// The project file of the project that `from` is in: the nearest `sgconfig.yml` in `from`
    /// or a folder above it, as ast-grep finds it. `None` when there is none.
    pub(crate) fn find(from: &Path) -> Result<Option<PathBuf>> {
        let from = absolute(from);
        for dir in from.ancestors() {
            let config = CONFIG_NAMES.iter().map(|name| dir.join(name));
            if let Some(config) = config.into_iter().find(|path| path.is_file()) {
                return Ok(Some(config));
            }
            let manifest = dir.join(MANIFEST_NAME);
            ensure!(
                !manifest.is_file(),
                "{} lists packs of an ast-grep project, and the folder has no {}",
                manifest.display(),
                CONFIG_NAMES[0]
            );
        }
        Ok(None)
    }

    /// Reads the project of the project file `config`, with every rule file it has. Returns
    /// the project with its packs: those of `xpd.yml` in the order of the file, then one for
    /// each package of the rules that no pack holds, in name order.
    pub(crate) fn read(config: &Path) -> Result<(Self, Vec<Pack>)> {
        let (project, manifests) = Self::layout(config)?;
        let packs = project.load_rules(manifests)?;
        Ok((project, packs))
    }

    /// Reads the project file `config`, `xpd.yml` and the utility rules: everything but the
    /// rule files. Returns the project with the manifest of each of its packs.
    fn layout(config: &Path) -> Result<(Self, Vec<Manifest>)> {
        let config = fs::canonicalize(config).with_context(|| cannot_read(config))?;
        let root = config
            .parent()
            .context("a file is in a folder")?
            .to_path_buf();
        let text = fs::read_to_string(&config).with_context(|| cannot_read(&config))?;
        let sg: SgConfig =
            ast_grep_config::from_str(&text).with_context(|| config.display().to_string())?;
        let custom_languages = sg
            .custom_languages
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .map(|languages| {
                languages
                    .keys()
                    .filter(|name| name.parse::<ast_grep_language::SupportLang>().is_err())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let manifest = Some(root.join(MANIFEST_NAME)).filter(|path| path.is_file());
        let entries = match &manifest {
            Some(path) => {
                let text = fs::read_to_string(path).with_context(|| cannot_read(path))?;
                XpdConfig::parse(path, &text)?.packs
            }
            None => Vec::new(),
        };

        let mut rule_dirs = folders(&root, &sg.rule_dirs, "ruleDirs", &config)?;
        let util_dirs = folders(
            &root,
            sg.util_dirs.as_deref().unwrap_or_default(),
            "utilDirs",
            &config,
        )?;
        let test_configs = sg.tests(&root)?;
        let mut packs: Vec<LocalPack> = Vec::new();
        let mut manifests = Vec::new();
        for entry in entries {
            let source = manifest.as_deref().unwrap_or(&config);
            let dirs = folders(&root, &entry.rule_dirs, "ruleDirs", source)?;
            for dir in &dirs {
                if let Some(other) = packs.iter().find(|pack| pack.rule_dirs.contains(dir)) {
                    bail!(
                        "{}: the packs {} and {} both name the folder {}",
                        source.display(),
                        other.module,
                        entry.module,
                        dir.display()
                    );
                }
            }
            rule_dirs.extend(dirs.iter().cloned());
            packs.push(LocalPack {
                module: entry.module,
                rule_dirs: dirs,
            });
            manifests.push(Manifest {
                aliases: entry.aliases,
                members: entry.members,
                packages: entry.packages,
                every_file: entry.every_file,
                also_on: entry.also_on,
            });
        }

        let util_files = yaml_files(&util_dirs)?;
        let util_texts = util_files
            .iter()
            .map(|path| {
                let text = fs::read_to_string(path).with_context(|| cannot_read(path))?;
                supported_yaml(&text, &custom_languages)
            })
            .collect::<Result<Vec<String>>>()?;
        let utils = util_files.iter().map(PathBuf::as_path);
        let globals = pack::global_utils(
            utils
                .zip(util_texts.iter().map(String::as_str))
                .filter(|(_, text)| !text.is_empty()),
        )
        .with_context(|| format!("the utility rules of {}", config.display()))?;

        let project = Self {
            root,
            unsupported: sg.unsupported(),
            custom_languages,
            manifest,
            rule_dirs,
            util_dirs,
            test_configs,
            packs,
            globals,
            config,
        };
        Ok((project, manifests))
    }

    fn load_rules(&self, manifests: Vec<Manifest>) -> Result<Vec<Pack>> {
        let mut in_packs: Vec<Vec<Rule>> = self.packs.iter().map(|_| Vec::new()).collect();
        let mut structural: BTreeMap<String, Vec<Rule>> = BTreeMap::new();
        for path in yaml_files(&self.rule_dirs)? {
            if !self.is_rule_file(&path) {
                continue;
            }
            ensure!(
                path.file_name()
                    .is_none_or(|name| name != EMBEDDED_MANIFEST),
                "{}: a rule folder of a project holds no {EMBEDDED_MANIFEST}. Describe the pack under packs in {MANIFEST_NAME}",
                path.display()
            );
            let text = fs::read_to_string(&path).with_context(|| cannot_read(&path))?;
            let owner = self.owner(&path);
            for rule in self.compile(&path, &text)? {
                match owner {
                    Some(pack) => in_packs[pack].push(rule),
                    None => {
                        structural
                            .entry(rule.id.package().to_owned())
                            .or_default()
                            .push(rule);
                    }
                }
            }
        }

        let mut loaded = Vec::new();
        for ((pack, manifest), rules) in self.packs.iter().zip(manifests).zip(in_packs) {
            let root = Some(self.root.clone());
            let pack = Pack::library(&pack.module, manifest, rules, &self.globals, root)
                .with_context(|| format!("the pack {} of {MANIFEST_NAME}", pack.module))?;
            loaded.push(pack);
        }
        loaded.extend(
            structural
                .into_iter()
                .map(|(package, rules)| Pack::structural(package, rules, self.root.clone())),
        );
        Ok(loaded)
    }

    /// Whether the YAML file at `path` under a rule folder is a rule file. The folders of the
    /// utility rules and of the test cases can be inside a rule folder, and so can the
    /// project's own two files.
    fn is_rule_file(&self, path: &Path) -> bool {
        let tests = self.test_configs.iter().flat_map(|test| {
            [Some(&test.test_dir), test.snapshot_dir.as_ref()]
                .into_iter()
                .flatten()
        });
        let mut others = self.util_dirs.iter().chain(tests);
        self.rule_dirs.iter().any(|dir| path.starts_with(dir))
            && !others.any(|dir| path.starts_with(dir))
            && path != self.config
            && Some(path) != self.manifest.as_deref()
    }

    /// The pack of `xpd.yml` that holds the rule file at `path`, by its place in the file. The
    /// deepest folder wins, so a pack can have a folder inside a folder of plain rules.
    fn owner(&self, path: &Path) -> Option<usize> {
        let dirs = self.packs.iter().enumerate().flat_map(|(index, pack)| {
            let dirs = pack.rule_dirs.iter().filter(|dir| path.starts_with(dir));
            dirs.map(move |dir| (dir.components().count(), index))
        });
        dirs.max().map(|(_, index)| index)
    }

    /// The rules of the rule file at `path` with the text `yaml`, compiled as the project
    /// loads them.
    fn compile(&self, path: &Path, yaml: &str) -> Result<Vec<Rule>> {
        let yaml = supported_yaml(yaml, &self.custom_languages)?;
        let pack = self
            .owner(path)
            .map(|index| self.packs[index].module.as_str());
        let origin = match pack {
            Some(_) => Origin::LocalPack,
            None => Origin::Structural,
        };
        pack::compile_rules(path, &yaml, &self.globals, origin, pack)
            .with_context(|| path.display().to_string())
    }

    /// Checks `text` as the content of the project's file at `path`, which need not be saved:
    /// the project file, `xpd.yml`, a utility rule or a rule file. Returns the rules of a rule
    /// file. A utility rule is checked together with the saved ones, which it may name.
    pub(crate) fn check_document(&self, path: &Path, text: &str) -> Result<Vec<Rule>> {
        let path = absolute(path);
        if path == self.config {
            ast_grep_config::from_str::<SgConfig>(text)
                .with_context(|| path.display().to_string())?;
        } else if Some(&path) == self.manifest.as_ref() {
            XpdConfig::parse(&path, text)?;
        } else if self.util_dirs.iter().any(|dir| path.starts_with(dir)) {
            let mut sources = vec![(path.clone(), supported_yaml(text, &self.custom_languages)?)];
            for saved in yaml_files(&self.util_dirs)? {
                if saved != path {
                    let text = fs::read_to_string(&saved).with_context(|| cannot_read(&saved))?;
                    sources.push((saved, supported_yaml(&text, &self.custom_languages)?));
                }
            }
            pack::global_utils(
                sources
                    .iter()
                    .filter(|(_, text)| !text.is_empty())
                    .map(|(path, text)| (path.as_path(), text.as_str())),
            )
            .with_context(|| path.display().to_string())?;
        } else if self.is_rule_file(&path) {
            return self.compile(&path, text);
        }
        Ok(Vec::new())
    }

    /// One sentence for each key of `sgconfig.yml` that xpd does not follow, for the user.
    pub(crate) fn warnings(&self) -> Vec<String> {
        self.unsupported
            .iter()
            .map(|key| {
                if *key == "customLanguages" {
                    return format!("{}: xpd skips rules and utilities for custom languages", self.config.display());
                }
                format!(
                    "{}: xpd does not read {key}, so it finds each file's language by the file's name alone",
                    self.config.display()
                )
            })
            .collect()
    }
}

fn supported_yaml(text: &str, custom: &BTreeSet<String>) -> Result<String> {
    if custom.is_empty() {
        return Ok(text.to_owned());
    }
    let documents: Vec<serde_json::Value> = ast_grep_config::from_multiple(text)?;
    let mut supported = String::new();
    for document in documents {
        if document
            .get("language")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|language| custom.contains(language))
        {
            continue;
        }
        supported.push_str("---\n");
        supported.push_str(&serde_saphyr::to_string(&document)?);
    }
    Ok(supported)
}
