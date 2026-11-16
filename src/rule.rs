//! One lint rule: an ast-grep rule, plus the settings xpd reads from its `metadata`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use ast_grep_config::{RuleConfig, SerializableRule};
use ast_grep_core::NodeMatch;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::imports;
use crate::names;
use crate::pack::Library;
use crate::scan::Root;
use crate::version::Version;

/// A shape of code that a rule's metadata names, as an ast-grep rule that can use the `utils`
/// of its document.
type Shape = ast_grep_config::Rule;

/// Whether a fix keeps the code's behavior. `xpd check --fix` applies safe fixes, and
/// `--unsafe-fixes` adds the unsafe ones. The order matters: a fix applies when its
/// applicability is at or below the one asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Applicability {
    Safe,
    Unsafe,
}

impl Applicability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Unsafe => "unsafe",
        }
    }
}

/// Where a rule document comes from, which decides how its `metadata` is read and whether
/// xpd's Python checks run on its matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A pack that ships with xpd. A rule with a fix must say how safe the fix is.
    Embedded,
    /// A pack that the project's `xpd.yml` names. A fix that does not say how safe it is
    /// counts as unsafe.
    LocalPack,
    /// A rule file of the project that no pack holds. It is a plain ast-grep rule: xpd reports
    /// each match and runs none of its Python checks on it. A fix that does not say how safe
    /// it is counts as unsafe, so `--fix` alone never applies it.
    Structural,
}

/// The name of one rule document, which no other document of the catalog has. Several
/// documents can share one public name, one for each form or library version the rule
/// matches, and this tells them apart: `rules/pandas/chained-assignment.yml#2` is the second
/// document of that file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct VariantId(String);

impl fmt::Display for VariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The package of the rules that no pack holds and whose name has no package.
pub(crate) const LOCAL_PACKAGE: &str = "local";

/// What a rule's `metadata` may hold, such as `metadata: { fix: unsafe, since: "3.1" }`.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    fix: Option<Applicability>,
    /// The name the rule reports under, when the document's own `id` must differ from it.
    /// ast-grep's own tools refuse two rule files with one id, so the documents of one rule
    /// that are in several files each take their own `id` and share this name.
    public_id: Option<String>,
    since: Option<Version>,
    until: Option<Version>,
    /// `holds: { FIG: <rule> }` keeps a match only when the variable that `$FIG` matched holds
    /// a value of the rule's shape.
    #[serde(default)]
    holds: BTreeMap<String, SerializableRule>,
    /// Drops a match when the same proof as `holds` succeeds, for fallback advice.
    #[serde(default)]
    unless_holds: BTreeMap<String, SerializableRule>,
    /// `unlessImported: <rule>` drops the matches in a file that imports a module of its
    /// project with a direct module statement or expression of the rule's shape.
    unless_imported: Option<SerializableRule>,
}

/// The settings that need a pack, with how each is written in `metadata`.
const PACK_SETTINGS: [&str; 5] = ["since", "until", "holds", "unlessHolds", "unlessImported"];

impl Metadata {
    /// Reads the `metadata` of a rule document from `origin`. A structural rule may hold any
    /// metadata, as ast-grep allows, and xpd reads `fix` and `publicId` from it.
    fn read(config: &RuleConfig<SupportLang>, origin: Origin) -> Result<Self> {
        let Some(metadata) = &config.metadata else {
            return Ok(Self::default());
        };
        let mut value = serde_json::to_value(metadata)?;
        if origin == Origin::Structural {
            let fields = value.as_object_mut().context("metadata is a mapping")?;
            if let Some(setting) = PACK_SETTINGS.iter().find(|key| fields.contains_key(**key)) {
                bail!(
                    "metadata.{setting} works only in a rule of a pack. Name the rule's folder under ruleDirs of a pack in xpd.yml"
                );
            }
            fields.retain(|key, _| matches!(key.as_str(), "fix" | "publicId"));
        }
        serde_json::from_value(value).context(
            "metadata may hold fix: safe or unsafe, publicId: <name>, since: \"<version>\", until: \"<version>\", holds: { <VARIABLE>: <rule> }, unlessHolds: { <VARIABLE>: <rule> } and unlessImported: <rule>",
        )
    }
}

