//! Rule packs: the ones for Python libraries that build.rs embeds at compile time, and the
//! ones a project adds with its own rule files.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Error, Result, bail, ensure};
use ast_grep_config::{
    CombinedScan, DeserializeEnv, GlobalRules, RuleConfig, SerializableGlobalRule,
    SerializableRule, Severity,
};
use ast_grep_language::SupportLang;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use crate::pack_names::OWN_PACK;
use crate::package_check::{PackageCheck, PackageCheckSource};
use crate::rule::{Origin, Rule};
use crate::rule_id::RuleId;
use crate::source::File;
use crate::style::Style;
use crate::syntax::Root;
use crate::venv::Distributions;
use crate::version::Version;

/// One pack as build.rs found it under `rules/<module>/`.
#[derive(Debug)]
struct PackSource {
    module: &'static str,
    /// The text of `pack.yml`, when the pack has one.
    manifest: Option<&'static str>,
    /// Each `rules/<module>/<id>.yml`. build.rs checks that its documents have that id.
    rules: &'static [RuleSource],
    /// Each `rules/<module>/utils/<id>.yml`, a utility rule that every rule of the pack can
    /// name with `matches`.
    utils: &'static [RuleSource],
    /// The Markdown of `rules/<module>/<id>.md` for each rule and package check of the pack.
    /// build.rs fails when one is missing.
    docs: &'static [DocSource],
}

/// The Markdown that explains one rule or package check.
#[derive(Debug)]
struct DocSource {
    id: &'static str,
    text: &'static str,
}

/// One YAML file of a pack, with its path below the repository.
#[derive(Debug)]
struct RuleSource {
    path: &'static str,
    yaml: &'static str,
}

impl RuleSource {
    /// The file's name without `.yml`, which is the id of its rule.
    fn id(&self) -> &'static str {
        let name = self.path.rsplit('/').next().unwrap_or(self.path);
        name.strip_suffix(".yml").unwrap_or(name)
    }
}

/// Every pack under `rules/`, in name order.
const SOURCES: &[PackSource] = include!(concat!(env!("OUT_DIR"), "/packs.rs"));

/// The Markdown of xpd's own rules, from `rules/xpd/<id>.md`.
const OWN_DOCS: &[DocSource] = include!(concat!(env!("OUT_DIR"), "/own_docs.rs"));

/// What belongs to a library as a whole and not to one rule: a `pack.yml` of an embedded pack,
/// or one entry under `packs` in a project's `xpd.yml`.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Manifest {
    #[serde(default)]
    pub(crate) aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) members: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) packages: Vec<PackageCheckSource>,
    /// The pack is not about one library, so its rules run on every file.
    #[serde(default)]
    pub(crate) every_file: bool,
    /// A second way for a file to use the library, without importing it.
    pub(crate) also_on: Option<AlsoOnSource>,
}

/// `alsoOn` in a manifest, as two ast-grep rules.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AlsoOnSource {
    setting: SerializableRule,
    stand_in: SerializableRule,
}

/// A file that uses a pack's library through another library, as pandas draws `df.plot` with
/// plotly in a file that makes plotly its plotting backend.
pub(crate) struct AlsoOn {
    /// What the file must hold for the pack's rules to run on it without an import.
    setting: ManifestRule,
    /// The expression that stands for the library's module in such a file, such as `df.plot`.
    stand_in: ManifestRule,
}

impl AlsoOn {
    /// Both shapes can name the pack's utility rules, which are in `globals`.
    fn new(source: AlsoOnSource, globals: &GlobalRules) -> Result<Self> {
        Ok(Self {
            setting: ManifestRule::read(source.setting, globals).context("setting")?,
            stand_in: ManifestRule::read(source.stand_in, globals).context("standIn")?,
        })
    }
}

/// An ast-grep rule that a manifest writes, which can name the pack's utility rules with
/// `matches`.
pub(crate) struct ManifestRule {
    pub(crate) shape: Shape,
    /// What the shape was read in. A shape that names a utility rule finds it through this, so
    /// it must live as long as the shape does.
    _env: DeserializeEnv<SupportLang>,
}

impl ManifestRule {
    /// Reads `rule` against the pack's utility rules in `globals`.
    pub(crate) fn read(rule: SerializableRule, globals: &GlobalRules) -> Result<Self> {
        let env = DeserializeEnv::new(SupportLang::Python).with_globals(globals);
        let shape = env.deserialize_rule(rule)?;
        shape.verify_util()?;
        Ok(Self { shape, _env: env })
    }
}

