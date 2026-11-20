//! The findings of one lint run, and how `check` and `hook` print them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::severity::Severity;
use serde_json::{Value, json};

use crate::fix_safety::Applicability;
use crate::rule_id::RuleId;

/// Where in a file an issue starts. Lines and columns count from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

/// One thing a rule or a package check found.
#[derive(Debug)]
pub(crate) struct Issue {
    pub(crate) id: RuleId,
    pub(crate) file: PathBuf,
    /// `None` for an issue about the project as a whole, such as a missing package.
    pub(crate) position: Option<Position>,
    pub(crate) severity: Severity,
    pub(crate) message: String,
    /// `None` when the rule has no fix.
    pub(crate) fix: Option<Applicability>,
    pub(crate) help: Option<String>,
}

impl Issue {
    fn fixable(&self, allowed: Applicability) -> bool {
        self.fix.is_some_and(|fix| fix <= allowed)
    }

    /// One line per issue. `[*]` marks an issue that `--fix` fixes at the `allowed` level.
    pub(crate) fn line(&self, allowed: Applicability) -> String {
        let place = match self.position {
            Some(Position { line, column }) => format!("{}:{line}:{column}", self.file.display()),
            None => self.file.display().to_string(),
        };
        let mark = if self.fixable(allowed) { " [*]" } else { "" };
        format!(
            "{place}: {} [{}]{mark} {}",
            self.severity, self.id, self.message
        )
    }

    fn to_json(&self) -> Value {
        json!({
            "package": self.id.package(),
            "rule": self.id.name(),
            "file": self.file.display().to_string(),
            "line": self.position.map(|at| at.line),
            "column": self.position.map(|at| at.column),
            "severity": self.severity.as_str(),
            "message": self.message,
            "fix": self.fix.map(Applicability::as_str),
        })
    }
}

/// Everything one run found.
#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) issues: Vec<Issue>,
    /// Number of fixes `--fix` wrote.
    pub(crate) fixed: usize,
    /// Number of files that could not be read, which the run named on stderr.
    pub(crate) skipped: usize,
}

impl Report {
    pub(crate) fn sarif(&self) -> Value {
        let ids: BTreeMap<_, _> = self
            .issues
            .iter()
            .map(|issue| (issue.id.to_string(), issue))
            .collect();
        let rules: Vec<_> = ids
            .iter()
            .map(|(id, issue)| {
                let mut rule = json!({
                    "id": id,
                    "shortDescription": {"text": issue.message},
                });
                if let Some(url) = &issue.help {
                    rule["helpUri"] = json!(url);
                }
                rule
            })
            .collect();
        let indexes: BTreeMap<_, _> = ids.keys().enumerate().map(|(n, id)| (id, n)).collect();
        let results: Vec<_> = self
            .issues
            .iter()
            .map(|issue| {
                let id = issue.id.to_string();
                let level = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    _ => "note",
                };
                let mut location = json!({
                    "artifactLocation": {"uri": artifact_uri(&issue.file)},
                });
                if let Some(position) = issue.position {
                    location["region"] = json!({
                        "startLine": position.line,
                        "startColumn": position.column,
                    });
                }
                json!({
                    "ruleId": id,
                    "ruleIndex": indexes[&id],
                    "level": level,
                    "message": {"text": issue.message},
                    "locations": [{"physicalLocation": location}],
                })
            })
            .collect();
        json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {
                    "name": "xpd",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": env!("CARGO_PKG_HOMEPAGE"),
                    "rules": rules,
                }},
                "columnKind": "unicodeCodePoints",
                "results": results,
            }],
        })
    }

    pub(crate) fn github(&self) -> Vec<String> {
        self.issues
            .iter()
            .map(|issue| {
                let level = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    _ => "notice",
                };
                let file = annotation_property(&issue.file.to_string_lossy());
                let title = annotation_property(&issue.id.to_string());
                let position = issue.position.map_or_else(String::new, |position| {
                    format!(",line={},col={}", position.line, position.column)
                });
                let message = annotation_text(&issue.message);
                format!("::{level} file={file}{position},title={title}::{message}")
            })
            .collect()
    }

    /// Whether any issue has the severity `lowest` or a higher one, which makes `check` exit 1.
    pub(crate) fn has_issues_from(&self, lowest: Severity) -> bool {
        (self.issues.iter()).any(|i| i.severity >= lowest)
    }

    /// `fixable` counts the issues that `--fix` fixes at the `allowed` level.
    pub(crate) fn to_json(&self, allowed: Applicability) -> Value {
        let issues: Vec<Value> = self.issues.iter().map(Issue::to_json).collect();
        let fixable = self.issues.iter().filter(|i| i.fixable(allowed)).count();
        json!({"fixed": self.fixed, "fixable": fixable, "issues": issues})
    }

    /// For example `fixed 2, 3 issues, 1 fixable with --fix, 1 more with --fix --unsafe-fixes`.
    pub(crate) fn summary(&self, allowed: Applicability) -> String {
        let mut parts = Vec::new();
        if self.fixed > 0 {
            parts.push(format!("fixed {}", self.fixed));
        }
        parts.push(match self.issues.len() {
            1 => "1 issue".to_owned(),
            n => format!("{n} issues"),
        });
        let fixable = self.issues.iter().filter(|i| i.fixable(allowed)).count();
        if fixable > 0 {
            let flags = match allowed {
                Applicability::Safe => "--fix",
                Applicability::Unsafe => "--fix --unsafe-fixes",
            };
            parts.push(format!("{fixable} fixable with {flags}"));
        }
        let hidden = self.issues.iter().filter(|i| i.fix > Some(allowed)).count();
        if hidden > 0 {
            parts.push(format!("{hidden} more with --fix --unsafe-fixes"));
        }
        parts.join(", ")
    }
}

fn annotation_text(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn annotation_property(value: &str) -> String {
    annotation_text(value)
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn artifact_uri(path: &std::path::Path) -> String {
    use std::fmt::Write as _;

    let path = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            write!(uri, "%{byte:02X}").expect("writing into a String cannot fail");
        }
    }
    uri
}