/// One rule document of a pack.
pub(crate) struct Rule {
    /// The rule's name in its package, such as `chained-assignment`, which a report shows.
    pub(crate) id: String,
    /// The document's own name.
    pub(crate) variant: VariantId,
    /// `config.id` is `<package>/<id>`, and not the `id` the document has on disk. ast-grep
    /// reads a suppression comment against it, so `# ast-grep-ignore: pandas/chained-assignment`
    /// silences every document of the rule.
    pub(crate) config: RuleConfig<SupportLang>,
    /// The rule file the document is in.
    pub(crate) source: PathBuf,
    /// `None` when the rule has no fix.
    pub(crate) fix: Option<Applicability>,
    /// The first library version the rule applies to.
    pub(crate) since: Option<Version>,
    /// The first library version the rule no longer applies to. A rule with `until` gives the
    /// advice for the older releases, and another document gives the advice from `until` on.
    pub(crate) until: Option<Version>,
    /// The rule's `files:` globs. When set, the rule runs only on a path that matches one.
    files: Option<GlobSet>,
    /// The rule's `ignores:` globs. The rule skips a path that matches one.
    ignores: Option<GlobSet>,
    /// Whether xpd's Python checks run on the rule's matches: the proof that an import alias
    /// still names its module, `holds`, `unlessHolds` and `unlessImported`. They read Python
    /// syntax, so they run only for a Python rule of a pack.
    python_checks: bool,
    /// From `metadata.holds`: each variable of the rule that must hold a value of a shape,
    /// which `names::holds` proves from the file. A match that does not bind the variable is
    /// kept, so a rule can match `px.bar(...).update_xaxes(...)` and `fig.update_xaxes(...)`
    /// in one `any`.
    holds: Vec<(String, Shape)>,
    /// The complement of `holds`, so fallback advice survives an unknown binding.
    unless_holds: Vec<(String, Shape)>,
    /// From `metadata.unlessImported`: a shape that silences the rule on a file when a module
    /// the file loads has it at module level, outside any branch or definition body.
    unless_imported: Option<Shape>,
}

/// One entry of a rule's `files:` or `ignores:`, as ast-grep writes it.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FileGlob {
    Pattern(String),
    #[serde(rename_all = "camelCase")]
    Config {
        glob: String,
        #[serde(default)]
        case_insensitive: bool,
    },
}

/// The globs of a rule's `files:` or `ignores:`, matched as ast-grep matches them. ast-grep
/// keeps the type of `globs` private, so this reads its serialized form.
fn glob_set(globs: &impl serde::Serialize) -> Result<GlobSet> {
    let globs: Vec<FileGlob> = serde_json::to_value(globs).and_then(serde_json::from_value)?;
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        builder.add(match glob {
            FileGlob::Pattern(pattern) => Glob::new(&pattern)?,
            FileGlob::Config {
                glob,
                case_insensitive,
            } => GlobBuilder::new(&glob)
                .case_insensitive(case_insensitive)
                .build()?,
        });
    }
    Ok(builder.build()?)
}

impl Rule {
    /// Document number `number`, from 1, of the rule file `source`. `pack` is the module of
    /// the pack that holds the file. Without one the rule is structural, and its package is
    /// the part of its name before a slash, or else `local`.
    ///
    /// A fix applies without asking, so an embedded rule with `fix:` must say how safe the fix
    /// is with `metadata.fix` and may have only one fix. A local rule's fix that does not say
    /// so is unsafe, and the first of several fixes is the one that applies.
    pub(crate) fn new(
        mut config: RuleConfig<SupportLang>,
        origin: Origin,
        pack: Option<&str>,
        source: PathBuf,
        number: usize,
    ) -> Result<Self> {
        let metadata = Metadata::read(&config, origin)?;
        let fix = match (config.fixer.len(), metadata.fix, origin) {
            (0, None, _) => None,
            (0, Some(_), _) => bail!("metadata.fix is set, but the rule has no fix"),
            (1, Some(applicability), _) => Some(applicability),
            (1, None, Origin::Embedded) => {
                bail!("a rule with a fix needs metadata.fix: safe or unsafe")
            }
            (fixes, _, Origin::Embedded) => {
                bail!("a rule may have only one fix, and this one has {fixes}")
            }
            (_, applicability, Origin::LocalPack | Origin::Structural) => {
                Some(applicability.unwrap_or(Applicability::Unsafe))
            }
        };
        let python_checks = origin != Origin::Structural && config.language == SupportLang::Python;
        ensure!(
            python_checks
                || (metadata.holds.is_empty()
                    && metadata.unless_holds.is_empty()
                    && metadata.unless_imported.is_none()),
            "metadata.holds, unlessHolds and unlessImported read Python code, and the rule's language is {}",
            config.language
        );
        let files = config.files.as_ref().map(glob_set).transpose()?;
        let ignores = config.ignores.as_ref().map(glob_set).transpose()?;
        // The document's own environment, so that a shape can say `matches: <one of its utils>`.
        let env = config.matcher.get_env(config.language);
        let shape = |rule: SerializableRule| -> Result<Shape> {
            let shape = env.deserialize_rule(rule)?;
            shape.verify_util()?;
            Ok(shape)
        };
        let held_shapes = |conditions: BTreeMap<String, SerializableRule>, key: &str| {
            conditions
                .into_iter()
                .map(|(variable, rule)| {
                    ensure!(
                        config.matcher.defined_vars().contains(variable.as_str()),
                        "metadata.{key} names ${variable}, which the rule does not match"
                    );
                    let shape =
                        shape(rule).with_context(|| format!("metadata.{key}.{variable}"))?;
                    Ok((variable, shape))
                })
                .collect::<Result<_>>()
        };
        let holds = held_shapes(metadata.holds, "holds")?;
        let unless_holds = held_shapes(metadata.unless_holds, "unlessHolds")?;
        let unless_imported =
            (metadata.unless_imported.map(shape).transpose()).context("metadata.unlessImported")?;
        let declared = metadata.public_id.unwrap_or_else(|| config.id.clone());
        let (package, id) = match (declared.split_once('/'), pack) {
            (None, Some(pack)) => (pack, declared.as_str()),
            (None, None) => (LOCAL_PACKAGE, declared.as_str()),
            (Some((package, id)), None) => (package, id),
            (Some((package, id)), Some(pack)) => {
                ensure!(
                    package == pack,
                    "the rule is named {declared}, and its file belongs to the pack {pack}"
                );
                (pack, id)
            }
        };
        ensure!(
            !package.is_empty() && !id.is_empty(),
            "the rule needs a name, as <rule> or <package>/<rule>, and has {declared:?}"
        );
        let id = id.to_owned();
        config.id = format!("{package}/{id}");
        Ok(Self {
            id,
            variant: VariantId(format!("{}#{number}", source.display())),
            config,
            source,
            fix,
            since: metadata.since,
            until: metadata.until,
            files,
            ignores,
            python_checks,
            holds,
            unless_holds,
            unless_imported,
        })
    }

