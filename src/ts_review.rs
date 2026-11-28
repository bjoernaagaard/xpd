mod static_rules;
mod survey;

use std::process::ExitCode;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use crate::ledger::{self, QuestionCatalog, Select, WorkflowId};
use crate::output::Format;

const CATALOG: &str = include_str!("../rules/xpd/ts-review.yml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    Evaluation,
    Lags,
    Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    AnalyticForecastHorizon,
    ArchIgnoredArgument,
    ArchRejectedNumber,
    ArchUnknownOption,
    ArchUnseededDraws,
    FitOutputInLoop,
    ForecastPerOrigin,
    OptBandwidthAsBandwidth,
    ReindexIsDefault,
    RemovedInArch,
    RescaledUnits,
    ResultsBeforeCompute,
    UnscaledReturns,
    IntervalsWithoutCalibration,
    MaseWithoutTrainDf,
    RandomTimeSplit,
    MapeOnIntermittent,
    FittedValuesWithoutFitted,
    MintraceWithoutInsample,
    IndptrMissingZero,
    UngroupedLag,
    MissingFutureExog,
    RemovedTsaArguments,
    DeprecatedVarSeed,
    TimeBeforeEntityIndex,
    PanelOlsWithoutEffects,
    PanelDefaultCovariance,
    ClusteredWithoutClusters,
    CovOptionWithoutCovType,
    TooManyEffects,
    NotATimeSeries,
    IncoherentGroup,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;
    const GROUPS: &[Group] = &[Group::Evaluation, Group::Lags, Group::Identity];
    const WORKFLOW: WorkflowId = WorkflowId::TsReview;
    const GOALS: &[ledger::Goal] = &[ledger::Goal::Design];

    fn group(self) -> Group {
        match self {
            Self::AnalyticForecastHorizon
            | Self::ArchIgnoredArgument
            | Self::ArchRejectedNumber
            | Self::ArchUnknownOption
            | Self::ArchUnseededDraws
            | Self::FitOutputInLoop
            | Self::ForecastPerOrigin
            | Self::OptBandwidthAsBandwidth
            | Self::ReindexIsDefault
            | Self::RemovedInArch
            | Self::RescaledUnits
            | Self::ResultsBeforeCompute
            | Self::UnscaledReturns
            | Self::IntervalsWithoutCalibration
            | Self::MaseWithoutTrainDf
            | Self::RandomTimeSplit
            | Self::MapeOnIntermittent => Group::Evaluation,
            Self::FittedValuesWithoutFitted
            | Self::MintraceWithoutInsample
            | Self::IndptrMissingZero
            | Self::UngroupedLag
            | Self::MissingFutureExog => Group::Lags,
            Self::RemovedTsaArguments
            | Self::DeprecatedVarSeed
            | Self::TimeBeforeEntityIndex
            | Self::PanelOlsWithoutEffects
            | Self::PanelDefaultCovariance
            | Self::ClusteredWithoutClusters
            | Self::CovOptionWithoutCovType
            | Self::TooManyEffects
            | Self::NotATimeSeries
            | Self::IncoherentGroup => Group::Identity,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Evaluation => "Evaluation",
            Group::Lags => "Lags and exogenous features",
            Group::Identity => "Grouping and identity",
        }
    }
}

#[derive(Debug, clap::Args)]
pub(crate) struct TsReviewArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// A Python adapter file or a shell command that emits time series facts.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Print the facts and questions catalog without scanning files.
    #[arg(long)]
    questions: bool,
}

impl TsReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            return QuestionCatalog::load(CATALOG)?
                .print(self.select.format_override().unwrap_or(default));
        }
        self.select
            .print_with(default, self.adapter.as_deref(), survey::survey, |ledger| {
                let mut heading = format!(
                    "xpd ts-review: {} in {} Python files, {} checklist items\n",
                    ledger.count(),
                    ledger.read.files,
                    ledger.checklist.len()
                );
                if let Some(error) = ledger
                    .read
                    .facts
                    .as_ref()
                    .and_then(|facts| facts["error"].as_str())
                {
                    use std::fmt::Write as _;
                    let _ = writeln!(heading, "Adapter error: {error}");
                }
                heading
            })
    }
}
