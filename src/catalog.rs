//! The rule catalog of one project: the packs that ship with xpd, the project's own rules, and
//! xpd's rule about suppression comments. `xpd check`, `xpd rules`, the hook and the language
//! server all read their rules from it.

mod local;

use std::collections::HashMap;
use std::path::Path;

use crate::severity::Severity;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub(crate) use self::local::{LocalProject, is_module_name};
use crate::fix_safety::Applicability;
use crate::pack::{self, Pack};
use crate::package_check;
use crate::rule::Rule;
use crate::rule_id::RuleId;
use crate::style::{Mark, Style};
use crate::venv::Distributions;
use crate::version::Version;

/// The rules of one project for one set of installed libraries.
#[derive(Debug)]
pub(crate) struct Catalog {
    /// The embedded packs in name order, then the project's packs, then xpd's own pack. A
    /// scan reports the issues at one place in this order.
    pub(crate) packs: Vec<Pack>,
    /// The project whose rules the catalog holds beside the embedded ones.
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
    fn keeps(&self, package: &str, id: &str, severity: Severity) -> bool {
        let named = |selector: &String| match selector.split_once('/') {
            Some((in_package, rule)) => in_package == package && rule == id,
            None => selector == package || selector == id,
        };
        (self.rules.is_empty() || self.rules.iter().any(named))
            && self.severity.is_none_or(|lowest| severity >= lowest)
    }
}

impl Catalog {
    /// The catalog for the project that `from` is in, which is the nearest folder at or above
    /// it with an `sgconfig.yml`. Without such a folder the catalog holds the embedded rules.
    /// `installed` selects the rules for the library versions of the project's venv, and
    /// `style` the severity of the rules of the house style.
    pub(crate) fn load(
        from: &Path,
        installed: Option<&Distributions>,
        style: Style,
    ) -> Result<Self> {
        match LocalProject::find(from)? {
            Some(config) => Self::load_config(&config, installed, style),
            None => Self::embedded(installed, style),
        }
    }

    /// The catalog for the project of the project file `config`.
    pub(crate) fn load_config(
        config: &Path,
        installed: Option<&Distributions>,
        style: Style,
    ) -> Result<Self> {
        let (project, packs) = LocalProject::read(config)?;
        Self::assemble(packs, Some(project), installed, style)
    }

    /// The catalog of the rules that ship with xpd, whatever project the run is in.
    pub(crate) fn embedded(installed: Option<&Distributions>, style: Style) -> Result<Self> {
        Self::assemble(Vec::new(), None, installed, style)
    }