    /// The package the rule reports under, which is the module of its pack.
    pub(crate) fn package(&self) -> &str {
        let package = self.config.id.len() - self.id.len() - 1;
        &self.config.id[..package]
    }
}

impl Rule {
    /// Whether the rule's advice works on the `installed` version of its library.
    pub(crate) fn applies_to(&self, installed: &Version) -> bool {
        self.since.as_ref().is_none_or(|since| since <= installed)
            && self.until.as_ref().is_none_or(|until| installed < until)
    }

    /// Whether the rule's advice works on the newest version of its library.
    pub(crate) fn applies_to_newest(&self) -> bool {
        self.until.is_none()
    }
}

impl Rule {
    /// Whether the rule runs on the file at `path`, by its `files:` and `ignores:` globs. A glob
    /// matches the path as it was given, so it starts with `**/` to match at any depth.
    pub(crate) fn runs_on(&self, path: &Path) -> bool {
        !self.ignores.as_ref().is_some_and(|set| set.is_match(path))
            && self.files.as_ref().is_none_or(|set| set.is_match(path))
    }

    /// What the rule reports in the parsed file `root`: the shared scanner's matches that
    /// `metadata.holds` keeps, or nothing when `metadata.unlessImported` silences the file. A
    /// rule without the Python checks reports every match.
    /// `disk` is where the file is, from which its imports are followed. Without it no import
    /// is read. A value proof starts from the pack's `library` as the file uses it.
    pub(crate) fn retain_matches<'r>(
        &self,
        root: &'r Root,
        matches: Vec<NodeMatch<'r, StrDoc<SupportLang>>>,
        disk: Option<&Path>,
        library: Library<'_>,
    ) -> Vec<NodeMatch<'r, StrDoc<SupportLang>>> {
        if !self.python_checks {
            return matches;
        }
        let proven = |found: &NodeMatch<'r, StrDoc<SupportLang>>,
                      conditions: &[(String, Shape)]| {
            let context = found.ancestors().find(|node| {
                node.parent()
                    .is_some_and(|parent| matches!(&*parent.kind(), "block" | "module"))
            });
            context.is_none_or(|context| names::imports_match(found, &context))
                && conditions.iter().all(|(variable, shape)| {
                    (found.get_env().get_match(variable))
                        .is_none_or(|usage| names::holds(usage, shape, library))
                })
        };
        let found: Vec<_> = matches
            .into_iter()
            .filter(|found| {
                proven(found, &self.holds)
                    && (self.unless_holds.is_empty() || !proven(found, &self.unless_holds))
            })
            .collect();
        let silenced = !found.is_empty()
            && (self.unless_imported.as_ref().zip(disk)).is_some_and(|(setting, disk)| {
                imports::any_loaded(disk, root, |module| {
                    module.root().find_all(setting).any(|setting| {
                        setting.ancestors().all(|parent| {
                            matches!(&*parent.kind(), "expression_statement" | "module")
                        })
                    })
                })
            });
        if silenced { Vec::new() } else { found }
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("id", &self.id)
            .field("variant", &self.variant)
            .field("source", &self.source)
            .field("fix", &self.fix)
            .field("since", &self.since)
            .field("until", &self.until)
            .finish_non_exhaustive()
    }
}
