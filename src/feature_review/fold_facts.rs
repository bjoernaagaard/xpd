use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ledger::adapter::invoke_adapter;

use super::{FeatureKind, Manifest};

const RUNNER: &str = include_str!("../../py/feature_facts.py");
const MINIMUM_CORRELATION_PAIRS: u32 = 3;
const MINIMUM_CORRELATION: f64 = 0.95;
const TOLERANCE: f64 = 1e-8;

#[derive(Debug, Deserialize, Serialize)]
struct Protocol {
    development_fingerprint: String,
    split_fingerprint: String,
    development_rows: u32,
    mode: String,
    baseline: Vec<String>,
    metric: String,
    direction: String,
    justification: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Fold {
    id: Value,
    train_count: u32,
    validation_count: u32,
    boundary: Boundary,
    baseline_score: f64,
    stability: BTreeMap<String, Stability>,
    unseen: BTreeMap<String, Unseen>,
    clusters: Clusters,
    ablation: BTreeMap<String, Ablation>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Boundary {
    overlap: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_overlap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    train_latest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_cutoff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    train_label_latest: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Stability {
    usable_count: u32,
    missing_share: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    levels: Option<u32>,
    median: Option<f64>,
    iqr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Unseen {
    numerator: u32,
    denominator: u32,
    missing: u32,
    rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Clusters {
    edges: Vec<Edge>,
    components: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Edge {
    left: String,
    right: String,
    coefficient: f64,
    pairs: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Ablation {
    score: f64,
    improvement: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Facts {
    schema_version: u32,
    protocol: Protocol,
    folds: Vec<Fold>,
}

pub(super) fn measure(adapter: &str, root: &Path, manifest: &Manifest) -> Result<Value> {
    let python = root.join(adapter).is_file()
        && Path::new(adapter)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("py"));
    let payload = invoke_adapter(adapter, root, RUNNER)?;
    let facts: Facts = serde_json::from_value(payload).context("invalid feature adapter schema")?;
    let candidates = manifest
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Feature)
        .map(|feature| feature.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        facts.schema_version == 1,
        "unsupported feature adapter schema version"
    );
    facts.protocol.validate(&manifest.baseline)?;
    ensure!(
        facts.folds.len() >= 2,
        "adapter needs at least two distinct inner folds"
    );
    let mut ids = BTreeSet::new();
    for fold in &facts.folds {
        ensure!(
            (nonblank(fold.id.as_str()) || fold.id.as_u64().is_some())
                && ids.insert(fold.id.to_string()),
            "fold IDs must be nonempty and distinct"
        );
        fold.validate(&facts.protocol, &candidates)?;
    }
    let fold_count = f64::from(u32::try_from(facts.folds.len()).context("too many inner folds")?);
    let summary: BTreeMap<_, _> = candidates
        .into_iter()
        .map(|id| {
            let improvements = facts.folds.iter().map(|fold| fold.ablation[id].improvement);
            let mean = improvements
                .clone()
                .map(|lift| lift / fold_count)
                .sum::<f64>();
            let minimum = improvements.clone().fold(f64::INFINITY, f64::min);
            let maximum = improvements.fold(f64::NEG_INFINITY, f64::max);
            (id, json!({"mean": mean, "range": [minimum, maximum]}))
        })
        .collect();
    let mut measured = serde_json::to_value(facts)?;
    measured["adapter_mode"] = json!(if python { "python" } else { "command" });
    measured["ablation"] = json!(summary);
    Ok(measured)
}

fn nonblank(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

impl Protocol {
    fn validate(&self, baseline: &[String]) -> Result<()> {
        ensure!(
            !self.development_fingerprint.trim().is_empty()
                && !self.split_fingerprint.trim().is_empty()
                && !self.metric.trim().is_empty(),
            "feature adapter protocol needs fingerprints and a metric"
        );
        ensure!(
            self.baseline == baseline,
            "feature adapter baseline differs from the manifest"
        );
        ensure!(
            matches!(self.direction.as_str(), "loss" | "gain"),
            "metric direction must be loss or gain"
        );
        ensure!(
            matches!(self.mode.as_str(), "time" | "group" | "both" | "iid"),
            "unsupported inner split mode"
        );
        ensure!(
            self.mode != "iid" || nonblank(self.justification.as_deref()),
            "IID needs justification"
        );
        Ok(())
    }
}

fn utc_time(value: Option<&str>) -> Option<(&str, &str)> {
    let value = value?;
    let value = value
        .strip_suffix("+00:00")
        .or_else(|| value.strip_suffix('Z'))?;
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
    let valid = seconds.len() == 19
        && seconds.bytes().enumerate().all(|(at, byte)| match at {
            4 | 7 => byte == b'-',
            10 => byte == b'T',
            13 | 16 => byte == b':',
            _ => byte.is_ascii_digit(),
        })
        && fraction.bytes().all(|byte| byte.is_ascii_digit());
    valid.then_some((seconds, fraction.trim_end_matches('0')))
}

impl Boundary {
    fn validate(&self, mode: &str) -> Result<()> {
        ensure!(
            self.overlap == 0,
            "inner fold train and validation rows overlap"
        );
        if matches!(mode, "time" | "both") {
            let latest = utc_time(self.train_latest.as_deref())
                .context("invalid training timestamp evidence")?;
            let cutoff = utc_time(self.validation_cutoff.as_deref())
                .context("invalid validation cutoff evidence")?;
            let label = utc_time(self.train_label_latest.as_deref())
                .context("invalid training label timestamp evidence")?;
            ensure!(
                latest < cutoff,
                "inner training prediction timestamps reach validation cutoff"
            );
            ensure!(
                label <= cutoff,
                "inner training label was unavailable at validation cutoff"
            );
        }
        if matches!(mode, "group" | "both") {
            ensure!(
                self.group_overlap == Some(0),
                "inner training and validation groups overlap or lack isolation evidence"
            );
        }
        Ok(())
    }
}

impl Fold {
    fn validate(&self, protocol: &Protocol, candidates: &BTreeSet<&str>) -> Result<()> {
        ensure!(
            self.train_count > 0
                && self.validation_count > 0
                && self.train_count.saturating_add(self.validation_count)
                    <= protocol.development_rows,
            "invalid inner fold row counts"
        );
        self.boundary.validate(&protocol.mode)?;
        ensure!(self.baseline_score.is_finite(), "nonfinite baseline score");
        ensure!(
            self.stability
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == *candidates
                && self
                    .ablation
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>()
                    == *candidates,
            "fold {} does not cover every implemented candidate with stability and ablation",
            self.id
        );
        let categories: BTreeSet<_> = self
            .stability
            .iter()
            .filter(|(_, row)| row.levels.is_some())
            .map(|(id, _)| id)
            .collect();
        ensure!(
            self.unseen.keys().collect::<BTreeSet<_>>() == categories,
            "categorical candidates need unseen-level evidence"
        );
        for (feature, stats) in &self.stability {
            stats.validate(self.train_count)?;
            let score = &self.ablation[feature];
            let measured = if protocol.direction == "loss" {
                self.baseline_score - score.score
            } else {
                score.score - self.baseline_score
            };
            ensure!(
                score.score.is_finite() && close(measured, score.improvement),
                "invalid paired ablation score for {feature}"
            );
        }
        for counts in self.unseen.values() {
            ensure!(
                counts.denominator.saturating_add(counts.missing) == self.validation_count
                    && counts.numerator <= counts.denominator,
                "invalid unseen-level counts"
            );
            if counts.denominator == 0 {
                ensure!(
                    counts.rate.is_none() && nonblank(counts.reason.as_deref()),
                    "unseen-level rate is undefined without observed levels"
                );
            } else {
                ensure!(
                    counts.rate.is_some_and(|rate| close(
                        rate,
                        ratio(counts.numerator, counts.denominator)
                    )),
                    "unseen-level rate does not match its counts"
                );
            }
        }
        self.clusters.validate(&self.stability)
    }
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    f64::from(numerator) / f64::from(denominator)
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

impl Stability {
    fn validate(&self, train_count: u32) -> Result<()> {
        ensure!(
            self.usable_count <= train_count
                && close(
                    self.missing_share,
                    1.0 - ratio(self.usable_count, train_count)
                ),
            "invalid training stability counts or missing share"
        );
        if let Some(levels) = self.levels {
            ensure!(
                levels <= self.usable_count
                    && (levels > 0) == (self.usable_count > 0)
                    && self.median.is_none()
                    && self.iqr.is_none(),
                "invalid categorical stability evidence"
            );
        } else if self.usable_count == 0 {
            ensure!(
                self.median.is_none() && self.iqr.is_none() && nonblank(self.reason.as_deref()),
                "undefined numeric stability needs a reason"
            );
        } else {
            ensure!(
                self.median.is_some_and(f64::is_finite)
                    && self.iqr.is_some_and(|iqr| iqr.is_finite() && iqr >= 0.0),
                "invalid numeric stability evidence"
            );
        }
        Ok(())
    }
}

impl Clusters {
    fn validate(&self, stability: &BTreeMap<String, Stability>) -> Result<()> {
        let mut seen = BTreeSet::new();
        let mut components: Vec<BTreeSet<&str>> = Vec::new();
        for edge in &self.edges {
            ensure!(
                edge.left < edge.right
                    && seen.insert((&edge.left, &edge.right))
                    && edge.pairs >= MINIMUM_CORRELATION_PAIRS
                    && edge.coefficient.abs() >= MINIMUM_CORRELATION
                    && edge.coefficient.abs() <= 1.0,
                "invalid collinearity edge"
            );
            for id in [&edge.left, &edge.right] {
                ensure!(
                    stability.get(id).is_some_and(
                        |stats| stats.levels.is_none() && edge.pairs <= stats.usable_count
                    ),
                    "invalid collinearity edge candidate or pair count"
                );
            }
            let mut joined = BTreeSet::from([edge.left.as_str(), edge.right.as_str()]);
            components.retain(|component| {
                let disjoint = component.is_disjoint(&joined);
                if !disjoint {
                    joined.extend(component);
                }
                disjoint
            });
            components.push(joined);
        }
        components.sort();
        ensure!(
            components
                == self
                    .components
                    .iter()
                    .map(|component| component.iter().map(String::as_str).collect())
                    .collect::<Vec<BTreeSet<_>>>()
                && self
                    .components
                    .iter()
                    .all(|component| component.windows(2).all(|pair| pair[0] < pair[1])),
            "collinearity components do not match their edges"
        );
        Ok(())
    }
}

pub(super) fn facts_text(facts: &Value) -> String {
    let Some(folds) = facts["folds"].as_array() else {
        return "Runtime evidence unavailable.\n".to_owned();
    };
    let protocol = &facts["protocol"];
    let mut text = format!(
        "Development folds ({}, {}, {}):\n",
        facts["adapter_mode"].as_str().unwrap_or_default(),
        protocol["mode"].as_str().unwrap_or_default(),
        protocol["metric"].as_str().unwrap_or_default(),
    );
    for fold in folds {
        let _ = writeln!(
            text,
            "  Fold {}: {} train, {} validation, baseline {}",
            fold["id"], fold["train_count"], fold["validation_count"], fold["baseline_score"]
        );
        for (id, score) in fold["ablation"].as_object().into_iter().flatten() {
            let _ = writeln!(
                text,
                "    {id}: score {}, lift {}, stability {}",
                score["score"], score["improvement"], fold["stability"][id]
            );
            if let Some(unseen) = fold["unseen"].get(id) {
                let _ = writeln!(
                    text,
                    "      unseen {}/{} ({} missing)",
                    unseen["numerator"], unseen["denominator"], unseen["missing"]
                );
            }
        }
        let _ = writeln!(text, "    Collinearity: {}", fold["clusters"]);
    }
    text
}
