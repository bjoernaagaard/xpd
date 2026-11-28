mod static_review;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, ensure};
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::Filters;
use crate::ledger::gix_source::Listing;
use crate::ledger::{
    self, AdapterPolicy, FindingPlace, Goal, LedgerFinding, QuestionCatalog, Select, SurveyOutput,
    Tree, WorkflowId,
};
use crate::output::Format;
use crate::syntax::{self, Node};
use crate::workspace::Workspace;

const OPT_CATALOG: &str = include_str!("../rules/xpd/opt-review.yml");
const SOLVER_FACTS: &str = include_str!("../py/solver_facts.py");

#[derive(Deserialize, Serialize)]
struct OptCatalog {
    #[serde(flatten)]
    checklist: QuestionCatalog,
    rules: BTreeMap<String, Kind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Group {
    Formulation,
    Solving,
    PortfolioValidation,
    PackRules,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    NonDcpProblem,
    UncheckedSolverStatus,
    ConstraintUnitMismatch,
    FitOnHoldout,
    PackRule,
}

impl ledger::LedgerKind for Kind {
    type Group = Group;
    const WORKFLOW: WorkflowId = WorkflowId::OptReview;
    const GROUPS: &[Group] = &[
        Group::Formulation,
        Group::Solving,
        Group::PortfolioValidation,
        Group::PackRules,
    ];
    const GOALS: &[Goal] = &[Goal::Design];

    fn group(self) -> Group {
        match self {
            Self::NonDcpProblem | Self::ConstraintUnitMismatch => Group::Formulation,
            Self::UncheckedSolverStatus => Group::Solving,
            Self::FitOnHoldout => Group::PortfolioValidation,
            Self::PackRule => Group::PackRules,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::Formulation => "Formulation",
            Group::Solving => "Solving",
            Group::PortfolioValidation => "Portfolio validation",
            Group::PackRules => "Pack rules",
        }
    }
}

pub(crate) fn opt_rule_detector(
    package: &str,
    rule: &str,
) -> Option<for<'a, 't> fn(&'a Node<'t>) -> Option<Node<'t>>> {
    match (package, rule) {
        ("cvxpy", "non-dcp-problem") => Some(static_review::non_dcp),
        ("cvxpy", "unchecked-solver-status") => Some(static_review::unchecked_status),
        ("cvxpy", "constraint-unit-mismatch") => Some(static_review::unit_mismatch),
        ("skfolio", "skfolio-fit-on-holdout") => Some(static_review::fit_on_holdout),
        _ => None,
    }
}

/// Review numerical optimization and portfolio problems.
#[derive(Debug, clap::Args)]
pub(crate) struct OptReviewArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// Project Python adapter or shell command that emits solver facts.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Print the evidence catalog without surveying files.
    #[arg(long)]
    questions: bool,
}

/// Runs a static opt-review survey without runtime adapters for `xpd audit`.
pub(crate) fn audit_survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
) -> Result<ledger::AuditSurvey> {
    Ok(ledger::AuditSurvey::from_output(survey_problems(
        paths,
        tree,
        top,
        commit,
        None,
        AdapterPolicy::BestEffort,
    )?))
}

impl OptReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            let catalog: OptCatalog = serde_saphyr::from_str(OPT_CATALOG)?;
            let format = self.select.format_override().unwrap_or(default);
            if matches!(format, Format::Json) {
                println!("{}", serde_json::to_string(&catalog)?);
                return Ok(ExitCode::SUCCESS);
            }
            let _ = catalog.checklist.print(format)?;
            println!(
                "Rules\n  {}",
                catalog
                    .rules
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n  ")
            );
            return Ok(ExitCode::SUCCESS);
        }
        self.select.print_with(
            default,
            self.adapter.as_deref(),
            survey_problems,
            |ledger| {
                let mut text = format!(
                    "xpd opt-review: {}, {} problems, {} checklist items\n",
                    ledger.count(),
                    ledger.read.problems.len(),
                    ledger.checklist.len(),
                );
                for problem in &ledger.read.problems {
                    let _ = writeln!(
                        text,
                        "  {}  {}:{}  {}  {}",
                        problem.id, problem.file, problem.line, problem.kind, problem.name
                    );
                }
                if !ledger.read.facts.is_null() {
                    let _ = writeln!(text, "Solver facts:\n{}", ledger.read.facts);
                }
                text
            },
        )
    }
}

#[derive(Debug, Serialize)]
struct Problem {
    id: String,
    file: String,
    line: usize,
    kind: &'static str,
    name: String,
    #[serde(skip)]
    front: bool,
}

#[derive(Debug, Serialize)]
struct OptRead {
    problems: Vec<Problem>,
    facts: Value,
}

