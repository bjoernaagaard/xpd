//! Compile-time registry of xpd ledger workflows and their behavioral policies.
//!
//! This module imports only `core`/`std` so `build.rs` and `examples/xtask.rs` can include it
//! directly via `#[path = "src/ledger/workflow_registry.rs"]` without adding workspace crates or
//! build dependencies.

use std::fmt;
use std::str::FromStr;

/// Error returned when a persisted or CLI workflow string does not match any [`WorkflowId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownWorkflowError(String);

impl fmt::Display for UnknownWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown ledger workflow `{}`", self.0)
    }
}

impl std::error::Error for UnknownWorkflowError {}

/// Every recorded ledger workflow shipped by xpd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum WorkflowId {
    ChartReview,
    DataPrep,
    Discover,
    FeatureReview,
    Harden,
    ModelReview,
    OptReview,
    Optimize,
    TsReview,
    Untangle,
}

/// How a workflow validates runtime adapters, `--target`, and worker survey facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowAdapterPolicy {
    /// Accepts any adapter argument without extra start or worker restrictions.
    Unrestricted,
    /// Allows `--target` on `start` and fails worker surveys on `read.facts.error`.
    DataPrepTarget,
    /// Fails worker surveys when `read.facts.error` is present.
    FactsRequiredOnWorker,
    /// Requires a `.py` adapter passed to `start` to be committed at `HEAD`.
    CommittedPython,
    /// Requires a committed `.py` adapter and fails worker surveys on facts errors or ambiguities.
    CommittedModelFacts,
}

impl WorkflowAdapterPolicy {
    /// Whether `start` accepts the `--target` column flag.
    pub(crate) fn allows_target_flag(self) -> bool {
        matches!(self, Self::DataPrepTarget)
    }

    /// Whether a Python file passed to `start --adapter` must be committed at `HEAD`.
    pub(crate) fn requires_committed_python(self) -> bool {
        matches!(self, Self::CommittedPython | Self::CommittedModelFacts)
    }
}

/// How a workflow handles checklist questions, fix goals, finding removal, and empty inventories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChecklistPolicy {
    /// Module-level structural questions (`untangle`, `optimize`, `harden`, `discover`).
    ModuleQuestions,
    /// Pipeline or series questions with soft-fail when finishing with an open fix.
    PipelineQuestions,
    /// Per-figure questions allowed without a `design` goal and strict finding removal.
    FigureQuestions,
    /// Per-feature questions with strict finding removal and soft-fail open fix on finish.
    FeatureQuestions,
    /// Non-empty checklist inventory required on `start`, strict removal, and soft-fail finish.
    RequiredInventory { empty_error: &'static str },
}

impl ChecklistPolicy {
    /// Whether a gate requires at least one cited finding to disappear even when a goal is set.
    pub(crate) fn strict_finding_removal(self) -> bool {
        matches!(
            self,
            Self::FigureQuestions | Self::FeatureQuestions | Self::RequiredInventory { .. }
        )
    }

    /// Whether a plan fix may cite checklist questions without setting `goal: "design"`.
    pub(crate) fn allows_questions_without_design_goal(self) -> bool {
        matches!(self, Self::FigureQuestions)
    }

    /// Whether `finish` returns exit 1 with a status line instead of an error when a fix is open.
    pub(crate) fn soft_fail_open_fix_on_finish(self) -> bool {
        matches!(
            self,
            Self::PipelineQuestions | Self::FeatureQuestions | Self::RequiredInventory { .. }
        )
    }

    /// Error message when `start` discovers zero checklist items in a required-inventory workflow.
    pub(crate) fn empty_inventory_error(self) -> Option<&'static str> {
        match self {
            Self::RequiredInventory { empty_error } => Some(empty_error),
            Self::ModuleQuestions
            | Self::PipelineQuestions
            | Self::FigureQuestions
            | Self::FeatureQuestions => None,
        }
    }
}

/// How a workflow resolves the commit or tree measured during scans and recorded stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeadMeasurementPolicy {
    /// Surveys the working tree directly.
    WorkingTree,
    /// Rewrites `--since HEAD` to `HEAD^` when the working tree has no tracked diff.
    CleanSinceParent,
    /// Recorded runs survey committed `HEAD` through `at_commit`.
    CommittedHead,
    /// Surveys committed `HEAD` and requires `HEAD == run.current` on `plan` and `finish`.
    LockedKeptCommit,
}

impl HeadMeasurementPolicy {
    /// Whether recorded stage measurements survey the committed `HEAD` snapshot.
    pub(crate) fn measures_committed_head(self) -> bool {
        matches!(self, Self::CommittedHead | Self::LockedKeptCommit)
    }
}

/// Whether a workflow tracks fold-level feature ablation evidence across stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeatureEvidencePolicy {
    None,
    FoldAblation,
}

/// How strictly a gate rejects benchmark slowdowns on `--bench`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BenchmarkPolicy {
    /// Rejects only when slowdown exceeds threshold, gap > 5 ms, and intervals do not overlap.
    NonOverlappingSlowdown,
    /// Rejects any median slowdown exceeding the threshold.
    StrictSlowdown,
}

