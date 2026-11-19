//! The documents `xpd new` writes: rules, test cases, utilities and the Markdown of a rule, each checked
//! as ast-grep and xpd read it.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use anyhow::{Context, Result, ensure};
use ast_grep_config::{DeserializeEnv, GlobalRules, SerializableGlobalRule, SerializableRule};
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rule::{Origin, Rule};
use crate::version::Version;

/// The name of a rule or utility, which is also its file name: lowercase ASCII letters, digits,
/// `-` and `_`, starting with a letter or digit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Name(String);

impl FromStr for Name {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut chars = text.chars();
        ensure!(
            chars
                .next()
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && chars
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'),
            "{text:?} is not a name: use lowercase letters, digits, - and _, starting with a letter or digit"
        );
        Ok(Self(text.to_owned()))
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Checks a pack's module, the name its library is imported by, such as `category_encoders`.
pub(super) fn module(text: &str) -> Result<String> {
    ensure!(
        crate::catalog::is_module_name(text),
        "{text:?} is not a module name, such as pandas or category_encoders"
    );
    Ok(text.to_owned())
}

/// A rule's id: `<pack>/<name>` in a pack, and `<name>` for a structural rule.
#[derive(Debug, Clone)]
pub(super) struct RuleId {
    pub(super) pack: Option<String>,
    pub(super) name: Name,
}

impl RuleId {
    pub(super) fn new(pack: Option<String>, name: Name) -> Self {
        Self { pack, name }
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.pack {
            Some(pack) => write!(f, "{pack}/{}", self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

pub(super) fn non_empty(text: &str, what: &str) -> Result<()> {
    ensure!(!text.trim().is_empty(), "{what} is empty");
    Ok(())
}

pub(super) fn version(text: &str) -> Result<Version> {
    text.parse()
}

/// Checks that `pattern` parses as one piece of `language` code, as `ast-grep run` reads it.
pub(super) fn check_pattern(pattern: &str, language: SupportLang) -> Result<()> {
    non_empty(pattern, "the pattern")?;
    let parsed = Pattern::try_new(pattern, language)
        .with_context(|| format!("{pattern:?} is not a {language} pattern"))?;
    ensure!(
        !parsed.has_error(),
        "{pattern:?} does not parse as {language} code"
    );
    Ok(())
}

/// An ast-grep rule written as YAML on the command line, such as `{kind: call}`.
pub(super) fn yaml_rule(text: &str) -> Result<Value> {
    let rule: Value =
        ast_grep_config::from_str(text).with_context(|| format!("{text:?} is not YAML"))?;
    serde_json::from_value::<SerializableRule>(rule.clone())
        .with_context(|| format!("{text:?} is not an ast-grep rule"))?;
    Ok(rule)
}

/// The `VAR=RULE` values of `--holds` or `--unless-holds`.
pub(super) fn conditions(values: &[String], flag: &str) -> Result<BTreeMap<String, Value>> {
    values
        .iter()
        .map(|value| {
            let (variable, rule) = value
                .split_once('=')
                .with_context(|| format!("{flag} takes VAR=RULE, not {value:?}"))?;
            let variable = variable.trim_start_matches('$');
            Ok((
                variable.to_owned(),
                yaml_rule(rule).with_context(|| format!("{flag} {variable}"))?,
            ))
        })
        .collect()
}

/// A new rule, as the user chose it.
#[derive(Debug)]
pub(super) struct RuleDoc {
    pub(super) id: RuleId,
    pub(super) language: SupportLang,
    pub(super) language_name: &'static str,
    pub(super) severity: &'static str,
    pub(super) message: String,
    pub(super) note: Option<String>,
    pub(super) pattern: String,
    /// The fix and how safe it is.
    pub(super) fix: Option<(String, &'static str)>,
    pub(super) since: Option<String>,
    pub(super) until: Option<String>,
    pub(super) holds: BTreeMap<String, Value>,
    pub(super) unless_holds: BTreeMap<String, Value>,
    pub(super) unless_imported: Option<Value>,
    pub(super) public_id: Option<String>,
}

/// A rule document in the order a person writes one.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuleYaml<'a> {
    id: String,
    language: &'a str,
    severity: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
    rule: PatternRule<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<MetadataYaml<'a>>,
}

#[derive(Serialize)]
struct PatternRule<'a> {
    pattern: &'a str,
}

/// xpd's settings in a rule's `metadata`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetadataYaml<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    public_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    until: Option<&'a str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    holds: &'a BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    unless_holds: &'a BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unless_imported: Option<&'a Value>,
}

impl RuleDoc {
    fn yaml(&self, public_id: Option<&str>) -> Result<String> {
        let metadata = MetadataYaml {
            public_id,
            fix: self.fix.as_ref().map(|(_, safety)| *safety),
            since: self.since.as_deref(),
            until: self.until.as_deref(),
            holds: &self.holds,
            unless_holds: &self.unless_holds,
            unless_imported: self.unless_imported.as_ref(),
        };
        let has_metadata = metadata.public_id.is_some()
            || metadata.fix.is_some()
            || metadata.since.is_some()
            || metadata.until.is_some()
            || !metadata.holds.is_empty()
            || !metadata.unless_holds.is_empty()
            || metadata.unless_imported.is_some();
        let document = RuleYaml {
            id: self.id.to_string(),
            language: self.language_name,
            severity: self.severity,
            message: &self.message,
            note: self.note.as_deref(),
            rule: PatternRule {
                pattern: &self.pattern,
            },
            fix: self.fix.as_ref().map(|(fix, _)| fix.as_str()),
            metadata: has_metadata.then_some(metadata),
        };
        Ok(serde_saphyr::to_string(&document)?)
    }

    /// The rule file's text, after checking it as `ast-grep scan` reads it, with the project's
    /// utilities, and as `xpd check` reads it.
    pub(super) fn checked_yaml(&self, utils: &GlobalRules) -> Result<String> {
        if self.id.pack.is_none() {
            ensure!(
                self.since.is_none()
                    && self.until.is_none()
                    && self.holds.is_empty()
                    && self.unless_holds.is_empty()
                    && self.unless_imported.is_none(),
                "--since, --until, --holds, --unless-holds and --unless-imported are about a \
                 library's Python code, so they need a --pack"
            );
        }
        if let Some(public_id) = &self.public_id {
            check_public_id(public_id, &self.id)?;
        }
        check_pattern(&self.pattern, self.language)?;
        let text = self.yaml(self.public_id.as_deref())?;
        let mut configs = ast_grep_config::from_yaml_string::<SupportLang>(&text, utils)
            .context("ast-grep does not read the new rule")?;
        ensure!(configs.len() == 1, "the new rule is not one document");
        let config = configs.pop().context("the new rule is empty")?;
        ensure!(
            config.id == self.id.to_string() && config.language == self.language,
            "the new rule does not keep its id and language"
        );
        let origin = if self.id.pack.is_some() {
            Origin::LocalPack
        } else {
            Origin::Structural
        };
        let rule = Rule::new(
            config,
            origin,
            self.id.pack.as_deref(),
            self.id.to_string().into(),
            1,
        )
        .context("xpd does not read the new rule")?;
        if let (Some(since), Some(until)) = (&rule.since, &rule.until) {
            ensure!(
                since < until,
                "--since {since} is not before --until {until}"
            );
        }
        Ok(text)
    }

    /// The Markdown that explains the rule, in the form of xpd's own rule pages: the message,
    /// the note, then what the rule reports and how to fix it.
    pub(super) fn markdown(&self) -> String {
        let language = self.language_name;
        let versions = match (&self.since, &self.until) {
            (Some(since), Some(until)) => Some(format!("from {since}, and before {until}")),
            (Some(since), None) => Some(format!("from {since}")),
            (None, Some(until)) => Some(format!("before {until}")),
            (None, None) => None,
        };
        let fix = self.fix.as_ref().map(|(fix, safety)| {
            let command = match *safety {
                "safe" => "`xpd check --fix`",
                _ => "`xpd check --fix --unsafe-fixes`",
            };
            format!(
                "## How to fix it\n\n{command} applies the {safety} fix of this rule, which writes the code below.\n\n```{language}\n{fix}\n```"
            )
        });
        let paragraphs = [
            Some(self.message.clone()),
            self.note.clone(),
            Some(format!(
                "## What the rule reports\n\nThe rule reports code that matches this pattern.\n\n```{language}\n{}\n```",
                self.pattern
            )),
            (versions.zip(self.id.pack.as_ref()))
                .map(|(versions, pack)| format!("It applies to {pack} {versions}.")),
            fix,
        ];
        let mut text = paragraphs
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n\n");
        text.push('\n');
        text
    }
}

/// A public id names the rule's pack as its own id does, so its issues stay in the pack.
fn check_public_id(public_id: &str, id: &RuleId) -> Result<()> {
    let (pack, name) = match public_id.split_once('/') {
        Some((pack, name)) => (Some(pack), name),
        None => (None, public_id),
    };
    name.parse::<Name>()?;
    ensure!(
        pack == id.pack.as_deref(),
        "--public-id {public_id} must name the rule's pack as its id {id} does"
    );
    ensure!(
        public_id != id.to_string(),
        "--public-id is the rule's own id"
    );
    Ok(())
}

/// A test case with no code yet, which a person fills with the code the rule must report and
/// the code it must leave alone.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestCase {
    id: String,
    valid: Vec<String>,
    invalid: Vec<String>,
}

pub(super) fn empty_test(id: &str) -> Result<String> {
    let text = serde_saphyr::to_string(&TestCase {
        id: id.to_owned(),
        valid: Vec::new(),
        invalid: Vec::new(),
    })?;
    let read: TestCase =
        ast_grep_config::from_str(&text).context("the test case does not parse")?;
    ensure!(
        read.id == id && read.valid.is_empty() && read.invalid.is_empty(),
        "the test case does not keep its id"
    );
    Ok(text)
}

/// A global utility.
#[derive(Serialize)]
struct UtilYaml<'a> {
    id: &'a str,
    language: &'a str,
    rule: PatternRule<'a>,
}

/// The utility file's text, after checking that ast-grep compiles it with the project's other
/// utilities, given as the text of each file.
pub(super) fn checked_util(
    name: &Name,
    language: SupportLang,
    language_name: &str,
    pattern: &str,
    others: &[(String, String)],
) -> Result<String> {
    check_pattern(pattern, language)?;
    let text = serde_saphyr::to_string(&UtilYaml {
        id: &name.0,
        language: language_name,
        rule: PatternRule { pattern },
    })?;
    let mut utils = Vec::new();
    for (path, other) in others {
        let util: SerializableGlobalRule<SupportLang> =
            ast_grep_config::from_str(other).with_context(|| format!("{path} does not parse"))?;
        utils.push(util);
    }
    utils.push(ast_grep_config::from_str(&text).context("the new utility does not parse")?);
    DeserializeEnv::parse_global_utils(utils).context("ast-grep does not compile the utilities")?;
    Ok(text)
}
