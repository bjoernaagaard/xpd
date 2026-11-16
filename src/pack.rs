//! Rule packs, one per Python library, embedded at compile time by build.rs.

use std::collections::BTreeMap;
use std::fmt;

use anyhow::{Context, Error, Result};
use ast_grep_config::{DeserializeEnv, GlobalRules, RuleConfig, SerializableRule, Severity};
use ast_grep_language::SupportLang;
use serde::Deserialize;

use crate::package_check::PackageCheck;
use crate::rule::Rule;
use crate::scan::Root;
use crate::venv::Distributions;
use crate::version::Version;

/// One pack as build.rs found it under `rules/<module>/`.
#[derive(Debug)]
struct PackSource {
    module: &'static str,
    /// The text of `pack.yml`, when the pack has one.
    manifest: Option<&'static str>,
    rules: &'static [RuleSource],
    /// The `#### <id>` section of docs/rules.md for each rule and package check of the pack.
    /// build.rs fails when one is missing.
    docs: &'static [DocSource],
}

/// One section of docs/rules.md, from its heading to the next heading.
#[derive(Debug)]
struct DocSource {
    id: &'static str,
    text: &'static str,
}

/// One rule file, `rules/<module>/<id>.yml`. build.rs checks that its documents have that id.
#[derive(Debug)]
struct RuleSource {
    id: &'static str,
    yaml: &'static str,
}

/// Every pack under `rules/`, in name order.
const SOURCES: &[PackSource] = include!(concat!(env!("OUT_DIR"), "/packs.rs"));

/// A pack's `pack.yml`: what belongs to the library as a whole and not to one rule.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    #[serde(default)]
    aliases: BTreeMap<String, String>,
    #[serde(default)]
    packages: Vec<PackageCheck>,
    /// The pack is about Python itself, so its rules run on every file.
    #[serde(default)]
    every_file: bool,
    /// A second way for a file to use the library, without importing it.
    also_on: Option<AlsoOnSource>,
}

/// `alsoOn` in a `pack.yml`, as two ast-grep rules.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AlsoOnSource {
    setting: SerializableRule,
    stand_in: SerializableRule,
}

/// A file that uses a pack's library through another library, as pandas draws `df.plot` with
/// plotly in a file that makes plotly its plotting backend.
pub(crate) struct AlsoOn {
    /// What the file must hold for the pack's rules to run on it without an import.
    setting: Shape,
    /// The expression that stands for the library's module in such a file, such as `df.plot`.
    stand_in: Shape,
}

impl TryFrom<AlsoOnSource> for AlsoOn {
    type Error = Error;

