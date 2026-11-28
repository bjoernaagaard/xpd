mod column_adapter;
mod survey;

use std::process::ExitCode;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use crate::ledger::{self, LedgerKind as _, QuestionCatalog, Select, WorkflowId};
use crate::output::Format;

#[cfg(feature = "ledger-distributed")]
pub(crate) use column_adapter::adapter_command;
pub(crate) use column_adapter::record_target;

const CATALOG: &str = include_str!("../rules/xpd/data-prep.yml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    Leakage,
    Joins,
    Target,
    Coercion,
    PipelineShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    FitBeforeSplit,
    FitOnTest,
    CrossValOnFittedData,
    VectorizerFitThenTransform,
    ScalerFitBeforeSplit,
    PrefitBeforeCrossValidation,
    MergeWithoutValidate,
    PolarsJoinWithoutValidate,
    MergeWithoutOn,
    MergeAsofUnsorted,
    PolarsJoinAsofUnsorted,
    ConcatColumnsUnchecked,
    JoinKeyUnvalidated,
    TransformedTargetPair,
    ScaledTargetWithoutInverse,
    CompareToMissing,
    PolarsNullComparison,
    CompareToNan,
    NanToInt,
    ApplyCast,
    AstypeThenToNumpy,
    SilentDtypeCoercion,
    StatefulFunctionTransformer,
    ReadParquetAllColumns,
    NoInplace,
    FrameColumnWrite,
    LambdaInAssign,
    NestedStepCalls,
    ScriptSteps,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;

    const GROUPS: &[Group] = &[
        Group::Leakage,
        Group::Joins,
        Group::Target,
        Group::Coercion,
        Group::PipelineShape,
    ];
    const WORKFLOW: WorkflowId = WorkflowId::DataPrep;
    const GOALS: &[ledger::Goal] = &[ledger::Goal::Design];

    fn group(self) -> Group {
        match self {
            Self::FitBeforeSplit
            | Self::FitOnTest
            | Self::CrossValOnFittedData
            | Self::VectorizerFitThenTransform
            | Self::ScalerFitBeforeSplit
            | Self::PrefitBeforeCrossValidation => Group::Leakage,
            Self::MergeWithoutValidate
            | Self::PolarsJoinWithoutValidate
            | Self::MergeWithoutOn
            | Self::MergeAsofUnsorted
            | Self::PolarsJoinAsofUnsorted
            | Self::ConcatColumnsUnchecked
            | Self::JoinKeyUnvalidated => Group::Joins,
            Self::TransformedTargetPair | Self::ScaledTargetWithoutInverse => Group::Target,
            Self::CompareToMissing
            | Self::PolarsNullComparison
            | Self::CompareToNan
            | Self::NanToInt
            | Self::ApplyCast
            | Self::AstypeThenToNumpy
            | Self::SilentDtypeCoercion => Group::Coercion,
            Self::StatefulFunctionTransformer
            | Self::ReadParquetAllColumns
            | Self::NoInplace
            | Self::FrameColumnWrite
            | Self::LambdaInAssign
            | Self::NestedStepCalls
            | Self::ScriptSteps => Group::PipelineShape,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Leakage => "Leakage",
            Group::Joins => "Joins and alignment",
            Group::Target => "Target transforms",
            Group::Coercion => "Dtypes and missing values",
            Group::PipelineShape => "Pipeline shape",
        }
    }
}

#[derive(Debug, clap::Args)]
pub(crate) struct DataPrepArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// A Python adapter file or shell command that emits column facts.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Name the target column.
    #[arg(long, value_name = "COLUMN")]
    target: Option<String>,
    /// Print the facts and questions catalog without a file scan.
    #[arg(long)]
    questions: bool,
}

/// Runs a static data-prep survey without runtime adapters for `xpd audit`.
pub(crate) fn audit_survey(
    paths: &[std::path::PathBuf],
    tree: ledger::Tree<'_>,
    top: Option<&std::path::Path>,
    commit: &str,
) -> Result<ledger::AuditSurvey> {
    Ok(ledger::AuditSurvey::from_output(survey::survey(
        paths,
        tree,
        top,
        commit,
        None,
        ledger::AdapterPolicy::BestEffort,
    )?))
}

impl DataPrepArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            return QuestionCatalog::load(CATALOG)?
                .print(self.select.format_override().unwrap_or(default));
        }
        let adapter = record_target(self.adapter, self.target)?;
        self.select
            .print_with(default, adapter.as_deref(), survey::survey, |ledger| {
                let mut text = format!(
                    "xpd {}: {} in {} Python files, {} checklist items\n",
                    Kind::WORKFLOW.as_str(),
                    ledger.count(),
                    ledger.read.files,
                    ledger.checklist.len()
                );
                if let Some(facts) = &ledger.read.facts {
                    use std::fmt::Write as _;
                    if let Some(error) = &facts.error {
                        let _ = writeln!(text, "Adapter error: {error}");
                    }
                    for column in &facts.columns {
                        let _ = writeln!(text, "  {}: {}", column.name, column.recommendation);
                    }
                }
                text
            })
    }
}