impl fmt::Debug for ManifestRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManifestRule").finish_non_exhaustive()
    }
}

/// A pack's library as one file uses it, which a value proof starts from.
#[derive(Clone, Copy)]
pub(crate) struct Library<'p> {
    /// The name the library is imported by.
    pub(crate) module: &'p str,
    /// The expression that stands for the library's module in this file. `None` in a file
    /// without the pack's `alsoOn` setting.
    pub(crate) stand_in: Option<&'p Shape>,
}

/// A shape of code that a pack's manifest names, as an ast-grep rule.
pub(crate) type Shape = ast_grep_config::Rule;

/// Which files a pack's rules run on. A rule runs only on a file of the rule's own language.
pub(crate) enum Gate {
    /// Python files that import the pack's module, and Python files with the pack's `alsoOn`
    /// setting. The rule matches such an import.
    Import {
        import: Box<RuleConfig<SupportLang>>,
        also: Option<Box<AlsoOn>>,
    },
    /// Every file.
    EveryFile,
    /// The rule of xpd itself that reports a suppression comment which silenced nothing. It
    /// is no rule to match: ast-grep's scanner reports it for a comment no selected rule used.
    Suppressions,
}

/// One package's rules: a library's, a project's own, or xpd's.
pub(crate) struct Pack {
    pub(crate) style: Style,
    /// The name issues are reported under. For a library's pack it is the name the library is
    /// imported by, such as `pandas`. Two packs can share it, as a project's own pandas rules
    /// do with the embedded ones.
    pub(crate) module: String,
    gate: Gate,
    pub(crate) rules: Vec<Rule>,
    /// Versioned-out rules still consume suppression comments, but never report or fix.
    pub(crate) inactive_rules: Vec<Rule>,
    /// Short names a fix may write, each with the module it must be imported as, such as `pd`
    /// for `pandas`.
    pub(crate) aliases: BTreeMap<String, String>,
    /// Names a fix may write bare, each with the module it must be imported from, such as
    /// `assert_allclose` from `numpy.testing`.
    pub(crate) members: BTreeMap<String, String>,
    /// Packages the library works better with.
    pub(crate) package_checks: Vec<PackageCheck>,
    docs: &'static [DocSource],
    /// The project folder of a local pack. The `files` and `ignores` globs of its rules match
    /// a path below this folder, as they do in ast-grep. An embedded pack's globs match the
    /// path as it was given.
    root: Option<PathBuf>,
}

/// The rules of the YAML file at `path` with the text `yaml`, compiled against the utility
/// rules in `globals`. `pack` is the module of the pack that holds the file. A document with
/// `severity: off` is no rule. A document without an id takes the file's name, as in ast-grep.
pub(crate) fn compile_rules(
    path: &Path,
    yaml: &str,
    globals: &GlobalRules,
    origin: Origin,
    pack: Option<&str>,
) -> Result<Vec<Rule>> {
    let configs = ast_grep_config::from_yaml_string::<SupportLang>(yaml, globals)?;
    let count = configs.len();
    let mut rules = Vec::new();
    for (index, mut config) in configs.into_iter().enumerate() {
        config.id = RuleId::document_name(&config.id, path, index, count)?;
        if config.severity != Severity::Off {
            let id = config.id.clone();
            let rule = Rule::new(config, origin, pack, path.to_path_buf(), index + 1);
            rules.push(rule.with_context(|| format!("the rule {id}"))?);
        }
    }
    Ok(rules)
}

/// The utility rules of the YAML files in `sources`, each a path with its text and one utility
/// rule, which the rules compiled against the result can name with `matches`.
pub(crate) fn global_utils<'a>(
    sources: impl IntoIterator<Item = (&'a Path, &'a str)>,
) -> Result<GlobalRules> {
    let mut utils: Vec<SerializableGlobalRule<SupportLang>> = Vec::new();
    let mut seen: HashMap<String, &Path> = HashMap::new();
    for (path, yaml) in sources {
        let util: SerializableGlobalRule<SupportLang> =
            ast_grep_config::from_str(yaml).with_context(|| path.display().to_string())?;
        if let Some(first) = seen.insert(util.id.clone(), path) {
            bail!(
                "the utility rule {} is in {} and in {}",
                util.id,
                first.display(),
                path.display()
            );
        }
        utils.push(util);
    }
    Ok(DeserializeEnv::parse_global_utils(utils)?)
}

