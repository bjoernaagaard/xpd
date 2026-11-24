//! Finds the code that makes a conditional decision relevant, with the evidence rules of
//! `evidence.yml`, whose ids are the triggers that the catalog's records name. A match is a place
//! in a file; what it means for the project stays the developer's decision, which the record's
//! condition states.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ast_grep_config::{CombinedScan, DeserializeEnv, RuleConfig, SerializableGlobalRule};
use ast_grep_language::SupportLang;

use super::catalog::Trigger;
use crate::source;
use crate::syntax;

const RULES: &str = include_str!("evidence.yml");
const UTILS: &str = include_str!("evidence-utils.yml");

/// One place that a trigger found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(crate) struct Location {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) text: String,
}

/// The places each trigger found, in file order.
#[derive(Debug, Default)]
pub(crate) struct Evidence(BTreeMap<Trigger, Vec<Location>>);

impl Evidence {
    /// The places `trigger` found in one of `files`.
    pub(crate) fn within<'a>(
        &'a self,
        trigger: Trigger,
        files: &'a [PathBuf],
    ) -> impl Iterator<Item = &'a Location> {
        self.0
            .get(&trigger)
            .into_iter()
            .flatten()
            .filter(move |location| files.binary_search(&location.file).is_ok())
    }

    pub(crate) fn add(&mut self, trigger: Trigger, location: Location) {
        self.0.entry(trigger).or_default().push(location);
    }
}

fn configs() -> Result<Vec<RuleConfig<SupportLang>>> {
    let utils = ast_grep_config::from_multiple::<SerializableGlobalRule<SupportLang>>(UTILS)
        .context("evidence-utils.yml")?;
    let globals = DeserializeEnv::parse_global_utils(utils).context("evidence-utils.yml")?;
    ast_grep_config::from_yaml_string(RULES, &globals).context("evidence.yml")
}

/// Returns the IDs of the bundled doctor evidence rules.
///
/// # Errors
///
/// Returns an error if a bundled rule or utility cannot be loaded.
pub fn evidence_rule_ids() -> Result<Vec<String>> {
    Ok(configs()?
        .into_iter()
        .map(|config| config.id.clone())
        .collect())
}

/// The evidence rules, each with the trigger of its id.
fn rules() -> Result<Vec<(Trigger, RuleConfig<SupportLang>)>> {
    let mut configs = configs()?;
    let mut rules = Vec::new();
    for trigger in Trigger::ALL.into_iter().filter(|trigger| trigger.in_code()) {
        let name = trigger.name();
        let index = configs
            .iter()
            .position(|config| config.id == name)
            .with_context(|| format!("evidence.yml has no rule for the trigger {name}"))?;
        rules.push((trigger, configs.swap_remove(index)));
    }
    Ok(rules)
}

/// What every evidence rule finds in `files`, which must be sorted.
pub(crate) fn find(files: &[PathBuf]) -> Result<Evidence> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let rules = rules()?;
    let combined = CombinedScan::new(rules.iter().map(|(_, config)| config).collect());
    let per_file: Vec<Vec<(Trigger, Location)>> = files
        .par_iter()
        .map(|file| {
            let Some(src) = source::read_source(source::File::on_disk(file)) else {
                return Vec::new();
            };
            let root = syntax::parse(&src);
            let mut found = Vec::new();
            for (config, matches) in combined.scan(&root, false).matches {
                let Some((trigger, _)) = rules.iter().find(|(_, rule)| std::ptr::eq(rule, config))
                else {
                    continue;
                };
                for node in matches {
                    found.push((*trigger, location(file, &src, &node)));
                }
            }
            found
        })
        .collect();
    let mut evidence = Evidence::default();
    for (trigger, location) in per_file.into_iter().flatten() {
        evidence.add(trigger, location);
    }
    for locations in evidence.0.values_mut() {
        locations.sort();
        locations.dedup();
    }
    Ok(evidence)
}

/// The place of `node`, shown as the source line it starts on.
fn location(file: &Path, src: &str, node: &syntax::Node<'_>) -> Location {
    let row = node.start_pos().line();
    let line = src.lines().nth(row).unwrap_or_default().trim();
    let mut text: String = line.chars().take(80).collect();
    if text.len() < line.len() {
        text.push_str(" ...");
    }
    Location {
        file: file.to_path_buf(),
        line: row + 1,
        text,
    }
}
