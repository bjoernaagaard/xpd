use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use anyhow::{Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::run::{AnswerRecord, Answered, Verdict};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct FeatureEvidence {
    context: Value,
    candidates: BTreeMap<String, Value>,
    facts: Value,
}

impl FeatureEvidence {
    pub(super) fn from_read(read: &Value) -> Result<Self> {
        let manifest = &read["manifest"];
        let candidates = manifest["features"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|feature| {
                let id = feature["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("feature review candidate has no ID"))?;
                Ok((id.to_owned(), feature.clone()))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut context = manifest.clone();
        if let Some(fields) = context.as_object_mut() {
            fields.remove("features");
        }
        let facts = read["facts"].clone();
        ensure!(
            !facts.is_null()
                || !candidates
                    .values()
                    .any(|candidate| candidate["kind"] == "feature"),
            "recorded feature-review with implemented candidates requires an adapter and measured lift"
        );
        Ok(Self {
            context,
            candidates,
            facts,
        })
    }

    pub(super) fn compatible(&self, other: &Self) -> Result<()> {
        for field in ["development", "target", "prediction_cutoff", "baseline"] {
            ensure!(
                self.context[field] == other.context[field],
                "manifest {field} changed during feature review"
            );
        }
        let protocol = &self.facts["protocol"];
        let compared = &other.facts["protocol"];
        for field in [
            "development_fingerprint",
            "split_fingerprint",
            "mode",
            "baseline",
            "metric",
            "direction",
            "development_rows",
        ] {
            ensure!(
                protocol[field] == compared[field],
                "feature adapter protocol changed {field}"
            );
        }
        let identities = |facts: &Value| -> Vec<Value> {
            facts["folds"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|fold| {
                    json!([
                        fold["id"],
                        fold["train_count"],
                        fold["validation_count"],
                        fold["boundary"]
                    ])
                })
                .collect()
        };
        ensure!(
            identities(&self.facts) == identities(&other.facts),
            "feature adapter fold identities or boundaries changed"
        );
        Ok(())
    }

    pub(super) fn stale_items(
        &self,
        current: &Self,
        changed: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let questions = ["mechanism", "availability", "regimes", "lineage", "value"];
        self.candidates
            .keys()
            .chain(current.candidates.keys())
            .flat_map(|id| {
                let before = self.candidates.get(id);
                let after = current.candidates.get(id);
                let owner_changed = [before, after].into_iter().flatten().any(|candidate| {
                    candidate["owner"]
                        .as_str()
                        .is_some_and(|owner| changed.contains(owner))
                });
                let all = owner_changed || before != after || self.context != current.context;
                let value = fold_scores(&self.facts, id).ne(fold_scores(&current.facts, id));
                questions
                    .into_iter()
                    .filter(move |question| all || (*question == "value" && value))
                    .map(move |question| format!("{id}/{question}"))
            })
            .collect()
    }

    pub(super) fn validate_answer(&self, answer: &Answered) -> Result<()> {
        let (id, question) = answer
            .item
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("invalid feature checklist ID {}", answer.item))?;
        let candidate = self
            .candidates
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("the checklist has no candidate {id}"))?;
        let reason = answer.reason.trim();
        ensure!(
            !reason.is_empty(),
            "the answered item {} has no reason",
            answer.item
        );
        let hypothesis = candidate["kind"] == "hypothesis";
        if answer.verdict == Verdict::NotApplicable {
            ensure!(
                hypothesis && matches!(question, "lineage" | "value"),
                "na does not apply to {}",
                answer.item
            );
            return Ok(());
        }
        if question == "mechanism" {
            ensure!(
                answer.verdict == Verdict::Pass && SOURCE_LOCATOR.is_match(reason),
                "{} needs pass and a library key with a page or a primary source locator",
                answer.item
            );
        }
        if question == "availability" {
            ensure!(
                answer.verdict == Verdict::Pass,
                "{} needs an availability justification with pass",
                answer.item
            );
        }
        if question == "value" {
            ensure!(
                fold_scores(&self.facts, id).count() >= 2,
                "{} needs measured fold lift evidence",
                answer.item
            );
        }
        Ok(())
    }
}

static SOURCE_LOCATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)(https?://\S+|\bdoi\s*:\s*\S+|\b[a-z]+\d{4}[a-z0-9_-]*\b.*(?:\bp{1,2}\.\s*\d|\bpages?\s+\d|§\s*\w|\bsection\s+\w))"
).expect("valid source-locator pattern")
});

fn fold_scores<'a>(facts: &'a Value, id: &'a str) -> impl Iterator<Item = (&'a Value, &'a Value)> {
    facts["folds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(move |fold| Some((fold.get("baseline_score")?, fold["ablation"].get(id)?)))
}

pub(super) fn plan_invalidation(
    recorded: Option<&FeatureEvidence>,
    measured: Option<&FeatureEvidence>,
    answers: &[Answered],
) -> Result<BTreeSet<String>> {
    match (recorded, measured) {
        (Some(recorded), Some(measured)) => {
            recorded.compatible(measured)?;
            for answer in answers {
                measured.validate_answer(answer)?;
            }
            Ok(recorded.stale_items(measured, &BTreeSet::new()))
        }
        _ => Ok(BTreeSet::new()),
    }
}

pub(super) fn gate_invalidation(
    recorded: Option<&FeatureEvidence>,
    before: Option<&FeatureEvidence>,
    after: Option<&FeatureEvidence>,
    changed: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    match (recorded, before, after) {
        (Some(recorded), Some(before), Some(after)) => {
            recorded.compatible(before)?;
            before.compatible(after)?;
            Ok(recorded
                .stale_items(after, changed)
                .union(&recorded.stale_items(before, &BTreeSet::new()))
                .cloned()
                .collect())
        }
        _ => Ok(BTreeSet::new()),
    }
}

pub(super) fn invalidate_answers(
    answers: &mut BTreeMap<String, AnswerRecord>,
    stale: &BTreeSet<String>,
) {
    answers.retain(|item, _| !stale.contains(item));
}