impl TryFrom<&PackSource> for Pack {
    type Error = Error;

    /// Parses the pack's utility rules, its rule files and its manifest.
    fn try_from(source: &PackSource) -> Result<Self> {
        let utils = source.utils.iter();
        let globals = global_utils(utils.map(|file| (Path::new(file.path), file.yaml)))?;
        let per_file: Vec<Vec<Rule>> = source
            .rules
            .par_iter()
            .map(|file| {
                let path = Path::new(file.path);
                let pack = Some(source.module);
                compile_rules(path, file.yaml, &globals, Origin::Embedded, pack)
                    .with_context(|| format!("{}.yml", file.id()))
            })
            .collect::<Result<_>>()?;
        let rules = per_file.into_iter().flatten().collect();
        let manifest: Manifest = match source.manifest {
            Some(yaml) => ast_grep_config::from_str(yaml).context("pack.yml")?,
            None => Manifest::default(),
        };
        let mut pack =
            Self::library(source.module, manifest, rules, &globals, None).context("pack.yml")?;
        pack.docs = source.docs;
        Ok(pack)
    }
}

impl Pack {
    /// The pack of the module `module` with `rules`, as `manifest` describes it. `root` is the
    /// project folder of a local pack.
    pub(crate) fn library(
        module: &str,
        manifest: Manifest,
        rules: Vec<Rule>,
        globals: &GlobalRules,
        root: Option<PathBuf>,
    ) -> Result<Self> {
        for rule in &rules {
            for name in &rule.fix_imports {
                ensure!(
                    manifest.aliases.contains_key(name) ^ manifest.members.contains_key(name),
                    "{}: {} must name one alias or member in pack.yml",
                    rule.id,
                    name
                );
            }
        }
        let gate = if manifest.every_file {
            Gate::EveryFile
        } else {
            // Only a Python file can import the module, so another rule would never run.
            if let Some(rule) =
                (rules.iter()).find(|rule| rule.config.language != SupportLang::Python)
            {
                bail!(
                    "{}: the rule {} is for {} files, and the pack {module} runs on Python files that import {module}. Set everyFile: true for the pack, or move the rule out of it",
                    rule.source().display(),
                    rule.id.name(),
                    rule.config.language
                );
            }
            let import = ast_grep_config::from_yaml_string(&gate_yaml(module), globals)?
                .pop()
                .context("the import rule did not parse")?;
            let also = (manifest.also_on)
                .map(|source| AlsoOn::new(source, globals).map(Box::new))
                .transpose()
                .context("alsoOn")?;
            Gate::Import {
                import: Box::new(import),
                also,
            }
        };
        let package_checks = (manifest.packages.into_iter())
            .map(|source| PackageCheck::new(source, globals))
            .collect::<Result<_>>()
            .context("packages")?;
        Ok(Self {
            style: Style::Default,
            module: module.to_owned(),
            gate,
            rules,
            inactive_rules: Vec::new(),
            aliases: manifest.aliases,
            members: manifest.members,
            package_checks,
            docs: &[],
            root,
        })
    }

    /// The rule files of the project in `root` that no pack holds and that report under
    /// `package`. They are plain ast-grep rules, so they run on every file of their language.
    pub(crate) fn structural(package: String, rules: Vec<Rule>, root: PathBuf) -> Self {
        Self {
            style: Style::Default,
            module: package,
            gate: Gate::EveryFile,
            rules,
            inactive_rules: Vec::new(),
            aliases: BTreeMap::new(),
            members: BTreeMap::new(),
            package_checks: Vec::new(),
            docs: &[],
            root: Some(root),
        }
    }

    /// The pack of `xpd/unused-suppression`, which reports an `ast-grep-ignore` comment that
    /// silenced nothing.
    ///
    /// ast-grep's scanner decides this from the shapes of the rules alone. A comment counts as
    /// used when a rule's shape matches under it, also when one of xpd's own checks, such as
    /// `holds`, would have dropped that match. So xpd may stay silent about a comment that
    /// hides nothing, and it never reports one that hides an issue.
    pub(crate) fn suppressions() -> Result<Self> {
        let config = CombinedScan::unused_config(Severity::Hint, SupportLang::Python);
        let source = PathBuf::from(OWN_PACK);
        let rule = Rule::new(config, Origin::Structural, Some(OWN_PACK), source, 1)?;
        Ok(Self {
            style: Style::Default,
            module: OWN_PACK.to_owned(),
            gate: Gate::Suppressions,
            rules: vec![rule],
            inactive_rules: Vec::new(),
            aliases: BTreeMap::new(),
            members: BTreeMap::new(),
            package_checks: Vec::new(),
            docs: OWN_DOCS,
            root: None,
        })
    }

