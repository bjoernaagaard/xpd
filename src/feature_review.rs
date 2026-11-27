mod fold_facts;
mod static_survey;

use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ledger::{self, Catalog, Select};
use crate::output::Format;

const CATALOG: &str = include_str!("../rules/xpd/feature-review.yml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    CutoffAndFolds,
    Computation,
    Pipeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    ColsLeftToDtype,
    EncodeBeforeSplit,
    EncoderNoiseWithoutSeed,
    HashingDefaultComponents,
    InvalidHandleOption,
    NamedColsAfterArrayStep,
    NanIntoPipeline,
    OrdinalMappingPairs,
    OrdinalMappingUnknown,
    SupervisedEncoderWithoutTarget,
    TransformWithTestTarget,
    WrapperFitReturnsNone,
    ZeroRegularization,
    NodeMutatesInput,
    PredictDisabledShapeCheck,
    AucFromLabels,
    CrossValOnFittedData,
    FitBeforeSplit,
    FitOnTest,
    GroupsIgnored,
    LeaveOneOutScore,
    MetricArgumentsSwapped,
    MissingGroups,
    MissingRandomState,
    PandasOutputSparse,
    PrefitBeforeCrossValidation,
    ScaledTargetWithoutInverse,
    ScalerFitBeforeSplit,
    ScoreOnTrainingRows,
    StatefulFunctionTransformer,
    TransformedTargetPair,
    TuneOnTest,
    UnknownScoringName,
    AddLagsPositive,
    ColumnCapperFraction,
    DatasetFrameAndXy,
    DecayOnShuffledRows,
    DictMapperNotInt,
    EstimatorTransformerProba,
    OutlierRemoverBeforeModel,
    OutlierThreshold,
    PcaOutlierAfterScaler,
    RegressionOutlierColumnName,
    ScorerCalledAsMetric,
    SelectorWithoutFeatureNames,
    UnseededRandomEstimator,
    NativePredictAfterEarlyStopping,
    QuantileEvalWithoutTrainRef,
    ReusedEarlyStoppingCallback,
    FrameColumnWrite,
    RollingIncludesCurrent,
    RollingCrossesGroups,
    TargetEncoderBeforeOuterCv,
    DuplicateFeatureComputation,
}

impl Kind {
    fn source_rule(self) -> &'static str {
        match self {
            Self::ColsLeftToDtype => "category_encoders/cols-left-to-dtype",
            Self::EncodeBeforeSplit => "category_encoders/encode-before-split",
            Self::EncoderNoiseWithoutSeed => "category_encoders/encoder-noise-without-seed",
            Self::HashingDefaultComponents => "category_encoders/hashing-default-components",
            Self::InvalidHandleOption => "category_encoders/invalid-handle-option",
            Self::NamedColsAfterArrayStep => "category_encoders/named-cols-after-array-step",
            Self::NanIntoPipeline => "category_encoders/nan-into-pipeline",
            Self::OrdinalMappingPairs => "category_encoders/ordinal-mapping-pairs",
            Self::OrdinalMappingUnknown => "category_encoders/ordinal-mapping-unknown",
            Self::SupervisedEncoderWithoutTarget => {
                "category_encoders/supervised-encoder-without-target"
            }
            Self::TransformWithTestTarget => "category_encoders/transform-with-test-target",
            Self::WrapperFitReturnsNone => "category_encoders/wrapper-fit-returns-none",
            Self::ZeroRegularization => "category_encoders/zero-regularization",
            Self::NodeMutatesInput => "hamilton/node-mutates-input",
            Self::PredictDisabledShapeCheck => "lightgbm/predict-disabled-shape-check",
            Self::AucFromLabels => "sklearn/auc-from-labels",
            Self::CrossValOnFittedData => "sklearn/cross-val-on-fitted-data",
            Self::FitBeforeSplit => "sklearn/fit-before-split",
            Self::FitOnTest => "sklearn/fit-on-test",
            Self::GroupsIgnored => "sklearn/groups-ignored",
            Self::LeaveOneOutScore => "sklearn/leave-one-out-score",
            Self::MetricArgumentsSwapped => "sklearn/metric-arguments-swapped",
            Self::MissingGroups => "sklearn/missing-groups",
            Self::MissingRandomState => "sklearn/missing-random-state",
            Self::PandasOutputSparse => "sklearn/pandas-output-sparse",
            Self::PrefitBeforeCrossValidation => "sklearn/prefit-before-cross-validation",
            Self::ScaledTargetWithoutInverse => "sklearn/scaled-target-without-inverse",
            Self::ScalerFitBeforeSplit => "sklearn/scaler-fit-before-split",
            Self::ScoreOnTrainingRows => "sklearn/score-on-training-rows",
            Self::StatefulFunctionTransformer => "sklearn/stateful-function-transformer",
            Self::TransformedTargetPair => "sklearn/transformed-target-pair",
            Self::TuneOnTest => "sklearn/tune-on-test",
            Self::UnknownScoringName => "sklearn/unknown-scoring-name",
            Self::AddLagsPositive => "sklego/add-lags-positive",
            Self::ColumnCapperFraction => "sklego/column-capper-fraction",
            Self::DatasetFrameAndXy => "sklego/dataset-frame-and-xy",
            Self::DecayOnShuffledRows => "sklego/decay-on-shuffled-rows",
            Self::DictMapperNotInt => "sklego/dict-mapper-not-int",
            Self::EstimatorTransformerProba => "sklego/estimator-transformer-proba",
            Self::OutlierRemoverBeforeModel => "sklego/outlier-remover-before-model",
            Self::OutlierThreshold => "sklego/outlier-threshold",
            Self::PcaOutlierAfterScaler => "sklego/pca-outlier-after-scaler",
            Self::RegressionOutlierColumnName => "sklego/regression-outlier-column-name",
            Self::ScorerCalledAsMetric => "sklego/scorer-called-as-metric",
            Self::SelectorWithoutFeatureNames => "sklego/selector-without-feature-names",
            Self::UnseededRandomEstimator => "sklego/unseeded-random-estimator",
            Self::NativePredictAfterEarlyStopping => "xgboost/native-predict-after-early-stopping",
            Self::QuantileEvalWithoutTrainRef => "xgboost/quantile-eval-without-train-ref",
            Self::ReusedEarlyStoppingCallback => "xgboost/reused-early-stopping-callback",
            Self::FrameColumnWrite => "pandas/frame-column-write",
            Self::RollingIncludesCurrent => "feature-review/rolling-includes-current",
            Self::RollingCrossesGroups => "feature-review/rolling-crosses-groups",
            Self::TargetEncoderBeforeOuterCv => "feature-review/target-encoder-before-outer-cv",
            Self::DuplicateFeatureComputation => "feature-review/duplicate-feature-computation",
        }
    }
}

