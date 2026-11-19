//! The rule catalog of one project: the packs that ship with xpd, the project's own rules, and
//! xpd's rule about suppression comments. `xpd check`, `xpd rules`, the hook and the language
//! server all read their rules from it.

mod local;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use ast_grep_config::Severity;
use serde_json::{Value, json};

pub(crate) use self::local::{LocalProject, is_module_name};
use crate::pack::{self, Pack};
use crate::package_check;
use crate::report::severity_name;
use crate::rule::{Applicability, Rule};
use crate::venv::Distributions;
use crate::version::Version;

/// The folder of the published rule pages, one page for each pack and each rule in it.
const RULES_URL: &str = concat!(env!("CARGO_PKG_HOMEPAGE"), "rules/");

/// The address of the published page about the rule or package check `id` of the pack `module`.
pub(crate) fn url(module: &str, id: &str) -> String {
    format!("{RULES_URL}{module}/{id}/")
}

/// The rules of one project for one set of installed libraries.
#[derive(Debug)]
pub(crate) struct Catalog {
    /// The embedded packs in name order, then the project's packs, then xpd's own pack. A
    /// scan reports the issues at one place in this order.
    pub(crate) packs: Vec<Pack>,
    /// The project whose rules the catalog holds beside the embedded ones. Its `inputs` are
    /// the files to watch, and loading again reads them again.
    pub(crate) project: Option<LocalProject>,
}

/// Which rules of a catalog a run uses.
#[derive(Debug, Default)]
pub(crate) struct Filters {
    /// When not empty, only the rules and package checks that one entry names: a package such
    /// as `pandas`, a rule such as `chained-assignment`, or both as
    /// `pandas/chained-assignment`.
    pub(crate) rules: Vec<String>,
    /// The lowest severity that still runs.
    pub(crate) severity: Option<Severity>,
}

impl Filters {
    fn keeps(&self, package: &str, id: &str, severity: &Severity) -> bool {
        let named = |selector: &String| match selector.split_once('/') {
            Some((in_package, rule)) => in_package == package && rule == id,
            None => selector == package || selector == id,
        };
        (self.rules.is_empty() || self.rules.iter().any(named))
            && (self.severity.as_ref()).is_none_or(|lowest| rank(severity) >= rank(lowest))
    }
}

impl Catalog {
    /// The catalog for the project that `from` is in, which is the nearest folder at or above
    /// it with an `sgconfig.yml`. Without such a folder the catalog holds the embedded rules.
    /// `installed` selects the rules for the library versions of the project's venv.
    pub(crate) fn load(from: &Path, installed: Option<&Distributions>) -> Result<Self> {
        match LocalProject::find(from)? {
            Some(config) => Self::load_config(&config, installed),
            None => Self::embedded(installed),
        }
    }

    /// The catalog for the project of the project file `config`.
    pub(crate) fn load_config(config: &Path, installed: Option<&Distributions>) -> Result<Self> {
        let (project, packs) = LocalProject::read(config)?;
        Self::assemble(packs, Some(project), installed)
    }

    /// The catalog of the rules that ship with xpd, whatever project the run is in.
    pub(crate) fn embedded(installed: Option<&Distributions>) -> Result<Self> {
        Self::assemble(Vec::new(), None, installed)
    }

    fn assemble(
        local: Vec<Pack>,
        project: Option<LocalProject>,
        installed: Option<&Distributions>,
    ) -> Result<Self> {
        let mut packs = pack::embedded().context("invalid embedded rules")?;
        packs.extend(local);
        packs.push(Pack::suppressions()?);
        // Public names may be shared by variants, but declared names belong to one file.
        let mut sources: HashMap<&str, &Path> = HashMap::new();
        for rule in packs.iter().flat_map(|pack| &pack.rules) {
            let first = sources.entry(&rule.declared_id).or_insert(&rule.source);
            ensure!(
                *first == rule.source,
                "the rule {} is in {} and in {}",
                rule.config.id,
                first.display(),
                rule.source.display()
            );
        }
        for pack in &mut packs {
            pack.keep_rules_for(installed);
        }
        Ok(Self { packs, project })
    }