    fn try_from(source: AlsoOnSource) -> Result<Self> {
        let env = DeserializeEnv::new(SupportLang::Python);
        Ok(Self {
            setting: env.deserialize_rule(source.setting).context("setting")?,
            stand_in: env.deserialize_rule(source.stand_in).context("standIn")?,
        })
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
type Shape = ast_grep_config::Rule;

/// Which files a pack's rules run on.
pub(crate) enum Gate {
    /// Files that import the pack's module, and files with the pack's `alsoOn` setting. The
    /// rule matches such an import.
    Import {
        import: Box<RuleConfig<SupportLang>>,
        also: Option<AlsoOn>,
    },
    /// Every file.
    EveryFile,
}

/// One library's rules.
pub(crate) struct Pack {
    /// The name the library is imported by, such as `pandas`.
    pub(crate) module: &'static str,
    gate: Gate,
    pub(crate) rules: Vec<Rule>,
    /// Short names a fix may write, each with the module it must be imported as, such as `pd`
    /// for `pandas`.
    pub(crate) aliases: BTreeMap<String, String>,
    /// Packages the library works better with.
    pub(crate) package_checks: Vec<PackageCheck>,
    docs: &'static [DocSource],
}

impl TryFrom<&PackSource> for Pack {
    type Error = Error;

    /// Parses the pack's rule files and its manifest.
    fn try_from(source: &PackSource) -> Result<Self> {
        let globals = GlobalRules::default();
        let mut rules = Vec::new();
        for file in source.rules {
            let context = || format!("{}.yml", file.id);
            for config in
                ast_grep_config::from_yaml_string(file.yaml, &globals).with_context(context)?
            {
                if config.severity != Severity::Off {
                    rules.push(Rule::try_from(config).with_context(context)?);
                }
            }
        }
        let manifest: Manifest = match source.manifest {
            Some(yaml) => ast_grep_config::from_str(yaml).context("pack.yml")?,
            None => Manifest::default(),
        };
        let gate = if manifest.every_file {
            Gate::EveryFile
        } else {
            let import = ast_grep_config::from_yaml_string(&gate_yaml(source.module), &globals)?
                .pop()
                .context("the import rule did not parse")?;
            let also =
                (manifest.also_on.map(AlsoOn::try_from).transpose()).context("pack.yml alsoOn")?;
            Gate::Import {
                import: Box::new(import),
                also,
            }
        };
        Ok(Self {
            module: source.module,
            gate,
            rules,
            aliases: manifest.aliases,
            package_checks: manifest.packages,
            docs: source.docs,
        })
    }
}

impl Pack {
    /// Whether the pack's rules can run on `src`, from its text alone. A file that never names
    /// the module cannot import it, so it needs no parse for this pack.
    pub(crate) fn may_apply(&self, src: &str) -> bool {
        match &self.gate {
            Gate::Import { .. } => src.contains(self.module),
            Gate::EveryFile => true,
        }
    }

    /// Whether the pack's rules run on the parsed file. Rules for a library run only on files
    /// that import it or have the pack's `alsoOn` setting, because they match names such as
    /// `merge` and `inplace` that other libraries share.
    pub(crate) fn applies(&self, root: &Root, src: &str) -> bool {
        match &self.gate {
            Gate::Import { import, .. } => {
                src.contains(self.module)
                    && (root.root().find(&import.matcher).is_some()
                        || self.stand_in(root).is_some())
            }
            Gate::EveryFile => true,
        }
    }

    /// The pack's library as the parsed file uses it.
    pub(crate) fn library(&self, root: &Root) -> Library<'_> {
        Library {
            module: self.module,
            stand_in: self.stand_in(root),
        }
    }

    /// The expression that stands for the library's module in the parsed file, when the file
    /// has the pack's `alsoOn` setting.
    fn stand_in(&self, root: &Root) -> Option<&Shape> {
        match &self.gate {
            Gate::Import {
                also: Some(also), ..
            } => (root.root().find(&also.setting).is_some()).then_some(&also.stand_in),
            _ => None,
        }
    }

    /// The Markdown section of docs/rules.md about the rule or package check `id`.
    pub(crate) fn doc(&self, id: &str) -> Option<&'static str> {
        self.docs
            .iter()
            .find(|doc| doc.id == id)
            .map(|doc| doc.text)
    }

    /// Drops the rules that need a newer or an older version of the library than `installed`.
    fn keep_rules_for(&mut self, installed: &Version) {
        self.rules.retain(|rule| rule.applies_to(installed));
    }

    /// Drops the rules whose advice is only for older versions of the library.
    fn keep_newest_rules(&mut self) {
        self.rules.retain(Rule::applies_to_newest);
    }
}

impl fmt::Debug for Pack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pack")
            .field("module", &self.module)
            .field("rules", &self.rules)
            .field("aliases", &self.aliases)
            .field("package_checks", &self.package_checks)
            .finish_non_exhaustive()
    }
}

/// Every pack, each with the rules for the `installed` version of its library. A pack whose
/// library is not installed, or a run without a venv, keeps every rule but those with `until`,
/// so the project gets the newest advice.
pub(crate) fn load(installed: Option<&Distributions>) -> Result<Vec<Pack>> {
    SOURCES
        .iter()
        .map(|source| {
            let mut pack =
                Pack::try_from(source).with_context(|| format!("rules/{}", source.module))?;
            match installed.and_then(|found| found.version(pack.module)) {
                Some(version) => pack.keep_rules_for(version),
                None => pack.keep_newest_rules(),
            }
            Ok(pack)
        })
        .collect()
}

/// The `installed` version of each pack's library, in the order of the packs. `load` gives the
/// same rules for two venvs with the same answer.
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
        for rule in source.rules {
            for document in rule.yaml.split("\n---") {
                let value: serde_json::Value = ast_grep_config::from_str(document)?;
                let mut tokens = std::collections::BTreeSet::new();
                pattern_names(
                    value.get("rule").unwrap_or(&serde_json::Value::Null),
                    &mut tokens,
                );
                pattern_names(
                    value.get("utils").unwrap_or(&serde_json::Value::Null),
                    &mut tokens,
                );
                for token in tokens {
                    let rules = links.entry(token).or_default();
                    let id = format!("{}/{}", module, rule.id);
                    if !rules.contains(&id) {
                        rules.push(id);
                    }
                }
            }
        }
    }
    Ok(links)
}

fn pattern_names(value: &serde_json::Value, names: &mut std::collections::BTreeSet<String>) {
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
