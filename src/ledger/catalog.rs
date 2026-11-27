use std::collections::BTreeSet;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::output::Format;

use super::{ChecklistEntry, Finding, Kind, SurveyOutput, named};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Catalog {
    facts: Vec<Fact>,
    questions: Vec<Question>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Fact {
    id: String,
    text: String,
    how: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Question {
    id: String,
    text: String,
    look: String,
}

impl Catalog {
    pub(crate) fn load(source: &str) -> Result<Self> {
        serde_saphyr::from_str(source).context("the ledger question catalog does not parse")
    }

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

    pub(crate) fn checklist(
        &self,
        entities: BTreeSet<(String, String)>,
        kind: &str,
    ) -> Vec<ChecklistEntry> {
        entities
            .into_iter()
            .flat_map(|(entity, file)| {
                self.questions.iter().map(move |question| ChecklistEntry {
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

    pub(crate) fn modules<K: Kind, R>(
        &self,
        read: R,
        findings: Vec<Finding<K>>,
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