fn validate_solver_facts(
    mut document: Value,
    problems: &[Problem],
    catalog: &QuestionCatalog,
) -> Result<Value> {
    let entries = document["problems"]
        .as_array_mut()
        .context("expected a problems array")?;
    let mut expected: BTreeMap<_, _> = problems.iter().map(|p| (p.id.as_str(), p.kind)).collect();
    for entry in &*entries {
        let id = entry["id"].as_str().context("problem entry has no ID")?;
        let kind = expected
            .remove(id)
            .with_context(|| format!("unknown or repeated problem ID {id}"))?;
        ensure!(
            entry["library"] == kind,
            "problem {id} must name library {kind}"
        );
        if let Some(reason) = entry.get("unmeasured") {
            ensure!(
                reason
                    .as_str()
                    .is_some_and(|reason| !reason.trim().is_empty())
                    && entry.get("facts").is_none(),
                "problem {id} must provide either facts or an unmeasured reason"
            );
            continue;
        }
        let facts = entry["facts"]
            .as_object()
            .with_context(|| format!("problem {id} has no facts object"))?;
        ensure!(!facts.is_empty(), "problem {id} has no facts");
        for (key, metric) in facts {
            ensure!(catalog.has_fact(key), "unknown solver fact {key} on {id}");
            ensure!(
                metric.get("value").is_some()
                    && metric["method"]
                        .as_str()
                        .is_some_and(|method| !method.trim().is_empty())
                    && (!metric["value"].is_null()
                        || metric["reason"]
                            .as_str()
                            .is_some_and(|reason| !reason.trim().is_empty())),
                "invalid solver fact {key} on {id}: expected value, method and a reason when unavailable"
            );
        }
    }
    ensure!(
        expected.is_empty(),
        "missing problem IDs: {}",
        expected.keys().copied().collect::<Vec<_>>().join(", ")
    );
    entries.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    Ok(document)
}

fn inventory_file(root: &Node<'_>, file: &str) -> Vec<Problem> {
    let mut occurrences = BTreeMap::new();
    root.dfs()
        .filter(|node| node.kind() == "call")
        .filter_map(|call| {
            let kind = static_review::problem_kind(&call)?;
            let name = static_review::definition_name(&call);
            let count = occurrences.entry((kind, name.clone())).or_insert(0);
            *count += 1;
            Some(Problem {
                id: ledger::finding_id(kind, &format!("{file}\n{name}"), *count),
                file: file.to_owned(),
                line: call.start_pos().line() + 1,
                kind,
                name,
                front: kind == "pymoo" && static_review::front_applies(&call),
            })
        })
        .collect()
}

fn scan_problem_findings(
    workspace: &Workspace,
    catalog: &OptCatalog,
    parsed: &Node<'_>,
    file: &str,
    absolute: &Path,
    text: &str,
) -> Vec<(LedgerFinding<Kind>, String)> {
    let linted = workspace.analyze(
        crate::source::File {
            path: Path::new(file),
            disk: Some(absolute),
        },
        text,
    );
    linted
        .found
        .into_iter()
        .filter_map(|found| {
            let range = found.range.clone();
            let issue = found.into_issue();
            let rule = issue.id.to_string();
            let kind = *catalog.rules.get(&rule)?;
            let anchor = parsed
                .dfs()
                .find(|node| node.range() == range && node.is_named())
                .map_or_else(
                    || text[range.clone()].to_owned(),
                    |node| {
                        let syntax = node
                            .dfs()
                            .filter(|part| part.children().len() == 0 && part.kind() != "comment")
                            .map(|part| part.text().into_owned())
                            .collect::<Vec<_>>()
                            .join(" ");
                        ledger::python_ast::scope_qualified_name(&node, &syntax)
                    },
                );
            let mut place = FindingPlace::file(file);
            if let Some(position) = issue.position {
                place.line = Some(position.line);
                place.column = Some(position.column);
            }
            let finding = LedgerFinding {
                kind,
                key: format!("{file}\n{rule}\n{anchor}"),
                places: vec![place],
                evidence: issue.message,
                weight: 1,
            };
            Some((finding, rule))
        })
        .collect()
}

fn survey_problems(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
    adapter: Option<&str>,
    _policy: AdapterPolicy,
) -> Result<SurveyOutput<Kind, OptRead>> {
    let catalog: OptCatalog = serde_saphyr::from_str(OPT_CATALOG)?;
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
    let mut problems = Vec::new();
    let mut findings = Vec::new();
    let mut source_rules = BTreeMap::new();
    for (path, language) in &listing.files {
        if *language != SupportLang::Python {
            continue;
        }
        let (Some(text), Some(absolute)) = (listing.text(path), listing.absolute(path)) else {
            continue;
        };
        let file = tree
            .tracked(&absolute)
            .or_else(|| {
                absolute
                    .strip_prefix(root)
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| tree.shown(path));
        let parsed = syntax::parse(&text);
        problems.extend(inventory_file(&parsed.root(), &file));
        for (finding, rule) in scan_problem_findings(
            &workspace,
            &catalog,
            &parsed.root(),
            &file,
            &absolute,
            &text,
        ) {
            source_rules.insert(finding.key.clone(), rule);
            findings.push(finding);
        }
    }
    let facts = adapter
        .map(|adapter| {
            ledger::adapter_runner::invoke_adapter(adapter, root, SOLVER_FACTS)
                .and_then(|document| validate_solver_facts(document, &problems, &catalog.checklist))
                .with_context(|| format!("opt-review adapter `{adapter}` failed"))
        })
        .transpose()?
        .unwrap_or(Value::Null);
    let mut checklist = Vec::new();
    for problem in &problems {
        checklist.extend(
            catalog
                .checklist
                .checklist(
                    BTreeSet::from([(problem.id.clone(), problem.file.clone())]),
                    problem.kind,
                )
                .into_iter()
                .filter(|item| match item.question.as_str() {
                    "portfolio-validation"
                    | "portfolio-implementation"
                    | "development-evidence" => problem.kind == "skfolio",
                    "front-quality" => problem.front,
                    _ => true,
                }),
        );
    }
    let mut findings = ledger::named(findings);
    for entry in &mut findings {
        entry.source_rule = source_rules.get(&entry.finding.key).cloned();
    }
    Ok(SurveyOutput {
        read: OptRead { problems, facts },
        findings,
        checklist,
    })
}