/// Whether the first check command is repeated to verify deterministic stdout and exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckDeterminismPolicy {
    SinglePass,
    RepeatFirstStdoutHash,
}

/// Full declarative policy descriptor for one registered ledger workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowSpec {
    pub(crate) id: WorkflowId,
    pub(crate) cli_name: &'static str,
    pub(crate) adapter: WorkflowAdapterPolicy,
    pub(crate) checklist: ChecklistPolicy,
    pub(crate) head: HeadMeasurementPolicy,
    pub(crate) feature_evidence: FeatureEvidencePolicy,
    pub(crate) benchmark: BenchmarkPolicy,
    pub(crate) check_determinism: CheckDeterminismPolicy,
}

impl WorkflowSpec {
    /// Returns the default declarative policy specification for a module-question workflow.
    const fn standard(id: WorkflowId) -> Self {
        Self {
            id,
            cli_name: id.as_str(),
            adapter: WorkflowAdapterPolicy::Unrestricted,
            checklist: ChecklistPolicy::ModuleQuestions,
            head: HeadMeasurementPolicy::WorkingTree,
            feature_evidence: FeatureEvidencePolicy::None,
            benchmark: BenchmarkPolicy::NonOverlappingSlowdown,
            check_determinism: CheckDeterminismPolicy::SinglePass,
        }
    }

    const fn with_adapter(mut self, adapter: WorkflowAdapterPolicy) -> Self {
        self.adapter = adapter;
        self
    }

    const fn with_checklist(mut self, checklist: ChecklistPolicy) -> Self {
        self.checklist = checklist;
        self
    }

    const fn with_head(mut self, head: HeadMeasurementPolicy) -> Self {
        self.head = head;
        self
    }

    const fn with_feature_evidence(mut self, feature_evidence: FeatureEvidencePolicy) -> Self {
        self.feature_evidence = feature_evidence;
        self
    }

    const fn with_benchmark(mut self, benchmark: BenchmarkPolicy) -> Self {
        self.benchmark = benchmark;
        self
    }

    const fn with_check_determinism(mut self, check_determinism: CheckDeterminismPolicy) -> Self {
        self.check_determinism = check_determinism;
        self
    }
}

impl WorkflowId {
    /// Every registered workflow in alphabetical order.
    pub(crate) const ALL: &'static [Self] = &[
        Self::ChartReview,
        Self::DataPrep,
        Self::Discover,
        Self::FeatureReview,
        Self::Harden,
        Self::ModelReview,
        Self::OptReview,
        Self::Optimize,
        Self::TsReview,
        Self::Untangle,
    ];

    /// The exact kebab-case CLI subcommand and wire identifier for this workflow.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ChartReview => "chart-review",
            Self::DataPrep => "data-prep",
            Self::Discover => "discover",
            Self::FeatureReview => "feature-review",
            Self::Harden => "harden",
            Self::ModelReview => "model-review",
            Self::OptReview => "opt-review",
            Self::Optimize => "optimize",
            Self::TsReview => "ts-review",
            Self::Untangle => "untangle",
        }
    }

    /// Returns the compile-time policy specification for this workflow.
    pub(crate) const fn spec(self) -> WorkflowSpec {
        let base = WorkflowSpec::standard(self);
        match self {
            Self::ChartReview => base.with_checklist(ChecklistPolicy::FigureQuestions),
            Self::DataPrep => base
                .with_adapter(WorkflowAdapterPolicy::DataPrepTarget)
                .with_checklist(ChecklistPolicy::PipelineQuestions),
            Self::Discover | Self::Untangle => base,
            Self::FeatureReview => base
                .with_adapter(WorkflowAdapterPolicy::CommittedPython)
                .with_checklist(ChecklistPolicy::FeatureQuestions)
                .with_head(HeadMeasurementPolicy::LockedKeptCommit)
                .with_feature_evidence(FeatureEvidencePolicy::FoldAblation),
            Self::Harden => {
                base.with_check_determinism(CheckDeterminismPolicy::RepeatFirstStdoutHash)
            }
            Self::ModelReview => base
                .with_adapter(WorkflowAdapterPolicy::CommittedModelFacts)
                .with_checklist(ChecklistPolicy::RequiredInventory {
                    empty_error: "no models found in the selected Python files",
                })
                .with_head(HeadMeasurementPolicy::CommittedHead),
            Self::OptReview => base
                .with_adapter(WorkflowAdapterPolicy::CommittedPython)
                .with_checklist(ChecklistPolicy::RequiredInventory {
                    empty_error: "no optimization problems found in the selected Python files",
                })
                .with_head(HeadMeasurementPolicy::CommittedHead),
            Self::Optimize => base.with_benchmark(BenchmarkPolicy::StrictSlowdown),
            Self::TsReview => base
                .with_adapter(WorkflowAdapterPolicy::FactsRequiredOnWorker)
                .with_checklist(ChecklistPolicy::PipelineQuestions)
                .with_head(HeadMeasurementPolicy::CleanSinceParent),
        }
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for WorkflowId {
    type Err = UnknownWorkflowError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|id| id.as_str() == raw)
            .ok_or_else(|| UnknownWorkflowError(raw.to_owned()))
    }
}
