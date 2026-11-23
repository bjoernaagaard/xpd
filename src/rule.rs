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

use crate::fix_safety::Applicability;
use crate::frames;
use crate::imports;
use crate::names;
use crate::pack::Library;
use crate::rule_id::RuleId;
use crate::severity::Severity;
use crate::style::{Mark, Style};
use crate::syntax::Root;
use crate::version::Version;

/// A shape of code that a rule's metadata names, as an ast-grep rule that can use the `utils`
/// of its document.
type Shape = ast_grep_config::Rule;

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

impl Origin {
    fn fix_policy(self) -> FixPolicy {
        match self {
            Self::Embedded => FixPolicy::ExplicitSingle,
            Self::LocalPack | Self::Structural => FixPolicy::FirstWithUnsafeDefault,
        }
    }
}

#[derive(Clone, Copy)]
enum FixPolicy {
    ExplicitSingle,
    FirstWithUnsafeDefault,
}

impl FixPolicy {
    fn resolve(
        self,
        count: usize,
        declared: Option<Applicability>,
    ) -> Result<Option<Applicability>> {
        if count == 0 {
            ensure!(
                declared.is_none(),
                "metadata.fix is set, but the rule has no fix"
            );
            return Ok(None);
        }
        match self {
            Self::ExplicitSingle => {
                ensure!(
                    count == 1,
                    "a rule may have only one fix, and this one has {count}"
                );
                declared
                    .map(Some)
                    .context("a rule with a fix needs metadata.fix: safe or unsafe")
            }
            Self::FirstWithUnsafeDefault => Ok(Some(declared.unwrap_or(Applicability::Unsafe))),
        }
    }
}

macro_rules! metadata {
    ($($(#[$attr:meta])* $field:ident: $ty:ty $(=> $pack_key:literal)?,)*) => {
        #[derive(Default, Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Metadata {
            $($(#[$attr])* $(#[serde(rename = $pack_key)])? $field: $ty,)*
        }

        const PACK_SETTINGS: &[&str] = &[$($($pack_key,)?) *];
    };
}

metadata! {
    fix: Option<Applicability>,
    #[serde(default)]
    fix_imports: Vec<String> => "fixImports",
    /// Several documents can report under the same name while keeping distinct IDs in ast-grep.
    public_id: Option<String>,
    since: Option<Version> => "since",
    until: Option<Version> => "until",
    /// The captured variable must hold a value of the rule's shape.
    #[serde(default)]
    holds: BTreeMap<String, SerializableRule> => "holds",
    /// Drops a match when the same proof as `holds` succeeds.
    #[serde(default)]
    unless_holds: BTreeMap<String, SerializableRule> => "unlessHolds",
    /// Drops matches when an imported module contains the given shape.
    unless_imported: Option<SerializableRule> => "unlessImported",
    style: Option<Mark>,
    /// The captured variable must hold a pandas frame at the start of the statement.
    #[serde(default)]
    frame: BTreeMap<String, frames::Role> => "frame",
    /// The minimum number of top-level frame statements for the rule to match a script.
    frame_script: Option<usize> => "frameScript",
}

pub(crate) fn pack_setting(
    fields: &serde_json::Map<String, serde_json::Value>,
) -> Option<&'static str> {
    PACK_SETTINGS
        .iter()
        .copied()
        .find(|key| fields.contains_key(*key))
}

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
            if let Some(setting) = pack_setting(fields) {
                bail!(
                    "metadata.{setting} works only in a rule of a pack. Name the rule's folder under ruleDirs of a pack in xpd.yml"
                );
            }
            fields.retain(|key, _| matches!(key.as_str(), "fix" | "publicId"));
        }
        serde_json::from_value(value).context(
            "metadata may hold fix: safe or unsafe, publicId: <name>, since: \"<version>\", until: \"<version>\", holds: { <VARIABLE>: <rule> }, unlessHolds: { <VARIABLE>: <rule> }, unlessImported: <rule>, style: house, frame: { <VARIABLE>: any or parameter } and frameScript: <count>",
        )
    }
}

