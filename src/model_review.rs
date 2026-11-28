mod model_runtime;
mod model_static;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::ledger::{self, Goal, QuestionCatalog, Select, WorkflowId};
use crate::output::Format;

const MODEL_CATALOG: &str = include_str!("../rules/xpd/model-review.yml");

#[derive(Deserialize, Serialize)]
struct ModelCatalog {
    #[serde(flatten)]
    checklist: QuestionCatalog,
    mcp_gates: Vec<String>,
    rules: BTreeMap<String, Kind>,
}

impl ModelCatalog {
    fn load() -> Result<Self> {
        Ok(serde_saphyr::from_str(MODEL_CATALOG)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    Validation,
    Comparison,
    Calibration,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    SplitMismatch,
    TestSetTuning,
    BaselineNotObserved,
    MetricLossMismatch,
    ConformalLeakage,
    ValidationLeakage,
    MetricResponse,
    ValidationReproducibility,
    ConformalConfiguration,
    BoostingValidation,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;
    const WORKFLOW: WorkflowId = WorkflowId::ModelReview;
    const GROUPS: &[Group] = &[Group::Validation, Group::Comparison, Group::Calibration];
    const GOALS: &[Goal] = &[Goal::Design];

    fn group(self) -> Group {
        match self {
            Self::SplitMismatch
            | Self::TestSetTuning
            | Self::ValidationLeakage
            | Self::ValidationReproducibility
            | Self::BoostingValidation => Group::Validation,
            Self::BaselineNotObserved | Self::MetricLossMismatch | Self::MetricResponse => {
                Group::Comparison
            }
            Self::ConformalLeakage | Self::ConformalConfiguration => Group::Calibration,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Validation => "Validation and selection",
            Group::Comparison => "Comparisons and metrics",
            Group::Calibration => "Calibration",
        }
    }
}

/// Review the validation design, comparisons and calibration of named models.
#[derive(Debug, clap::Args)]
pub(crate) struct ModelReviewArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// Project Python adapter or shell command that emits model facts.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Print the evidence catalog without surveying files.
    #[arg(long)]
    questions: bool,
}

impl ModelReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            let catalog = ModelCatalog::load()?;
            return match self.select.format_override().unwrap_or(default) {
                Format::Json => {
                    println!("{}", serde_json::to_string(&catalog)?);
                    Ok(ExitCode::SUCCESS)
                }
                Format::Text => {
                    let _ = catalog.checklist.print(Format::Text)?;
                    println!("MCP gates\n  {}", catalog.mcp_gates.join("\n  "));
                    Ok(ExitCode::SUCCESS)
                }
            };
        }
        self.select.print_with(
            default,
            self.adapter.as_deref(),
            model_runtime::survey_models,
            |ledger| {
                let mut text = format!(
                    "xpd model-review: {}, {} models, {} checklist items\n",
                    ledger.count(),
                    ledger.read.models.len(),
                    ledger.checklist.len(),
                );
                if ledger.read.models.is_empty() {
                    text.push_str("No models found in the selected Python files.\n");
                }
                for ambiguity in &ledger.read.ambiguities {
                    let _ = writeln!(text, "{ambiguity}");
                }
                for model in &ledger.read.models {
                    let _ = writeln!(text, "  {}  {}  {}", model.id, model.file, model.binding);
                }
                if let Some(error) = ledger.read.facts["error"].as_str() {
                    let _ = writeln!(text, "Adapter error: {error}");
                } else if !ledger.read.facts.is_null() {
                    let _ = writeln!(
                        text,
                        "Model facts:\n{}",
                        serde_json::to_string_pretty(&ledger.read.facts).unwrap_or_default()
                    );
                }
                text
            },
        )
    }
}

pub(crate) use model_static::early_stop_test_pair;
