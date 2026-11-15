//! The rule catalog that `xpd rules` prints: every rule and package check of every pack.

use ast_grep_config::Severity;
use serde_json::{Value, json};

use crate::pack::Pack;
use crate::package_check;
use crate::report::severity_name;
use crate::rule::Applicability;
use crate::version::Version;

/// The page whose `#<rule id>` anchors document each rule.
const RULES_URL: &str = concat!(env!("CARGO_PKG_HOMEPAGE"), "rules/");

/// One rule as the catalog lists it. A rule file may hold several documents with the same id,
/// one for each form the rule matches, and the catalog lists them as one rule: the highest
/// severity of its documents, the message of its first, the least safe fix, and the lowest
/// `since`, which is no version at all when one document applies to every version.
#[derive(Debug)]
pub(crate) struct Entry<'a> {
    /// Module name of the pack the rule belongs to.
    package: &'static str,
    rule: &'a str,
    severity: &'a Severity,
    message: &'a str,
    /// `None` when no document of the rule has a fix.
    fix: Option<Applicability>,
    since: Option<&'a Version>,
    doc: Option<&'static str>,
}

impl Entry<'_> {
    /// One line per rule, such as
    /// `error [pandas/chained-assignment] (unsafe fix) Chained assignment writes to a copy ...`.
    pub(crate) fn line(&self) -> String {
        let fix = self
            .fix
            .map(|fix| format!(" ({} fix)", fix.as_str()))
            .unwrap_or_default();
        let since = self
            .since
            .map(|since| format!(" (since {since})"))
            .unwrap_or_default();
        format!(
            "{} [{}/{}]{fix}{since} {}",
            severity_name(self.severity),
            self.package,
            self.rule,
            self.message
        )
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "package": self.package,
            "rule": self.rule,
            "severity": severity_name(self.severity),
            "message": self.message,
            "fix": self.fix.map(Applicability::as_str),
            "since": self.since.map(ToString::to_string),
            "url": format!("{RULES_URL}#{}", self.rule),
            "doc": self.doc,
        })
    }
}

/// How severe a severity is, so that merging a rule's documents keeps the highest.
fn rank(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 4,
        Severity::Warning => 3,
        Severity::Info => 2,
        Severity::Hint => 1,
        Severity::Off => 0,
    }
}

/// Every rule of `packs` and then every package check, pack by pack in name order.
pub(crate) fn entries(packs: &[Pack]) -> Vec<Entry<'_>> {
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for pack in packs {
        let first = entries.len();
        for rule in &pack.rules {
            let id = rule.config.id.as_str();
            let since = rule.since.as_ref();
            match entries[first..].iter_mut().find(|entry| entry.rule == id) {
                Some(entry) => {
                    if rank(&rule.config.severity) > rank(entry.severity) {
                        entry.severity = &rule.config.severity;
                    }
                    entry.fix = entry.fix.max(rule.fix);
                    entry.since = entry.since.zip(since).map(|(a, b)| a.min(b));
                }
                None => entries.push(Entry {
                    package: pack.module,
                    rule: id,
                    severity: &rule.config.severity,
                    message: &rule.config.message,
                    fix: rule.fix,
                    since,
                    doc: pack.doc(id),
                }),
            }
        }
        entries.extend(pack.package_checks.iter().map(|check| Entry {
            package: pack.module,
            rule: &check.id,
            severity: &package_check::SEVERITY,
            message: &check.message,
            fix: None,
            since: None,
            doc: pack.doc(&check.id),
        }));
    }
    entries
}
