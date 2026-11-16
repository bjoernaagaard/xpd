//! One lint rule: an ast-grep rule, plus the settings xpd reads from its `metadata`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use anyhow::{Context, Error, Result, bail, ensure};
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

/// What a rule's `metadata` may hold, such as `metadata: { fix: unsafe, since: "3.1" }`.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    fix: Option<Applicability>,
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

/// One rule document of a pack.
pub(crate) struct Rule {
    pub(crate) config: RuleConfig<SupportLang>,
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

impl TryFrom<RuleConfig<SupportLang>> for Rule {
    type Error = Error;

    /// A rule with `fix:` must say how safe the fix is with `metadata.fix`, and it may have
    /// only one fix, because `--fix` applies it without asking.
    fn try_from(config: RuleConfig<SupportLang>) -> Result<Self> {
        let metadata = match &config.metadata {
            Some(metadata) => serde_json::to_value(metadata)
                .and_then(serde_json::from_value)
                .context(
                    "metadata may hold fix: safe or unsafe, since: \"<version>\", until: \"<version>\", holds: { <VARIABLE>: <rule> }, unlessHolds: { <VARIABLE>: <rule> } and unlessImported: <rule>",
                )?,
            None => Metadata::default(),
        };
        let fix = match (config.fixer.len(), metadata.fix) {
            (0, None) => None,
            (1, Some(applicability)) => Some(applicability),
            (0, Some(_)) => bail!("metadata.fix is set, but the rule has no fix"),
            (1, None) => bail!("a rule with a fix needs metadata.fix: safe or unsafe"),
            (fixes, _) => bail!("a rule may have only one fix, and this one has {fixes}"),
        };
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
        Ok(Self {
            config,
            fix,
            since: metadata.since,
            until: metadata.until,
            files,
            ignores,
            holds,
            unless_holds,
            unless_imported,
        })
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
    /// `metadata.holds` keeps, or nothing when `metadata.unlessImported` silences the file.
    /// `disk` is where the file is, from which its imports are followed. Without it no import
    /// is read. A value proof starts from the pack's `library` as the file uses it.
    pub(crate) fn retain_matches<'r>(
        &self,
        root: &'r Root,
        matches: Vec<NodeMatch<'r, StrDoc<SupportLang>>>,
        disk: Option<&Path>,
        library: Library<'_>,
    ) -> Vec<NodeMatch<'r, StrDoc<SupportLang>>> {
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
            .field("id", &self.config.id)
            .field("fix", &self.fix)
            .field("since", &self.since)
            .field("until", &self.until)
            .finish_non_exhaustive()
    }
}