    fn assemble(
        local: Vec<Pack>,
        project: Option<LocalProject>,
        installed: Option<&Distributions>,
        style: Style,
    ) -> Result<Self> {
        let mut packs = pack::embedded().context("invalid embedded rules")?;
        packs.extend(local);
        packs.push(Pack::suppressions()?);
        let mut sources = DeclaredSources::default();
        for rule in packs.iter().flat_map(|pack| &pack.rules) {
            if let Some(first) = sources.record(rule) {
                bail!(
                    "the rule {} is in {} and in {}",
                    rule.id,
                    first.display(),
                    rule.source().display()
                );
            }
        }
        for pack in &mut packs {
            pack.keep_rules_for(installed);
            pack.style = style;
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
                let checks = pack.package_checks.iter().map(|check| check.id.as_str());
                let mut ids = pack.rules.iter().map(|rule| rule.id.name()).chain(checks);
                ids.any(|id| only.keeps(&pack.module, id, Severity::Error))
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
                style,
                ..
            } = pack;
            rules.retain(|rule| filters.keeps(module, rule.id.name(), rule.severity_under(*style)));
            package_checks
                .retain(|check| filters.keeps(module, &check.id, package_check::SEVERITY));
        }
        Ok(())
    }

    /// Checks an editor's rule document with the project's utilities and declared IDs.
    pub(crate) fn validate_document(&self, path: &Path, text: &str) -> Result<()> {
        let Some(project) = &self.project else {
            return Ok(());
        };
        let rules = project.check_document(path, text)?;
        let mut sources = DeclaredSources::default();
        for rule in self.packs.iter().flat_map(|pack| &pack.rules) {
            let _ = sources.record(rule);
        }
        for rule in &rules {
            if let Some(other) = sources.record(rule) {
                bail!(
                    "{}: the rule {} is also in {}",
                    rule.source().display(),
                    rule.id,
                    other.display()
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct DeclaredSources<'a>(HashMap<&'a RuleId, &'a Path>);

impl<'a> DeclaredSources<'a> {
    fn record(&mut self, rule: &'a Rule) -> Option<&'a Path> {
        let first = *self
            .0
            .entry(&rule.declared_id)
            .or_insert_with(|| rule.source());
        (first != rule.source()).then_some(first)
    }
}

#[derive(Debug)]
pub(crate) enum HelpLink {
    Url(String),
    File(std::path::PathBuf),
}

impl Catalog {
    pub(crate) fn help(&self, id: &RuleId) -> Option<(&str, Option<HelpLink>)> {
        let name = id.name();
        let find = |rule: &&Rule| {
            if id.source().is_some() {
                rule.id == *id
            } else {
                rule.id.name() == name
            }
        };
        let (pack, rule) = self
            .packs
            .iter()
            .filter(|pack| pack.module == id.package())
            .find_map(|pack| {
                let rule = pack.rules.iter().find(find);
                (rule.is_some() || (id.source().is_none() && pack.doc(name).is_some()))
                    .then_some((pack, rule))
            })?;
        let doc = pack.doc(name).or_else(|| {
            rule.map(|rule| rule.config.note.as_deref().unwrap_or(&rule.config.message))
        })?;
        let url = pack
            .page(name)
            .or_else(|| rule.and_then(|rule| rule.config.url.clone()));
        let link = url
            .map(HelpLink::Url)
            .or_else(|| rule.map(|rule| HelpLink::File(rule.source().to_owned())));
        Some((doc, link))
    }
}

/// One rule as the catalog lists it. A rule file may hold several documents with the same id,
/// one for each form the rule matches, and the catalog lists them as one rule: the highest
/// severity of its documents, the message of its first, the least safe fix, and the lowest
/// `since`, which is no version at all when one document applies to every version.
#[derive(Debug)]
pub(crate) struct Entry<'a> {
    id: RuleId,
    /// The severity that the catalog's run reports the rule with.
    severity: Severity,
    /// What `metadata.style` marks the rule as. `None` for a rule that is not about style.
    style: Option<Mark>,
    /// The severity without a style setting, and the one with `style = "strict"`.
    by_style: [Severity; 2],
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
            "{} [{}]{fix}{since} {}",
            self.severity, self.id, self.message
        )
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "package": self.id.package(),
            "rule": self.id.name(),
            "severity": self.severity.as_str(),
            "style": self.style.map(Mark::as_str),
            "severities": {
                Style::Default.as_str(): self.by_style[0].as_str(),
                Style::Strict.as_str(): self.by_style[1].as_str(),
            },
            "message": self.message,
            "fix": self.fix.map(Applicability::as_str),
            "since": self.since.map(ToString::to_string),
            "url": self.url,
            "doc": self.doc,
        })
    }
}

impl Catalog {
    /// Every rule of each pack and then its package checks, pack by pack.
    pub(crate) fn entries(&self) -> Vec<Entry<'_>> {
        let mut entries: Vec<Entry<'_>> = Vec::new();
        for pack in &self.packs {
            let first = entries.len();
            let page = |id: &str| pack.page(id);
            for rule in &pack.rules {
                let id = rule.id.name();
                let effective = rule.severity_under(pack.style);
                let since = rule.since.as_ref();
                let by_style =
                    [Style::Default, Style::Strict].map(|style| rule.severity_under(style));
                match entries[first..]
                    .iter_mut()
                    .find(|entry| entry.id.name() == id)
                {
                    Some(entry) => {
                        entry.severity = entry.severity.max(effective);
                        entry.style = entry.style.or(rule.style);
                        for (highest, severity) in entry.by_style.iter_mut().zip(by_style) {
                            *highest = (*highest).max(severity);
                        }
                        entry.fix = entry.fix.max(rule.fix);
                        entry.since = entry.since.zip(since).map(|(a, b)| a.min(b));
                    }
                    None => entries.push(Entry {
                        id: rule.id.clone(),
                        severity: effective,
                        style: rule.style,
                        by_style,
                        message: &rule.config.message,
                        fix: rule.fix,
                        since,
                        url: page(id).or_else(|| rule.config.url.clone()),
                        doc: pack.doc(id).or(rule.config.note.as_deref()),
                    }),
                }
            }
            entries.extend(pack.package_checks.iter().map(|check| Entry {
                id: RuleId::qualified(&pack.module, &check.id),
                severity: package_check::SEVERITY,
                style: None,
                by_style: [package_check::SEVERITY, package_check::SEVERITY],
                message: &check.message,
                fix: None,
                since: None,
                url: page(&check.id),
                doc: pack.doc(&check.id),
            }));
        }
        entries
    }
}
