//! Loads and prints YAML question and fact catalogs for ledger workflows.

use std::collections::BTreeSet;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::output::Format;

use super::{ChecklistEntry, LedgerFinding, LedgerKind, SurveyOutput, named};

/// Facts and review questions loaded from a `rules/xpd/<workflow>.yml` catalog.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct QuestionCatalog {
    facts: Vec<CatalogFact>,
    questions: Vec<CatalogQuestion>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CatalogFact {
    id: String,
    text: String,
    how: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CatalogQuestion {
    id: String,
    text: String,
    look: String,
}

impl QuestionCatalog {
    /// Parses a YAML question catalog string.
    pub(crate) fn load(source: &str) -> Result<Self> {
        serde_saphyr::from_str(source).context("the ledger question catalog does not parse")
    }

    /// Prints all catalog facts and questions in JSON or plain text.
    pub(crate) fn print(&self, format: Format) -> Result<ExitCode> {
        match format {
            Format::Json => println!("{}", serde_json::to_string(self)?),
            Format::Text => {
                println!("Facts");
                for fact in &self.facts {
                    println!("  {}  {} {}", fact.id, fact.text, fact.how);
                }
                println!("Questions");
                for question in &self.questions {
                    println!("  {}  {} {}", question.id, question.text, question.look);
                }
            }
        }
        Ok(ExitCode::SUCCESS)
    }

    /// Whether the catalog defines a fact with the identifier `id`.
    pub(crate) fn has_fact(&self, id: &str) -> bool {
        self.facts.iter().any(|fact| fact.id == id)
    }

    /// Expands catalog questions across `(entity, owner_file)` pairs for a checklist.
    pub(crate) fn checklist(
        &self,
        entities: BTreeSet<(String, String)>,
        kind: &str,
    ) -> Vec<ChecklistEntry> {
        entities
            .into_iter()
            .flat_map(|(entity, file)| {
                self.questions.iter().map(move |question| ChecklistEntry {
                    requires_na: false,
                    answer: None,
                    id: format!("{entity}/{}", question.id),
                    file: file.clone(),
                    figure: entity.clone(),
                    kind: kind.to_owned(),
                    question: question.id.clone(),
                    page: 0,
                    principle: question.id.clone(),
                    text: question.text.clone(),
                    look: question.look.clone(),
                })
            })
            .collect()
    }

    /// Builds a [`SurveyOutput`] attaching module-scoped checklist questions to files with findings.
    pub(crate) fn modules<K: LedgerKind, R>(
        &self,
        read: R,
        findings: Vec<LedgerFinding<K>>,
        tracked: impl Fn(&str) -> Option<String>,
    ) -> SurveyOutput<K, R> {
        let entities = findings
            .iter()
            .flat_map(|finding| &finding.places)
            .filter_map(|place| tracked(&place.path))
            .map(|path| (path.clone(), path))
            .collect();
        SurveyOutput {
            read,
            findings: named(findings),
            checklist: self.checklist(entities, "module"),
        }
    }
}
