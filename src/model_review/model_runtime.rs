use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result, ensure};
use ast_grep_language::SupportLang;
use serde::Serialize;
use serde_json::{Value, json};

use crate::catalog::Filters;
use crate::ledger::adapter_runner::invoke_adapter;
use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, AdapterPolicy, FindingPlace, LedgerEntry, LedgerFinding, SurveyOutput, Tree,
};
use crate::source;
use crate::syntax;
use crate::workspace::Workspace;

use super::{Kind, ModelCatalog, model_static};

const MODEL_FACTS_RUNNER: &str = include_str!("../../py/model_facts.py");

#[derive(Debug, Serialize)]
pub(super) struct Model {
    pub(super) id: String,
    pub(super) file: String,
    pub(super) binding: String,
    lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub(super) struct ModelRead {
    pub(super) models: Vec<Model>,
    pub(super) facts: Value,
    pub(super) ambiguities: Vec<String>,
    files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'static str>,
}

fn model_id(file: &str, binding: &str) -> String {
    ledger::finding_id("model", &format!("{file}\n{binding}"), 1)
}

type ModelKey = (String, String);
type Candidate = (LedgerFinding<Kind>, Option<String>, Option<String>);

#[derive(Default)]
struct StaticInventory {
    owners: BTreeSet<String>,
    bindings: BTreeMap<ModelKey, Vec<usize>>,
    candidates: Vec<Candidate>,
}

pub(super) fn survey_models(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
    adapter: Option<&str>,
    policy: AdapterPolicy,
) -> Result<SurveyOutput<Kind, ModelRead>> {
    let catalog = ModelCatalog::load()?;
    let listing = Listing::of(paths, tree, top, commit)?;
    let cwd = std::env::current_dir()?;
    let root = tree.top.unwrap_or(&cwd);
    let workspace = Workspace::open(
        root.to_path_buf(),
        None,
        &Filters {
            rules: catalog.rules.keys().cloned().collect(),
            severity: None,
        },
    )?;
    let StaticInventory {
        owners,
        mut bindings,
        candidates,
    } = scan_model_files(&listing, tree, root, &workspace, &catalog.rules);
    let ambiguous: Vec<_> = bindings
        .iter()
        .filter(|(_, lines)| lines.len() > 1)
        .map(|(key, lines)| (key.clone(), lines.len()))
        .collect();
    bindings.retain(|_, lines| lines.len() == 1);
    let work_dir = if policy == AdapterPolicy::Required || tree.shown_from.is_some() {
        root
    } else {
        &cwd
    };
    let facts = if let Some(adapter) = adapter {
        match invoke_adapter(adapter, work_dir, MODEL_FACTS_RUNNER)
            .and_then(|document| validate_model_facts(document, &owners))
        {
            Ok((facts, keys)) => {
                for key in keys {
                    bindings.entry(key).or_default();
                }
                facts
            }
            Err(error) if policy == AdapterPolicy::BestEffort => {
                json!({"error": format!("{error:#}")})
            }
            Err(error) => return Err(error.context("model-review adapter failed")),
        }
    } else {
        Value::Null
    };
    let ambiguities: Vec<_> = ambiguous.into_iter().filter_map(|((file, binding), count)| {
        let prefix = format!("{binding}.");
        let replacements = bindings.iter().filter(|((owner, name), lines)| {
            owner == &file && lines.is_empty() && name.starts_with(&prefix)
        }).count();
        (replacements < count).then(|| format!(
            "Ambiguous model {file} {binding}. Assign distinct bindings or register distinct {binding}.* adapter entries."
        ))
    }).collect();
    if policy == AdapterPolicy::Required {
        ensure!(ambiguities.is_empty(), "{}", ambiguities.join("\n"));
    }
    let models: Vec<_> = bindings
        .iter()
        .map(|((file, binding), lines)| Model {
            id: model_id(file, binding),
            file: file.clone(),
            binding: binding.clone(),
            lines: lines.clone(),
        })
        .collect();
    let findings = name_model_findings(candidates, &bindings);
    let checklist = catalog.checklist.checklist(
        models
            .iter()
            .map(|model| (model.id.clone(), model.file.clone()))
            .collect(),
        "model",
    );
    Ok(SurveyOutput {
        read: ModelRead {
            message: models
                .is_empty()
                .then_some("No models found in the selected Python files."),
            models,
            facts,
            ambiguities,
            files: owners.len(),
        },
        findings,
        checklist,
    })
}

fn name_model_findings(
    candidates: Vec<Candidate>,
    bindings: &BTreeMap<ModelKey, Vec<usize>>,
) -> Vec<LedgerEntry<Kind>> {
    let by_key: BTreeMap<_, _> = candidates
        .iter()
        .map(|(finding, rule, binding)| {
            let model = binding.as_ref().and_then(|binding| {
                let file = &finding.places.first()?.path;
                bindings
                    .contains_key(&(file.clone(), binding.clone()))
                    .then(|| model_id(file, binding))
            });
            (
                (finding.key.clone(), finding.places.clone()),
                (rule.clone(), model),
            )
        })
        .collect();
    ledger::named(
        candidates
            .into_iter()
            .map(|(finding, _, _)| finding)
            .collect(),
    )
    .into_iter()
    .map(|mut entry| {
        if let Some((rule, model)) =
            by_key.get(&(entry.finding.key.clone(), entry.finding.places.clone()))
        {
            entry.source_rule.clone_from(rule);
            entry.model.clone_from(model);
        }
        entry
    })
    .collect()
}

fn scan_model_files(
    listing: &Listing,
    tree: Tree<'_>,
    root: &Path,
    workspace: &Workspace,
    rules: &BTreeMap<String, Kind>,
) -> StaticInventory {
    let mut inventory = StaticInventory::default();
    for (path, lang) in &listing.files {
        if *lang != SupportLang::Python {
            continue;
        }
        let (Some(text), Some(absolute)) = (listing.text(path), listing.absolute(path)) else {
            continue;
        };
        let owner = tree
            .tracked(&absolute)
            .or_else(|| {
                absolute
                    .strip_prefix(root)
                    .ok()
                    .map(|file| file.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| tree.shown(path));
        inventory.owners.insert(owner.clone());
        let parsed = syntax::parse(&text);
        let static_review = model_static::inspect_models(&parsed.root(), &owner);
        inventory.bindings.extend(
            static_review
                .bindings
                .into_iter()
                .map(|(binding, lines)| ((owner.clone(), binding), lines)),
        );
        let linted = workspace.analyze(
            source::File {
                path: Path::new(&owner),
                disk: Some(&absolute),
            },
            &text,
        );
        let mut pack_sites = BTreeSet::new();
        for found in linted.found {
            let issue = found.into_issue();
            let rule = issue.id.to_string();
            let Some(&kind) = rules.get(&rule) else {
                continue;
            };
            let mut place = FindingPlace::file(&owner);
            let mut binding = None;
            if let Some(at) = issue.position {
                place.line = Some(at.line);
                place.column = Some(at.column);
                binding = model_static::model_binding_at(&parsed.root(), at.line, at.column);
            }
            let key = format!("{owner}\n{rule}\n{}", issue.message);
            if pack_sites.insert((kind, place.clone(), key.clone())) {
                inventory.candidates.push((
                    LedgerFinding {
                        kind,
                        key,
                        places: vec![place],
                        evidence: issue.message,
                        weight: 2,
                    },
                    Some(rule),
                    binding,
                ));
            }
        }
        for (finding, binding) in static_review.findings {
            if !pack_sites
                .iter()
                .any(|(kind, place, _)| *kind == finding.kind && finding.places.contains(place))
            {
                inventory.candidates.push((finding, None, binding));
            }
        }
    }
    inventory
}

type ModelKeys = BTreeSet<(String, String)>;

fn valid_binding(binding: &str) -> bool {
    binding.split('.').all(|part| {
        let mut chars = part.chars();
        chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
            && chars.all(|c| c == '_' || c.is_alphanumeric())
    })
}

fn validate_model_facts(
    mut document: Value,
    owners: &BTreeSet<String>,
) -> Result<(Value, ModelKeys)> {
    let models = document
        .get_mut("models")
        .and_then(Value::as_array_mut)
        .context("adapter must return a models array")?;
    ensure!(!models.is_empty(), "adapter returned no model facts");
    let mut keys = BTreeSet::new();
    for model in models {
        let file = model["file"]
            .as_str()
            .context("adapter model has no owner file")?;
        let binding = model["binding"]
            .as_str()
            .filter(|binding| valid_binding(binding))
            .context("adapter model needs a qualified Python binding")?;
        ensure!(
            Path::new(file)
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
                && owners.contains(file),
            "adapter owner {file} is outside the selected Python files"
        );
        ensure!(
            keys.insert((file.to_owned(), binding.to_owned())),
            "duplicate adapter model {file} {binding}"
        );
        let id = model_id(file, binding);
        validate_model_measurements(model)?;
        model["id"] = Value::String(id);
    }
    Ok((document, keys))
}

fn validate_model_measurements(model: &Value) -> Result<()> {
    for field in ["scorer", "splitter", "baseline"] {
        ensure!(
            model[field]
                .as_str()
                .is_some_and(|value| !value.trim().is_empty()),
            "adapter model needs a nonempty {field}"
        );
    }
    let folds = model["folds"]
        .as_array()
        .context("adapter model has no folds array")?;
    ensure!(!folds.is_empty(), "adapter model has no measured folds");
    let mut ids = BTreeSet::new();
    for fold in folds {
        let id = fold["id"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
            .context("adapter fold has no ID")?;
        ensure!(ids.insert(id), "duplicate adapter fold {id}");
        validate_model_partitions(fold, id)?;
        for score in [
            "train_score",
            "validation_score",
            "baseline_score",
            "validation_minus_baseline",
            "train_minus_validation",
        ] {
            ensure!(
                fold[score].as_f64().is_some_and(f64::is_finite),
                "adapter fold {id} has an invalid {score}"
            );
        }
    }
    ensure!(
        model["spread"]["mean"].as_f64().is_some_and(f64::is_finite),
        "adapter model has an invalid validation mean"
    );
    validate_probability_summary(&model["probability_calibration"])?;
    let coverage = &model["interval_coverage"];
    if measured_status(coverage)? {
        let levels = coverage["levels"]
            .as_array()
            .context("adapter interval report needs confidence levels")?;
        validate_interval_levels(levels)?;
        for fold in folds {
            let fold_levels = fold["interval_coverage"]
                .as_array()
                .context("adapter fold has no interval report")?;
            validate_interval_levels(fold_levels)?;
            ensure!(
                fold_levels.len() == levels.len()
                    && fold_levels.iter().zip(levels).all(
                        |(fold, pooled)| fold["confidence_level"] == pooled["confidence_level"]
                    ),
                "adapter interval levels differ across the folds"
            );
        }
    }
    Ok(())
}

fn validate_model_partitions(fold: &Value, id: &str) -> Result<()> {
    let partitions = &fold["partitions"];
    for role in ["fit", "early_stopping", "calibration", "validation"] {
        let count = partitions["counts"][role]
            .as_u64()
            .with_context(|| format!("adapter fold {id} is missing a {role} count"))?;
        ensure!(
            count > 0 || matches!(role, "early_stopping" | "calibration"),
            "adapter fold {id} needs a positive {role} count"
        );
    }
    for (role, field) in [("groups", "intersections"), ("times", "boundaries")] {
        let metadata = &partitions[role];
        ensure!(
            !measured_status(metadata)? || metadata[field].is_object(),
            "adapter fold {id} has invalid {role} metadata"
        );
    }
    let conflicts = partitions["conflicts"]
        .as_array()
        .context("adapter fold needs a conflicts array")?;
    ensure!(
        fold["valid"].as_bool() == Some(conflicts.is_empty()),
        "adapter fold {id} validity disagrees with partition conflicts"
    );
    Ok(())
}

fn measured_status(value: &Value) -> Result<bool> {
    Ok(match value["status"].as_str() {
        Some("measured") => true,
        Some("unavailable")
            if value["reason"]
                .as_str()
                .is_some_and(|reason| !reason.trim().is_empty()) =>
        {
            false
        }
        _ => anyhow::bail!("adapter evidence needs a measured status or an unavailable reason"),
    })
}

fn unit_rate(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|rate| (0.0..=1.0).contains(&rate))
}

fn validate_probability_summary(summary: &Value) -> Result<()> {
    if !measured_status(summary)? {
        return Ok(());
    }
    let bins = summary["bins"]
        .as_array()
        .context("adapter probability report needs reliability bins")?;
    ensure!(
        bins.len() == 5 && unit_rate(&summary["brier_loss"]),
        "adapter probability report needs five reliability bins and a Brier loss in [0, 1]"
    );
    let count = summary["count"]
        .as_u64()
        .filter(|count| *count > 0)
        .context("adapter probability report has no validation predictions")?;
    let mut bin_total = 0_u64;
    for bin in bins {
        let support = bin["count"]
            .as_u64()
            .context("adapter reliability bin has no count")?;
        bin_total = bin_total.saturating_add(support);
        ensure!(
            support == 0
                || (unit_rate(&bin["mean_probability"]) && unit_rate(&bin["observed_rate"])),
            "adapter reliability bin probabilities and observed rates must be in [0, 1]"
        );
    }
    ensure!(
        count == bin_total,
        "adapter reliability bin counts disagree with validation prediction count"
    );
    Ok(())
}

fn validate_interval_levels(levels: &[Value]) -> Result<()> {
    ensure!(
        !levels.is_empty(),
        "adapter interval report has no confidence levels"
    );
    let mut seen = BTreeSet::new();
    for evidence in levels {
        let level = evidence["confidence_level"]
            .as_f64()
            .filter(|level| *level > 0.0 && *level < 1.0)
            .context("adapter confidence levels must be in (0, 1)")?;
        let eligible = evidence["eligible"]
            .as_u64()
            .filter(|count| *count > 0)
            .context("adapter interval level has no eligible rows")?;
        ensure!(
            seen.insert(level.to_bits())
                && evidence["covered"]
                    .as_u64()
                    .is_some_and(|covered| covered <= eligible)
                && unit_rate(&evidence["coverage"])
                && evidence["coverage_minus_target"]
                    .as_f64()
                    .is_some_and(|gap| (-1.0..=1.0).contains(&gap))
                && (evidence["mean_width"].is_null()
                    || evidence["mean_width"]
                        .as_f64()
                        .is_some_and(|width| width >= 0.0 && width.is_finite())),
            "adapter interval level has invalid counts, coverage or width"
        );
    }
    Ok(())
}