    /// The rule that reports unused suppression comments, when this is the pack that has it.
    pub(crate) fn unused_suppression(&self) -> Option<&Rule> {
        match self.gate {
            Gate::Suppressions => self.rules.first(),
            Gate::Import { .. } | Gate::EveryFile => None,
        }
    }

    /// Whether the pack has a rule for files of `language`.
    fn has_rules_for(&self, language: SupportLang) -> bool {
        self.rules
            .iter()
            .chain(&self.inactive_rules)
            .any(|rule| rule.config.language == language)
    }

    /// Whether the pack's rules can run on `src` before parsing. An `alsoOn` rule can match
    /// independently of the pack's module name.
    pub(crate) fn may_apply(&self, src: &str, language: SupportLang) -> bool {
        match &self.gate {
            Gate::Import { also, .. } => {
                language == SupportLang::Python
                    && (also.is_some() || src.contains(self.module.as_str()))
            }
            Gate::EveryFile => self.has_rules_for(language),
            Gate::Suppressions => false,
        }
    }

    /// Whether the pack's rules run on the parsed file of `language`. Rules for a library run
    /// only on files that import it or have the pack's `alsoOn` setting, because they match
    /// names such as `merge` and `inplace` that other libraries share.
    pub(crate) fn applies(&self, root: &Root, src: &str, language: SupportLang) -> bool {
        match &self.gate {
            Gate::Import { import, .. } => {
                language == SupportLang::Python
                    && ((src.contains(self.module.as_str())
                        && root.root().find(&import.matcher).is_some())
                        || self.stand_in(root).is_some())
            }
            Gate::EveryFile => true,
            Gate::Suppressions => false,
        }
    }

    /// The pack's library as the parsed file uses it.
    pub(crate) fn library_in(&self, root: &Root) -> Library<'_> {
        Library {
            module: &self.module,
            stand_in: self.stand_in(root),
        }
    }

    /// The expression that stands for the library's module in the parsed file, when the file
    /// has the pack's `alsoOn` setting.
    fn stand_in(&self, root: &Root) -> Option<&Shape> {
        match &self.gate {
            Gate::Import {
                also: Some(also), ..
            } => (root.root().find(&also.setting.shape).is_some()).then_some(&also.stand_in.shape),
            _ => None,
        }
    }

    /// The path of `file` that the `files` and `ignores` globs of the pack's rules match.
    pub(crate) fn glob_path<'a>(&self, file: File<'a>) -> Cow<'a, Path> {
        let Some(root) = &self.root else {
            return Cow::Borrowed(file.path);
        };
        let given = file.disk.unwrap_or(file.path);
        let full = fs::canonicalize(given).or_else(|_| std::path::absolute(given));
        match full.as_deref().map(|full| full.strip_prefix(root)) {
            Ok(Ok(below)) => Cow::Owned(below.to_path_buf()),
            _ => Cow::Borrowed(file.path),
        }
    }

    /// The Markdown that explains the rule or package check `id`.
    pub(crate) fn doc(&self, id: &str) -> Option<&'static str> {
        self.docs
            .iter()
            .find(|doc| doc.id == id)
            .map(|doc| doc.text)
    }

    /// The published page of the rule or package check `id`, for a pack that ships with xpd and
    /// so has its Markdown in the binary. A project's own pack has no page.
    pub(crate) fn page(&self, id: &str) -> Option<String> {
        (!self.docs.is_empty()).then(|| crate::docs::rule_url(&self.module, id))
    }

    /// Keeps the rules for the `installed` version of the pack's library. A pack whose library
    /// is not installed, or a run without a venv, keeps every rule but those with `until`, so
    /// the project gets the newest advice.
    pub(crate) fn keep_rules_for(&mut self, installed: Option<&Distributions>) {
        let version = installed.and_then(|found| found.version(&self.module));
        (self.rules, self.inactive_rules) = std::mem::take(&mut self.rules)
            .into_iter()
            .chain(std::mem::take(&mut self.inactive_rules))
            .partition(|rule| {
                version.map_or_else(
                    || rule.applies_to_newest(),
                    |version| rule.applies_to(version),
                )
            });
    }
}

