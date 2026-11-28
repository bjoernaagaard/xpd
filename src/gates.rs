//! Pipeline gate verification: validates a saved model contract and judges the evidence from
//! `profile`, `check`, `graph`, `equiv`, and `refit` inside a durable Duroxide workflow.

mod workflow;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::catalog::Catalog;
use crate::output::Format;

const PIPELINE_CLASS: &str = "sklearn.pipeline.Pipeline";
const REQUIRED_VERSIONS: [&str; 3] = ["python", "scikit-learn", "pandas"];
const MIN_FOLDS: usize = 2;
const NO_SOURCE: &str = "which is no input column and which no earlier step adds.";
const SEED_LIBRARIES: &str = "scikit-learn, scikit-lego, MAPIE, Category Encoders, arch or pymoo";

/// Every rule that flags an unseeded draw across the supported ML and optimization packs.
pub(crate) const SEED_RULES: [(&str, &str); 8] = [
    ("sklearn", "missing-random-state"),
    ("sklearn", "random-state-without-shuffle"),
    ("sklego", "unseeded-random-estimator"),
    ("mapie", "unseeded-resampling"),
    ("category_encoders", "encoder-noise-without-seed"),
    ("arch", "arch-unseeded-draws"),
    ("pymoo", "unseeded-run"),
    ("pymoo", "numpy-seed-ignored"),
];