impl ledger::Kind for Kind {
    type Group = Group;
    const GROUPS: &[Group] = &[Group::CutoffAndFolds, Group::Computation, Group::Pipeline];
    const COMMAND: &str = "feature-review";
    const GOALS: &[ledger::Goal] = &[ledger::Goal::Design];

    fn source_rule(self) -> Option<&'static str> {
        Some(self.source_rule())
    }

    fn group(self) -> Group {
        match self {
            Self::ColsLeftToDtype
            | Self::EncoderNoiseWithoutSeed
            | Self::HashingDefaultComponents
            | Self::InvalidHandleOption
            | Self::NamedColsAfterArrayStep
            | Self::NanIntoPipeline
            | Self::OrdinalMappingPairs
            | Self::OrdinalMappingUnknown
            | Self::WrapperFitReturnsNone
            | Self::ZeroRegularization
            | Self::PredictDisabledShapeCheck
            | Self::LeaveOneOutScore
            | Self::MetricArgumentsSwapped
            | Self::EstimatorTransformerProba
            | Self::ScorerCalledAsMetric
            | Self::UnseededRandomEstimator => Group::Pipeline,
            Self::EncodeBeforeSplit
            | Self::SupervisedEncoderWithoutTarget
            | Self::TransformWithTestTarget
            | Self::CrossValOnFittedData
            | Self::FitBeforeSplit
            | Self::FitOnTest
            | Self::GroupsIgnored
            | Self::MissingGroups
            | Self::ScaledTargetWithoutInverse
            | Self::ScalerFitBeforeSplit
            | Self::ScoreOnTrainingRows
            | Self::TransformedTargetPair
            | Self::TuneOnTest
            | Self::NativePredictAfterEarlyStopping
            | Self::QuantileEvalWithoutTrainRef
            | Self::ReusedEarlyStoppingCallback
            | Self::RollingIncludesCurrent
            | Self::RollingCrossesGroups
            | Self::TargetEncoderBeforeOuterCv => Group::CutoffAndFolds,
            Self::NodeMutatesInput
            | Self::AucFromLabels
            | Self::MissingRandomState
            | Self::PandasOutputSparse
            | Self::PrefitBeforeCrossValidation
            | Self::StatefulFunctionTransformer
            | Self::UnknownScoringName
            | Self::AddLagsPositive
            | Self::ColumnCapperFraction
            | Self::DatasetFrameAndXy
            | Self::DecayOnShuffledRows
            | Self::DictMapperNotInt
            | Self::OutlierRemoverBeforeModel
            | Self::OutlierThreshold
            | Self::PcaOutlierAfterScaler
            | Self::RegressionOutlierColumnName
            | Self::SelectorWithoutFeatureNames
            | Self::FrameColumnWrite
            | Self::DuplicateFeatureComputation => Group::Computation,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::CutoffAndFolds => "Cutoff and fold integrity",
            Group::Computation => "Feature computation",
            Group::Pipeline => "Pipeline evidence",
        }
    }
}

#[derive(Debug, clap::Args)]
pub(crate) struct FeatureReviewArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// A Python adapter file or shell command that emits development-fold evidence.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Print the facts and questions catalog without scanning files.
    #[arg(long)]
    questions: bool,
}

impl FeatureReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            return Catalog::load(CATALOG)?.print(self.select.format_override().unwrap_or(default));
        }
        self.select.print_with(
            default,
            self.adapter.as_deref(),
            static_survey::survey,
            |ledger| {
                let runtime = fold_facts::facts_text(&ledger.read.facts);
                format!(
                    "xpd feature-review: {}, {} checklist items\n{runtime}",
                    ledger.count(),
                    ledger.checklist.len()
                )
            },
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    target: String,
    prediction_cutoff: String,
    development: Value,
    baseline: Vec<String>,
    features: Vec<Feature>,
    #[serde(flatten)]
    inputs: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Feature {
    id: String,
    name: String,
    kind: FeatureKind,
    owner: String,
    #[serde(alias = "output")]
    column: Option<String>,
    #[serde(alias = "graph_node")]
    node: Option<String>,
    rolling: Option<Rolling>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum FeatureKind {
    Feature,
    Hypothesis,
}

impl FeatureKind {
    fn name(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Hypothesis => "hypothesis",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Rolling {
    source: String,
    #[serde(default)]
    group_keys: Vec<String>,
    #[serde(default)]
    prior_only: bool,
}

fn repo_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}
