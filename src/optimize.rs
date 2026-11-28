mod optimize_python;
mod runtime;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;

use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, AdapterPolicy, FindingPlace, Ledger, QuestionCatalog, Select, SurveyOutput, Tree,
    WorkflowId,
};
use crate::output::Format;
use crate::source;
use crate::syntax;
use crate::workspace::Workspace;

type Finding = ledger::LedgerFinding<Kind>;
const PERFORMANCE_RULES: &[&str] = &[
    "arch/fit-output-in-loop",
    "arch/forecast-per-origin",
    "category_encoders/sklearn-ships-encoder",
    "cvxpy/dense-identity-quad-form",
    "mapie/refit-per-confidence-level",
    "numpy/eig-on-symmetric",
    "numpy/einsum-optimize",
    "numpy/element-loop",
    "numpy/file-roundtrip",
    "numpy/growth-in-loop",
    "numpy/inv-then-multiply",
    "numpy/mask-per-label",
    "numpy/mergesort-kind",
    "numpy/needless-copy",
    "numpy/np-vectorize",
    "numpy/per-value-comprehension",
    "numpy/random-draw-loop",
    "numpy/recurrence-loop",
    "numpy/rng-in-loop",
    "numpy/slice-fill-loop",
    "numpy/sort-then-select",
    "pandas/aggregate-then-merge-back",
    "pandas/apply-arithmetic",
    "pandas/apply-axis-1",
    "pandas/apply-cast",
    "pandas/apply-comparison",
    "pandas/apply-condition",
    "pandas/apply-date-expression",
    "pandas/apply-date-method",
    "pandas/apply-date-part",
    "pandas/apply-known-function",
    "pandas/apply-math-function",
    "pandas/apply-on-dates",
    "pandas/apply-parse-date",
    "pandas/apply-pd-series",
    "pandas/apply-recomputes-aggregate",
    "pandas/apply-text-method",
    "pandas/chunked-csv-concat",
    "pandas/column-loop",
    "pandas/column-loop-to-long",
    "pandas/concat-in-loop",
    "pandas/copy-after-indexing",
    "pandas/copy-in-chain",
    "pandas/frame-growth-in-loop",
    "pandas/groupby-apply",
    "pandas/groupby-filter-lambda",
    "pandas/groupby-lambda",
    "pandas/groupby-lambda-arithmetic",
    "pandas/groupby-lambda-window",
    "pandas/groupby-loop-collects",
    "pandas/groupby-then-reset-index",
    "pandas/index-loop",
    "pandas/list-over-column",
    "pandas/lookup-per-row",
    "pandas/named-agg-lambda",
    "pandas/numpy-round-trip",
    "pandas/read-csv-converters",
    "pandas/read-json-lines-engine",
    "pandas/read-then-head",
    "pandas/repeated-reset-index",
    "pandas/rolling-apply",
    "pandas/row-loop",
    "pandas/sort-then-head",
    "pandas/strftime-date-part",
    "pandas/strftime-round-trip",
    "pandas/zip-over-columns",
    "polars/polars-collect-in-loop",
    "polars/polars-concat-in-loop",
    "polars/polars-eager-read-before-query",
    "polars/polars-groupby-join-over",
    "polars/polars-sort-then-head",
    "pymoo/unread-history",
    "python/counting-loop-without-counter",
    "python/front-insert-in-loop",
    "python/setdefault-without-defaultdict",
    "scipy/callable-metric",
    "scipy/distance-per-pair",
    "scipy/per-value-scipy-call",
    "scipy/query-per-point",
    "scipy/solve-in-loop",
    "scipy/sparse-inv-then-multiply",
    "sklearn/cross-val-score-per-metric",
    "sklearn/predict-and-predict-proba",
    "sklearn/predict-per-row",
    "sklearn/refit-per-n-estimators",
    "sklearn/vectorizer-fit-then-transform",
    "sklego/group-time-series-split-folds",
    "xgboost/dmatrix-rebuilt-for-predict",
];

/// The kinds of work that code does for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    Rulekit,
    DoneAgain,
    LargerThanTheAnswer,
}