/// One rule document of a pack.
pub(crate) struct Rule {
    pub(crate) id: RuleId,
    pub(crate) declared_id: RuleId,
    /// ast-grep uses the public name to match suppression comments across variants.
    pub(crate) config: RuleConfig<SupportLang>,
    /// `None` when the rule has no fix.
    pub(crate) fix: Option<Applicability>,
    pub(crate) fix_imports: Vec<String>,
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
    /// From `metadata.style`. `None` for a rule that is not about style.
    pub(crate) style: Option<Mark>,
    /// From `metadata.frame`: each variable of the rule that must name a pandas frame, which
    /// `frames::holds` proves from the file.
    frames: Vec<(String, frames::Role)>,
    /// From `metadata.frameScript`: the fewest frame statements at the top of a module that
    /// make it a script, which `frames::script_start` counts.
    frame_script: Option<usize>,
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

/// The variables of `metadata.frame` with their roles. Each must be a variable of the rule.
fn frame_roles(
    roles: BTreeMap<String, frames::Role>,
    config: &RuleConfig<SupportLang>,
) -> Result<Vec<(String, frames::Role)>> {
    for variable in roles.keys() {
        ensure!(
            config.matcher.defined_vars().contains(variable.as_str()),
            "metadata.frame names ${variable}, which the rule does not match"
        );
    }
    Ok(roles.into_iter().collect())
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
        let fix = origin
            .fix_policy()
            .resolve(config.fixer.len(), metadata.fix)?;
        let python_checks = origin != Origin::Structural && config.language == SupportLang::Python;
        ensure!(
            metadata.fix_imports.is_empty()
                || (python_checks && fix == Some(Applicability::Unsafe)),
            "metadata.fixImports needs an unsafe fix in a Python rule pack"
        );
        ensure!(
            python_checks
                || (metadata.holds.is_empty()
                    && metadata.unless_holds.is_empty()
                    && metadata.unless_imported.is_none()
                    && metadata.frame.is_empty()
                    && metadata.frame_script.is_none()),
            "metadata.holds, unlessHolds, unlessImported, frame and frameScript read Python code, and the rule's language is {}",
            config.language
        );
        let frames = frame_roles(metadata.frame, &config)?;
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
        let declared_id = RuleId::declared(&config.id, pack);
        let reported = metadata.public_id.as_deref().unwrap_or(&config.id);
        let id = RuleId::reported(reported, pack, source, number)?;
        config.id = id.to_string();
        Ok(Self {
            id,
            declared_id,
            config,
            fix,
            fix_imports: metadata.fix_imports,
            since: metadata.since,
            until: metadata.until,
            files,
            ignores,
            python_checks,
            holds,
            unless_holds,
            unless_imported,
            style: metadata.style,
            frames,
            frame_script: metadata.frame_script,
        })
    }

    pub(crate) fn source(&self) -> &Path {
        self.id
            .source()
            .expect("a compiled rule has a source document")
    }

    /// The severity that a run with the setting `style` reports the rule with.
    pub(crate) fn severity_under(&self, style: Style) -> Severity {
        style.severity(Severity::from(&self.config.severity), self.style)
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
    /// `metadata.holds`, `metadata.frame` and `metadata.frameScript` keep, or nothing when
    /// `metadata.unlessImported` silences the file. A rule without the Python checks reports
    /// every match.
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
        let frames = frames::Frames::new();
        let is_frame = |found: &NodeMatch<'r, StrDoc<SupportLang>>| {
            self.frames.iter().all(|(variable, role)| {
                (found.get_env().get_match(variable)).is_none_or(|usage| frames.holds(usage, *role))
            })
        };
        let script = std::cell::OnceCell::new();
        let starts_script = |found: &NodeMatch<'r, StrDoc<SupportLang>>| {
            self.frame_script.is_none_or(|least| {
                *script.get_or_init(|| frames.script_start(root, least)) == Some(found.node_id())
            })
        };
        let found: Vec<_> = matches
            .into_iter()
            .filter(|found| {
                proven(found, &self.holds)
                    && (self.unless_holds.is_empty() || !proven(found, &self.unless_holds))
                    && is_frame(found)
                    && starts_script(found)
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
            .field("fix", &self.fix)
            .field("since", &self.since)
            .field("until", &self.until)
            .finish_non_exhaustive()
    }
}