    /// Keeps the rules and package checks that `filters` select. A filtered catalog no longer
    /// reports unused suppression comments, because a comment may be for a rule that is left
    /// out of the run.
    pub(crate) fn select(&mut self, filters: &Filters) -> Result<()> {
        if filters.rules.is_empty() && filters.severity.is_none() {
            return Ok(());
        }
        self.packs
            .retain(|pack| pack.unused_suppression().is_none());
        for selector in &filters.rules {
            let only = Filters {
                rules: vec![selector.clone()],
                severity: None,
            };
            let known = self.packs.iter().any(|pack| {
                let checks = pack.package_checks.iter().map(|check| &check.id);
                let mut ids = pack.rules.iter().map(|rule| &rule.id).chain(checks);
                ids.any(|id| only.keeps(&pack.module, id, &Severity::Error))
            });
            if !known {
                bail!("no rule or package is named {selector}. `xpd rules` lists them");
            }
        }
        for pack in &mut self.packs {
            let Pack {
                module,
                rules,
                package_checks,
                ..
            } = pack;
            rules.retain(|rule| filters.keeps(module, &rule.id, &rule.config.severity));
            package_checks
                .retain(|check| filters.keeps(module, &check.id, &package_check::SEVERITY));
        }
        Ok(())
    }

    /// Checks `text` as the content of the YAML file at `path`, which need not be saved, the
    /// way the catalog would load it: a rule file compiles against the project's utility
    /// rules, with the pack that `xpd.yml` gives its folder, and none of its rules may have
    /// the name of a rule in another file.
    pub(crate) fn validate_document(&self, path: &Path, text: &str) -> Result<()> {
        let rules = if let Some(project) = &self.project {
            project.check_document(path, text)?
        } else {
            return Ok(());
        };
        let known = || self.packs.iter().flat_map(|pack| &pack.rules);
        for rule in &rules {
            let same_name = |other: &&Rule| other.declared_id == rule.declared_id;
            if let Some(other) =
                known().find(|other| same_name(other) && other.source != rule.source)
            {
                bail!(
                    "{}: the rule {} is also in {}",
                    rule.source.display(),
                    rule.config.id,
                    other.source.display()
                );
            }
        }
        Ok(())
    }
}

/// One rule as the catalog lists it. A rule file may hold several documents with the same id,
/// one for each form the rule matches, and the catalog lists them as one rule: the highest
/// severity of its documents, the message of its first, the least safe fix, and the lowest
/// `since`, which is no version at all when one document applies to every version.
#[derive(Debug)]
pub(crate) struct Entry<'a> {
    /// Module name of the pack the rule belongs to.
    package: &'a str,
    rule: &'a str,
    severity: &'a Severity,
    message: &'a str,
    /// `None` when no document of the rule has a fix.
    fix: Option<Applicability>,
    since: Option<&'a Version>,
    /// The page about the rule. `None` for a local rule without `url`.
    url: Option<String>,
    /// The rule's Markdown, which its published page shows, or the `note` of a local rule.
    doc: Option<&'a str>,
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
            "url": self.url,
            "doc": self.doc,
        })
    }
}

/// How severe a severity is, so that merging a rule's documents keeps the highest.
pub(crate) fn rank(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 4,
        Severity::Warning => 3,
        Severity::Info => 2,
        Severity::Hint => 1,
        Severity::Off => 0,
    }
}

impl Catalog {
    /// Every rule of each pack and then its package checks, pack by pack.
    pub(crate) fn entries(&self) -> Vec<Entry<'_>> {
        entries(&self.packs)
    }
}

fn entries(packs: &[Pack]) -> Vec<Entry<'_>> {
    let mut entries: Vec<Entry<'_>> = Vec::new();
    for pack in packs {
        let first = entries.len();
        let page = |id: &str| pack.page(id);
        for rule in &pack.rules {
            let id = rule.id.as_str();
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
                    package: &pack.module,
                    rule: id,
                    severity: &rule.config.severity,
                    message: &rule.config.message,
                    fix: rule.fix,
                    since,
                    url: page(id).or_else(|| rule.config.url.clone()),
                    doc: pack.doc(id).or(rule.config.note.as_deref()),
                }),
            }
        }
        entries.extend(pack.package_checks.iter().map(|check| Entry {
            package: &pack.module,
            rule: &check.id,
            severity: &package_check::SEVERITY,
            message: &check.message,
            fix: None,
            since: None,
            url: page(&check.id),
            doc: pack.doc(&check.id),
        }));
    }
    entries
}