/// What a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// A rule of `xpd check` or the project's own rule pack fires on this line.
    RuleIssue,
    /// A function computes the same call more than once, with nothing changed in between.
    RepeatedCall,
    /// A function computes the same boolean mask or masked slice more than once.
    RepeatedMask,
    /// A loop or comprehension computes a call on every pass and changes nothing it reads.
    LoopInvariantCall,
    /// A matrix of every element against every other is built and then reduced or contracted.
    MatrixReduced,
    /// A full diff, sort or cumulative sum is built only to read one element or a short head.
    MaterializedForOne,
    /// A loop scans a list for membership or filters a whole frame on every pass.
    LinearScanInLoop,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;

    const GOALS: &'static [ledger::Goal] = &[
        ledger::Goal::Faster,
        ledger::Goal::Both,
        ledger::Goal::Design,
    ];

    const WORKFLOW: WorkflowId = WorkflowId::Optimize;

    const GROUPS: &'static [Group] =
        &[Group::Rulekit, Group::DoneAgain, Group::LargerThanTheAnswer];

    fn group(self) -> Group {
        match self {
            Self::RuleIssue => Group::Rulekit,
            Self::RepeatedCall | Self::RepeatedMask | Self::LoopInvariantCall => Group::DoneAgain,
            Self::MatrixReduced | Self::MaterializedForOne | Self::LinearScanInLoop => {
                Group::LargerThanTheAnswer
            }
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Rulekit => {
                "Rulekit: issues that `xpd check` and the project's own rules already report"
            }
            Group::DoneAgain => {
                "Done again: a value computed more than once, which one computation serves"
            }
            Group::LargerThanTheAnswer => {
                "Larger than the answer: work or storage that is much larger than the result"
            }
        }
    }
}

/// What a run read.
#[derive(Debug, Serialize)]
struct Read {
    /// The files of code.
    files: usize,
    /// The Python files among them.
    python: usize,
    /// Total lines across the files of code.
    lines: usize,
    workloads: Vec<runtime::Workload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adapter_error: Option<String>,
}

/// Find work that spends CPU, memory or I/O the result does not need.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd optimize
  xpd optimize src/ --format json
  xpd optimize --kind repeated-call,matrix-reduced --limit 0
  xpd optimize --since main")]
pub(crate) struct OptimizeArgs {
    #[command(flatten)]
    select: Select<Kind>,
    #[arg(long)]
    questions: bool,
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
}

impl OptimizeArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let catalog = QuestionCatalog::load(include_str!("../rules/xpd/optimize.yml"))?;
        if self.questions {
            return catalog.print(self.select.format_override().unwrap_or(default));
        }
        let header = |ledger: &Ledger<Kind, Read>| {
            let Read {
                files,
                python,
                lines,
                ..
            } = &ledger.read;
            let files = if *files == 1 {
                "1 file".to_owned()
            } else {
                format!("{files} files")
            };
            format!(
                "xpd optimize: {} in {files} ({lines} lines), {python} of them Python\n",
                ledger.count()
            )
        };
        self.select.print_with(
            default,
            self.adapter.as_deref(),
            |paths, tree, top, commit, adapter, policy| {
                let listing = Listing::of(paths, tree, top, commit)?;
                let work_dir = tree
                    .top
                    .map_or_else(std::env::current_dir, |top| Ok(top.to_path_buf()))?;
                let (workloads, adapter_error) = match adapter
                    .map(|adapter| runtime::measure(adapter, &work_dir))
                    .transpose()
                {
                    Ok(workloads) => (workloads.unwrap_or_default(), None),
                    Err(error) if policy == AdapterPolicy::BestEffort => {
                        (Vec::new(), Some(error.to_string()))
                    }
                    Err(error) => return Err(error),
                };
                survey(paths, tree, &listing, workloads, adapter_error, &catalog)
            },
            header,
        )
    }
}

/// Runs a static optimize survey without executing runtime workloads for `xpd audit`.
pub(crate) fn audit_survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
) -> Result<ledger::AuditSurvey> {
    let catalog = QuestionCatalog::load(include_str!("../rules/xpd/optimize.yml"))?;
    let listing = Listing::of(paths, tree, top, commit)?;
    Ok(ledger::AuditSurvey::from_output(survey(
        paths,
        tree,
        &listing,
        Vec::new(),
        None,
        &catalog,
    )?))
}

/// What one file gives the ledger.
struct Surveyed {
    shown: String,
    /// The path that names the file in the key of a finding, which is the same at another
    /// commit.
    named: String,
    lines: usize,
    python: Option<optimize_python::Facts>,
    rules: Vec<(usize, String, String)>,
}