/// Durable verification of a saved scikit-learn pipeline run from raw rows to predictions.
#[derive(Debug, Subcommand)]
pub(crate) enum GatesCommand {
    /// Start a durable gates run and judge the pipeline contract and scripts.
    Start {
        /// The contract JSON that the training script saved beside its model.
        #[arg(long)]
        contract: PathBuf,
        /// The training script, which leaves its held-out output in `result`.
        #[arg(long)]
        train: PathBuf,
        /// The prediction script, which loads the saved model and leaves the same `result`.
        #[arg(long)]
        predict: PathBuf,
        /// The file with the feature steps.
        #[arg(long)]
        features: PathBuf,
        /// Optional discovery proof JSON file, or `-` to read it from stdin.
        #[arg(long)]
        discovery: Option<PathBuf>,
        /// Output format. Default: text on a terminal, JSON otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// Run another verification round on an existing run, reusing cached results for unchanged inputs.
    Retry {
        /// The run UUID returned by `xpd gates start`.
        run: Uuid,
        /// Optional discovery proof JSON file, or `-` to read it from stdin.
        #[arg(long)]
        discovery: Option<PathBuf>,
        /// Force every activity to run again instead of reusing cached results.
        #[arg(long)]
        no_cache: bool,
        /// Output format. Default: text on a terminal, JSON otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// Show the current stage, cached activities, round history, and latest gate verdicts.
    Status {
        /// The run UUID returned by `xpd gates start`.
        run: Uuid,
        /// Output format. Default: text on a terminal, JSON otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// Recover an interrupted round or retry an errored round without discarding finished activities.
    Resume {
        /// The run UUID returned by `xpd gates start`.
        run: Uuid,
        /// Output format. Default: text on a terminal, JSON otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
}

impl GatesCommand {
    pub(crate) fn run(self, default_format: Format) -> Result<ExitCode> {
        workflow::run(self, default_format)
    }
}

/// Ensures every rule named in `SEED_RULES` still exists in `catalog`.
pub(crate) fn validate_seed_rules(catalog: &Catalog) -> Result<()> {
    let entries = catalog.entries();
    let known: BTreeSet<(&str, &str)> = entries
        .iter()
        .map(|entry| (entry.id.package(), entry.id.name()))
        .collect();
    for (package, rule) in SEED_RULES {
        ensure!(
            known.contains(&(package, rule)),
            "xpd gates seed rule {package}/{rule} is missing from the rule catalog"
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiscoveryQuestion {
    pub(crate) task: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiscoveryEntryRef {
    pub(crate) kind: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiscoveryCandidates {
    #[serde(default)]
    pub(crate) symbols: Vec<DiscoveryEntryRef>,
    #[serde(default)]
    pub(crate) rules: Vec<DiscoveryEntryRef>,
    #[serde(default)]
    pub(crate) recipes: Vec<DiscoveryEntryRef>,
}

impl DiscoveryCandidates {
    fn pairs(&self) -> Vec<(String, String)> {
        self.symbols
            .iter()
            .chain(&self.rules)
            .chain(&self.recipes)
            .map(|entry| (entry.kind.clone(), entry.name.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiscoveryChoice {
    pub(crate) name: String,
    pub(crate) reason: String,
    pub(crate) symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Discovery {
    pub(crate) question: DiscoveryQuestion,
    pub(crate) candidates: DiscoveryCandidates,
    pub(crate) choice: DiscoveryChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveryDecision {
    question: DiscoveryQuestion,
    candidates: Vec<(String, String)>,
    choice: DiscoveryChoice,
}

impl Discovery {
    fn decision(&self) -> DiscoveryDecision {
        DiscoveryDecision {
            question: self.question.clone(),
            candidates: self.candidates.pairs(),
            choice: self.choice.clone(),
        }
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            !self.question.task.is_empty(),
            "discovery.question.task must not be empty"
        );
        ensure!(
            !self.choice.name.is_empty(),
            "discovery.choice.name must not be empty"
        );
        ensure!(
            !self.choice.reason.is_empty(),
            "discovery.choice.reason must not be empty"
        );
        ensure!(
            !self.choice.symbols.is_empty(),
            "discovery.choice.symbols must not be empty"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiscoveryProof {
    pub(crate) discovery: Discovery,
    pub(crate) paths: Vec<String>,
    pub(crate) gate: Gate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawData {
    pub(crate) file: String,
    pub(crate) target: String,
    pub(crate) rows: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Split {
    pub(crate) seed: i64,
    pub(crate) test_size: f64,
    pub(crate) stratified: bool,
    pub(crate) train_rows: usize,
    pub(crate) test_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InputColumn {
    pub(crate) column: String,
    pub(crate) dtype: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FittedStep {
    pub(crate) name: String,
    pub(crate) estimator: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Splitter {
    #[serde(rename = "class")]
    pub(crate) class_name: String,
    pub(crate) args: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Validation {
    pub(crate) splitter: Splitter,
    pub(crate) scoring: String,
    pub(crate) folds: usize,
    pub(crate) cv_scores: Vec<f64>,
    pub(crate) test_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedModel {
    pub(crate) file: String,
    #[serde(rename = "type")]
    pub(crate) model_type: String,
    pub(crate) versions: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) code: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Contract {
    #[serde(default)]
    pub(crate) discovery: Option<Discovery>,
    pub(crate) data: RawData,
    pub(crate) split: Split,
    pub(crate) inputs: Vec<InputColumn>,
    pub(crate) steps: Vec<FittedStep>,
    pub(crate) validation: Validation,
    pub(crate) model: SavedModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ColumnProfile {
    pub(crate) column: String,
    pub(crate) dtype: String,
    pub(crate) nulls: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub(crate) file: String,
    pub(crate) pandas: String,
    pub(crate) rows: usize,
    pub(crate) columns: Vec<ColumnProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CheckIssue {
    pub(crate) package: String,
    pub(crate) rule: String,
    pub(crate) file: String,
    pub(crate) line: Option<usize>,
    pub(crate) severity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CheckReport {
    pub(crate) issues: Vec<CheckIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GraphStep {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) reads: Vec<String>,
    pub(crate) columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GraphPipeline {
    pub(crate) file: String,
    pub(crate) steps: Vec<GraphStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GraphProblem {
    pub(crate) file: String,
    pub(crate) line: usize,
    pub(crate) severity: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GraphReport {
    #[serde(default)]
    pub(crate) pipelines: Vec<GraphPipeline>,
    #[serde(default)]
    pub(crate) problems: Vec<GraphProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Equivalence {
    pub(crate) equivalent: bool,
    #[serde(default)]
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Refit {
    pub(crate) rows: usize,
    pub(crate) train_rows: usize,
    pub(crate) test_rows: usize,
    pub(crate) output: String,
    pub(crate) largest_gap: Option<f64>,
    pub(crate) model_matches: bool,
    #[serde(rename = "refit_test_score")]
    pub(crate) test_score: f64,
    #[serde(rename = "refit_cv_scores")]
    pub(crate) cv_scores: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Gate {
    pub(crate) name: String,
    pub(crate) stage: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) passed: bool,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GateReport {
    pub(crate) passed: bool,
    pub(crate) failed: Vec<String>,
    pub(crate) gates: Vec<Gate>,
}

pub(crate) struct Proof<'a> {
    pub(crate) discovery: Option<&'a DiscoveryProof>,
    pub(crate) lint: &'a CheckReport,
    pub(crate) dataflow: &'a GraphReport,
    pub(crate) reproducible: &'a Equivalence,
    pub(crate) refit: &'a Refit,
    pub(crate) served: &'a Equivalence,
}

/// Resolves `raw` relative to `root` and refuses any path outside `root` or missing on disk.
pub(crate) fn inside_file(root: &Path, raw: &Path) -> Result<String> {
    if raw
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        bail!("{} escapes the project root", raw.display());
    }
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        root.join(raw)
    };
    let canonical = fs::canonicalize(&candidate)
        .with_context(|| format!("{} is not a readable file in the project", raw.display()))?;
    ensure!(
        canonical.starts_with(root),
        "{} is outside the project root",
        raw.display()
    );
    ensure!(
        canonical.is_file(),
        "{} is not a file in the project",
        raw.display()
    );
    let relative = canonical
        .strip_prefix(root)
        .with_context(|| format!("{} is outside the project root", raw.display()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

/// Reads and validates a pipeline contract JSON file inside `root`, normalizing its paths.
pub(crate) fn read_contract(root: &Path, file: &str) -> Result<Contract> {
    let text = fs::read_to_string(root.join(file))
        .with_context(|| format!("cannot read contract {file}"))?;
    let mut contract: Contract =
        serde_json::from_str(&text).with_context(|| format!("{file} is not a valid contract"))?;
    ensure!(
        contract.split.test_size > 0.0 && contract.split.test_size < 1.0,
        "{file} split.test_size must be between 0 and 1"
    );
    ensure!(
        !contract.validation.splitter.class_name.is_empty(),
        "{file} validation.splitter.class must not be empty"
    );
    if let Some(discovery) = &contract.discovery {
        discovery
            .validate()
            .with_context(|| format!("{file} has an invalid discovery block"))?;
    }
    contract.data.file = inside_file(root, Path::new(&contract.data.file))
        .with_context(|| format!("{file} names data file {}", contract.data.file))?;
    contract.model.file = inside_file(root, Path::new(&contract.model.file))
        .with_context(|| format!("{file} names model file {}", contract.model.file))?;
    let mut normalized_code = Vec::with_capacity(contract.model.code.len());
    for code in &contract.model.code {
        normalized_code.push(
            inside_file(root, Path::new(code))
                .with_context(|| format!("{file} names code file {code}"))?,
        );
    }
    contract.model.code = normalized_code;
    Ok(contract)
}

/// Reads an optional `DiscoveryProof` JSON file, or stdin when the path is `-`.
pub(crate) fn read_discovery(path: Option<&Path>) -> Result<Option<DiscoveryProof>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = if path == Path::new("-") {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("cannot read discovery proof from stdin")?;
        buf
    } else {
        fs::read_to_string(path)
            .with_context(|| format!("cannot read discovery proof {}", path.display()))?
    };
    let proof: DiscoveryProof = serde_json::from_str(&text)
        .with_context(|| format!("cannot parse discovery proof {}", path.display()))?;
    proof.discovery.validate()?;
    Ok(Some(proof))
}

fn gate(name: &str, stage: &str, evidence: &[&str], failures: &[String], found: String) -> Gate {
    Gate {
        name: name.to_owned(),
        stage: stage.to_owned(),
        evidence: evidence.iter().map(|item| (*item).to_owned()).collect(),
        passed: failures.is_empty(),
        detail: if failures.is_empty() {
            found
        } else {
            failures.join(" ")
        },
    }
}

fn data_is_profiled(profile: &Profile, contract: &Contract) -> Gate {
    let data = &contract.data;
    let mut failures = Vec::new();
    if data.file != profile.file {
        failures.push(format!(
            "The contract names {}, and the profile is of {}.",
            data.file, profile.file
        ));
    }
    if data.rows != profile.rows {
        failures.push(format!(
            "The contract counts {} rows, and the file has {}.",
            data.rows, profile.rows
        ));
    }
    match profile
        .columns
        .iter()
        .find(|column| column.column == data.target)
    {
        None => failures.push(format!("{} has no column {}.", profile.file, data.target)),
        Some(target) if target.nulls > 0 => failures.push(format!(
            "The target {} has {} nulls.",
            data.target, target.nulls
        )),
        Some(_) => {}
    }
    let found = format!(
        "{} has {} rows, and the target {} has no nulls.",
        data.file, data.rows, data.target
    );
    gate(
        "data-is-profiled",
        "raw data",
        &["profile", "contract"],
        &failures,
        found,
    )
}

fn split_covers_every_row(profile: &Profile, contract: &Contract) -> Gate {
    let split = &contract.split;
    let mut failures = Vec::new();
    if split.train_rows.min(split.test_rows) < 1 {
        failures.push("The split leaves no rows on one side.".to_owned());
    }
    if split.train_rows + split.test_rows != profile.rows {
        failures.push(format!(
            "{} training rows and {} held out rows are not the {} rows of the file.",
            split.train_rows, split.test_rows, profile.rows
        ));
    }
    let found = format!(
        "The contract counts {} training rows and {} held out rows.",
        split.train_rows, split.test_rows
    );
    gate(
        "split-covers-every-row",
        "split",
        &["profile", "contract"],
        &failures,
        found,
    )
}

fn split_is_seeded(lint: &CheckReport, contract: &Contract) -> Gate {
    let failures: Vec<String> = lint
        .issues
        .iter()
        .filter(|issue| {
            SEED_RULES
                .iter()
                .any(|&(pkg, rule)| issue.package == pkg && issue.rule == rule)
        })
        .map(|issue| {
            let lineno = issue.line.map_or_else(String::new, |n| n.to_string());
            format!("{}:{lineno} {}/{}.", issue.file, issue.package, issue.rule)
        })
        .collect();
    let found = format!(
        "The contract records the seed {}, and check reports no unseeded {SEED_LIBRARIES} draw.",
        contract.split.seed
    );
    gate(
        "split-is-seeded",
        "split",
        &["check", "contract"],
        &failures,
        found,
    )
}

fn check_reports_no_error(lint: &CheckReport) -> Gate {
    let failures: Vec<String> = lint
        .issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .map(|issue| {
            let lineno = issue.line.map_or_else(String::new, |n| n.to_string());
            format!("{}:{lineno} {}/{}.", issue.file, issue.package, issue.rule)
        })
        .collect();
    gate(
        "check-reports-no-error",
        "proof",
        &["check"],
        &failures,
        "check reports no error.".to_owned(),
    )
}

fn inputs_are_file_columns(profile: &Profile, contract: &Contract) -> Gate {
    let raw: BTreeMap<&str, &str> = profile
        .columns
        .iter()
        .map(|column| (column.column.as_str(), column.dtype.as_str()))
        .collect();
    let mut failures = Vec::new();
    for column in &contract.inputs {
        if column.column == contract.data.target {
            failures.push(format!("The target {} is an input.", column.column));
        } else if let Some(&dtype) = raw.get(column.column.as_str()) {
            if dtype != column.dtype {
                failures.push(format!(
                    "The input {} is {}, and the file holds {dtype}.",
                    column.column, column.dtype
                ));
            }
        } else {
            failures.push(format!(
                "The input {} is no column of {}, so code outside the Pipeline made it.",
                column.column, profile.file
            ));
        }
    }
    if contract.inputs.is_empty() {
        failures.push("The contract lists no input column.".to_owned());
    }
    let found = format!(
        "The {} inputs of the contract have the names and dtypes of columns of {}.",
        contract.inputs.len(),
        profile.file
    );
    gate(
        "inputs-are-file-columns",
        "preprocessing",
        &["profile", "contract"],
        &failures,
        found,
    )
}

fn one_pipeline(contract: &Contract) -> Gate {
    let mut failures = Vec::new();
    if contract.model.model_type != PIPELINE_CLASS {
        failures.push(format!(
            "The contract names the saved object as {}, not {PIPELINE_CLASS}.",
            contract.model.model_type
        ));
    }
    if contract.steps.is_empty() {
        failures.push("The contract lists no step.".to_owned());
    }
    let steps: Vec<&str> = contract
        .steps
        .iter()
        .map(|step| step.estimator.as_str())
        .collect();
    let found = format!(
        "The contract lists one Pipeline of {}.",
        steps.join(" then ")
    );
    gate("one-pipeline", "model", &["contract"], &failures, found)
}

fn dataflow_connects(dataflow: &GraphReport, contract: &Contract) -> Gate {
    let mut failures: Vec<String> = dataflow
        .problems
        .iter()
        .filter(|problem| matches!(problem.severity.as_str(), "error" | "warning"))
        .map(|problem| format!("{}:{} {}", problem.file, problem.line, problem.message))
        .collect();
    let inputs: BTreeSet<&str> = contract
        .inputs
        .iter()
        .map(|column| column.column.as_str())
        .collect();
    let mut step_count = 0usize;
    for pipeline in &dataflow.pipelines {
        let mut known = inputs.clone();
        for step in &pipeline.steps {
            step_count += 1;
            let place = format!("{}:{} {}()", pipeline.file, step.line, step.name);
            for column in &step.reads {
                if !known.contains(column.as_str()) {
                    failures.push(format!("{place} reads {column}, {NO_SOURCE}"));
                }
            }
            for column in &step.columns {
                known.insert(column.as_str());
            }
        }
    }
    let found =
        format!("{step_count} feature steps read only input columns and columns of earlier steps.");
    gate(
        "dataflow-connects",
        "preprocessing",
        &["graph", "contract"],
        &failures,
        found,
    )
}

fn cross_validated(contract: &Contract) -> Gate {
    let validation = &contract.validation;
    let mut failures = Vec::new();
    if validation.folds < MIN_FOLDS {
        failures.push(format!(
            "{} folds is no cross-validation.",
            validation.folds
        ));
    }
    if validation.cv_scores.len() != validation.folds {
        failures.push(format!(
            "The contract has {} scores for {} folds.",
            validation.cv_scores.len(),
            validation.folds
        ));
    }
    let low = validation
        .cv_scores
        .iter()
        .copied()
        .reduce(f64::min)
        .unwrap_or(0.0);
    let high = validation
        .cv_scores
        .iter()
        .copied()
        .reduce(f64::max)
        .unwrap_or(0.0);
    let found = format!(
        "The contract records {} {low} to {high} on {} folds, and {} as the held out score.",
        validation.scoring, validation.folds, validation.test_score
    );
    gate(
        "cross-validated",
        "validation",
        &["contract"],
        &failures,
        found,
    )
}

fn equivalent(name: &str, stage: &str, result: &Equivalence, found: &str) -> Gate {
    let failures = if result.equivalent {
        Vec::new()
    } else {
        vec![
            result
                .detail
                .clone()
                .unwrap_or_else(|| "The results differ.".to_owned()),
        ]
    };
    gate(name, stage, &["equiv"], &failures, found.to_owned())
}

fn versions_recorded(profile: &Profile, contract: &Contract) -> Gate {
    let versions = &contract.model.versions;
    let mut failures: Vec<String> = REQUIRED_VERSIONS
        .iter()
        .filter(|&&name| versions.get(name).is_none_or(String::is_empty))
        .map(|name| format!("The contract records no version of {name}."))
        .collect();
    if let Some(pandas) = versions.get("pandas")
        && pandas != &profile.pandas
    {
        failures.push(format!(
            "The model was written with pandas {pandas}, and the project's Python has {}.",
            profile.pandas
        ));
    }
    let recorded: Vec<String> = versions
        .iter()
        .map(|(name, version)| format!("{name} {version}"))
        .collect();
    let found = format!("The contract records {}.", recorded.join(", "));
    gate(
        "versions-recorded",
        "persisted model",
        &["profile", "contract"],
        &failures,
        found,
    )
}

fn refit_matches_model(result: &Refit) -> Gate {
    let mut failures = Vec::new();
    if !result.model_matches {
        let gap = result
            .largest_gap
            .map_or_else(|| "None".to_owned(), |value| value.to_string());
        failures.push(format!(
            "The raw training refit differs on {}. The largest gap is {gap}.",
            result.output
        ));
    }
    let found = format!(
        "The raw training refit matches the saved model on {}.",
        result.output
    );
    gate("refit-matches-model", "proof", &["refit"], &failures, found)
}

fn refit_matches_scores(contract: &Contract, result: &Refit) -> Gate {
    let mut failures = Vec::new();
    if contract.validation.test_score.to_bits() != result.test_score.to_bits() {
        failures.push(format!(
            "The contract held out score is {}, and the refit gives {}.",
            contract.validation.test_score, result.test_score
        ));
    }
    let scores_equal = contract.validation.cv_scores.len() == result.cv_scores.len()
        && contract
            .validation
            .cv_scores
            .iter()
            .zip(&result.cv_scores)
            .all(|(left, right)| left.to_bits() == right.to_bits());
    if !scores_equal {
        failures.push(format!(
            "The contract fold scores are {:?}, and the refit gives {:?}.",
            contract.validation.cv_scores, result.cv_scores
        ));
    }
    if result.rows != contract.data.rows
        || result.train_rows != contract.split.train_rows
        || result.test_rows != contract.split.test_rows
        || result.cv_scores.len() != contract.validation.folds
    {
        failures.push("The refit row or fold counts differ.".to_owned());
    }
    gate(
        "refit-matches-scores",
        "validation",
        &["refit", "contract"],
        &failures,
        "The contract scores and row and fold counts match the raw training refit.".to_owned(),
    )
}

fn discovery_gates(contract: &Contract, discovery: Option<&DiscoveryProof>) -> Vec<Gate> {
    if contract.discovery.is_none() && discovery.is_none() {
        return Vec::new();
    }
    let mut gates = Vec::new();
    let mut failures = Vec::new();
    match discovery {
        None => failures.push("No discovery_gate result was supplied.".to_owned()),
        Some(proof) => {
            gates.push(proof.gate.clone());
            let recorded = contract.discovery.as_ref().map(Discovery::decision);
            if recorded.as_ref() != Some(&proof.discovery.decision()) {
                failures
                    .push("The contract and proof record different discovery choices.".to_owned());
            }
        }
    }
    gates.push(gate(
        "discovery-is-recorded",
        "discovery",
        &["contract"],
        &failures,
        "The contract records the discovery question, candidates and reason of the proof."
            .to_owned(),
    ));
    gates
}

/// Evaluates every pipeline gate from the collected evidence.
pub(crate) fn judge(profile: &Profile, contract: &Contract, proof: &Proof<'_>) -> GateReport {
    let mut gates = vec![
        data_is_profiled(profile, contract),
        split_covers_every_row(profile, contract),
        split_is_seeded(proof.lint, contract),
        inputs_are_file_columns(profile, contract),
        dataflow_connects(proof.dataflow, contract),
        one_pipeline(contract),
        check_reports_no_error(proof.lint),
        cross_validated(contract),
        equivalent(
            "reproducible",
            "proof",
            proof.reproducible,
            "Two runs of the training script, each in its own Python process, leave an equal `result`.",
        ),
        refit_matches_model(proof.refit),
        refit_matches_scores(contract, proof.refit),
        equivalent(
            "saved-model-predicts",
            "persisted model",
            proof.served,
            "The prediction script, in its own Python process, leaves the `result` that the training script left.",
        ),
        versions_recorded(profile, contract),
    ];
    gates.extend(discovery_gates(contract, proof.discovery));
    let failed: Vec<String> = gates
        .iter()
        .filter(|item| !item.passed)
        .map(|item| item.name.clone())
        .collect();
    GateReport {
        passed: failed.is_empty(),
        failed,
        gates,
    }
}

pub(crate) fn format_gates_text(
    run: &str,
    round: usize,
    stage: &str,
    executed: &[String],
    reused: &[String],
    report: Option<&GateReport>,
    error: Option<&str>,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Gates run {run} (round {round}, {stage})");
    if !executed.is_empty() {
        let _ = writeln!(out, "  Executed: {}", executed.join(", "));
    }
    if !reused.is_empty() {
        let _ = writeln!(out, "  Reused: {}", reused.join(", "));
    }
    if let Some(error) = error {
        let _ = writeln!(out, "  Error: {error}");
    }
    if let Some(report) = report {
        for item in &report.gates {
            let mark = if item.passed { "PASS" } else { "FAIL" };
            let _ = writeln!(
                out,
                "  [{mark}] {} ({}): {}",
                item.name, item.stage, item.detail
            );
        }
    }
    out
}
