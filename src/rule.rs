//! One lint rule: an ast-grep rule, plus the settings xpd reads from its `metadata`.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Error, Result, bail};
use ast_grep_config::RuleConfig;
use ast_grep_language::SupportLang;
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::version::Version;

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
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    fix: Option<Applicability>,
    since: Option<Version>,
}

/// One rule document of a pack.
pub(crate) struct Rule {
    pub(crate) config: RuleConfig<SupportLang>,
    /// `None` when the rule has no fix.
    pub(crate) fix: Option<Applicability>,
    /// The first library version the rule applies to.
    pub(crate) since: Option<Version>,
    /// The rule's `files:` globs. When set, the rule runs only on a path that matches one.
    files: Option<GlobSet>,
    /// The rule's `ignores:` globs. The rule skips a path that matches one.
    ignores: Option<GlobSet>,
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
                .context("metadata may hold fix: safe or unsafe, and since: \"<version>\"")?,
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
        Ok(Self {
            config,
            fix,
            since: metadata.since,
            files,
            ignores,
        })
    }
}

impl Rule {
    /// Whether the rule's advice works on the `installed` version of its library.
    pub(crate) fn applies_to(&self, installed: &Version) -> bool {
        self.since.as_ref().is_none_or(|since| since <= installed)
    }
}

impl Rule {
    /// Whether the rule runs on the file at `path`, by its `files:` and `ignores:` globs. A glob
    /// matches the path as it was given, so it starts with `**/` to match at any depth.
    pub(crate) fn runs_on(&self, path: &Path) -> bool {
        !self.ignores.as_ref().is_some_and(|set| set.is_match(path))
            && self.files.as_ref().is_none_or(|set| set.is_match(path))
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("id", &self.config.id)
            .field("fix", &self.fix)
            .field("since", &self.since)
            .finish_non_exhaustive()
    }
}