/// One file as the ledger needs it. `None` for a file that cannot be read.
fn read(
    path: &Path,
    language: SupportLang,
    tree: Tree<'_>,
    listing: &Listing,
    workspace: Option<&Workspace>,
) -> Option<Surveyed> {
    let text = listing.text(path)?;
    let absolute = listing.absolute(path)?;
    let root = syntax::parse_as(&text, language);
    let shown = tree.shown(path);
    let mut rules = Vec::new();
    if let Some(workspace) = workspace {
        let linted = workspace.analyze(source::File::on_disk(path), &text);
        for found in &linted.found {
            let issue = found.issue();
            let line = issue.position.map_or(1, |at| at.line);
            let rule = issue.id.to_string();
            if issue.severity != crate::severity::Severity::Error
                && PERFORMANCE_RULES.contains(&rule.as_str())
            {
                rules.push((line, rule, issue.message));
            }
        }
    }
    let summary =
        crate::ledger::salsa_db::cached_optimize_facts(&shown, &text, language, "optimize");
    Some(Surveyed {
        named: tree.tracked(&absolute).unwrap_or_else(|| shown.clone()),
        shown,
        lines: summary.lines,
        python: (language == SupportLang::Python).then(|| optimize_python::facts(path, &root)),
        rules,
    })
}

fn python_findings(surveyed: &[Surveyed]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in surveyed {
        let Some(facts) = &file.python else {
            continue;
        };
        for local in &facts.local {
            findings.push(Finding {
                kind: local.kind,
                key: format!("{}\n{}", file.named, local.key),
                places: (local.lines.iter().collect::<BTreeSet<_>>().into_iter())
                    .map(|line| FindingPlace::line(&file.shown, *line))
                    .collect(),
                evidence: local.evidence.clone(),
                weight: local.weight,
            });
        }
    }
    findings
}

/// The rule issues from `xpd check` and any project rule packs on the surveyed files.
fn rule_findings(surveyed: &[Surveyed]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in surveyed {
        for (line, rule, message) in &file.rules {
            findings.push(Finding {
                kind: Kind::RuleIssue,
                key: format!("{}\n{rule}\n{message}", file.named),
                places: vec![FindingPlace::line(&file.shown, *line)],
                evidence: format!("`{rule}`: {message}"),
                weight: 1,
            });
        }
    }
    findings
}

/// What was read and every finding of the files under `paths`, which `listing` lists and reads.
fn survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    listing: &Listing,
    workloads: Vec<runtime::Workload>,
    adapter_error: Option<String>,
    catalog: &QuestionCatalog,
) -> Result<SurveyOutput<Kind, Read>> {
    let files = &listing.files;
    let workspace = (files
        .iter()
        .any(|(_, language)| *language == SupportLang::Python))
    .then(|| ledger::rules_workspace(paths, tree))
    .transpose()?;
    let surveyed: Vec<Surveyed> = (files.par_iter())
        .filter_map(|(path, language)| {
            let ws = workspace
                .as_ref()
                .filter(|ws| *language == SupportLang::Python || ws.catalog.project.is_some());
            read(path, *language, tree, listing, ws)
        })
        .collect();
    let mut findings = rule_findings(&surveyed);
    findings.extend(python_findings(&surveyed));
    let by_path: BTreeMap<_, _> = surveyed
        .iter()
        .map(|file| (file.shown.as_str(), file))
        .collect();
    let mut entities = BTreeSet::new();
    for place in findings.iter().flat_map(|finding| &finding.places) {
        let Some(file) = by_path.get(place.path.as_str()) else {
            continue;
        };
        let Some(line) = place.line else { continue };
        let Some(facts) = &file.python else { continue };
        if let Some((name, _, _)) = facts
            .functions
            .iter()
            .filter(|(_, start, end)| (*start..=*end).contains(&line))
            .min_by_key(|(_, start, end)| end - start)
        {
            entities.insert((format!("{}:{name}", file.named), file.named.clone()));
        }
    }
    for function in workloads.iter().flat_map(|workload| &workload.functions) {
        if function.cpu_share >= 0.05
            && !Path::new(&function.file).is_absolute()
            && !function.file.starts_with(['<', '~'])
        {
            let file = function.file.clone();
            entities.insert((format!("{file}:{}", function.name), file));
        }
    }
    Ok(SurveyOutput {
        read: Read {
            files: surveyed.len(),
            python: surveyed.iter().filter(|file| file.python.is_some()).count(),
            lines: surveyed.iter().map(|file| file.lines).sum(),
            workloads,
            adapter_error,
        },
        findings: ledger::named(findings),
        checklist: catalog.checklist(entities, "function"),
    })
}