impl fmt::Debug for Pack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pack")
            .field("module", &self.module)
            .field("rules", &self.rules)
            .field("aliases", &self.aliases)
            .field("members", &self.members)
            .field("package_checks", &self.package_checks)
            .finish_non_exhaustive()
    }
}

/// Every pack that ships with xpd, each with all of its rules.
pub(crate) fn embedded() -> Result<Vec<Pack>> {
    SOURCES
        .par_iter()
        .map(|source| Pack::try_from(source).with_context(|| format!("rules/{}", source.module)))
        .collect()
}

/// The `installed` version of each embedded pack's library, in the order of the packs. Two
/// venvs with the same answer get the same embedded rules.
pub(crate) fn installed_versions(installed: Option<&Distributions>) -> Vec<Option<Version>> {
    SOURCES
        .iter()
        .map(|source| installed.and_then(|found| found.version(source.module).cloned()))
        .collect()
}

/// An ast-grep rule matching `import <module>`,`import <module>.sub as x` and
/// `from <module> import x`. It checks only the imported module, so `from other import <module>`
/// and `import os as <module>` don't match.
fn gate_yaml(module: &str) -> String {
    format!(
        "id: {module}-import
language: python
rule:
  any:
    - kind: import_statement
      has:
        any:
          - kind: dotted_name
            regex: ^{module}(\\.|$)
          - kind: aliased_import
            has:
              field: name
              regex: ^{module}(\\.|$)
    - kind: import_from_statement
      has:
        field: module_name
        regex: ^{module}(\\.|$)
"
    )
}

pub(crate) fn symbol_rules(module: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let mut links = BTreeMap::<String, Vec<String>>::new();
    if let Some(source) = SOURCES.iter().find(|s| s.module == module) {
        let mut shared = BTreeMap::new();
        for util in source.utils {
            let value: serde_json::Value = ast_grep_config::from_str(util.yaml)?;
            shared.insert(util.id(), value);
        }
        for rule in source.rules {
            for value in ast_grep_config::from_multiple::<serde_json::Value>(rule.yaml)? {
                let null = serde_json::Value::Null;
                let own = value.get("utils").unwrap_or(&null);
                let mut tokens = BTreeSet::new();
                pattern_names(value.get("rule").unwrap_or(&null), &mut tokens);
                pattern_names(own, &mut tokens);
                // The pack's utility rules that the document names count as its own, as they
                // did when each document held a copy of them.
                let mut named = BTreeSet::new();
                let mut open = vec![&value];
                while let Some(part) = open.pop() {
                    let mut found = BTreeSet::new();
                    utility_names(part, &mut found);
                    for name in found {
                        if own.get(&name).is_none()
                            && let Some(util) = shared.get(name.as_str())
                            && named.insert(name)
                        {
                            pattern_names(util.get("rule").unwrap_or(&null), &mut tokens);
                            open.push(util);
                        }
                    }
                }
                for token in tokens {
                    let rules = links.entry(token).or_default();
                    let id = format!("{}/{}", module, rule.id());
                    if !rules.contains(&id) {
                        rules.push(id);
                    }
                }
            }
        }
    }
    Ok(links)
}

/// The utility rules that `value` names with `matches: <id>`, at any depth.
fn utility_names(value: &serde_json::Value, names: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if key == "matches" {
                    if let Some(name) = value.as_str() {
                        names.insert(name.to_owned());
                    }
                    if let Some(calls) = value.as_object() {
                        names.extend(calls.keys().cloned());
                    }
                }
                utility_names(value, names);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                utility_names(value, names);
            }
        }
        _ => {}
    }
}

fn pattern_names(value: &serde_json::Value, names: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "pattern" | "regex") {
                    if let Some(text) = value.as_str() {
                        names.extend(
                            text.split(|c: char| !c.is_alphanumeric() && c != '_')
                                .filter(|s| !s.is_empty())
                                .map(str::to_owned),
                        );
                    }
                } else if key != "not" {
                    pattern_names(value, names);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                pattern_names(value, names);
            }
        }
        _ => {}
    }
}
