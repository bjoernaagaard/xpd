mod contract;
mod evidence;
mod lazy;

use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, ensure};
use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::graph_diagnostic::Level;
use crate::outline;
use crate::output::Format;
use crate::steps::{self, Link, Pipeline, StepKind};
use crate::syntax;
use contract::{Contract, Materialization};
use evidence::{Evidence, Observations};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Source {
    Pandas,
    Polars,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Target {
    Polars,
}

#[derive(Debug, Args)]
pub(crate) struct MapArgs {
    /// Python file containing the entry function.
    file: PathBuf,
    /// Name of a top-level function in the file.
    #[arg(long)]
    entry: String,
    /// Source library.
    #[arg(long, value_enum)]
    from: Source,
    /// Target library.
    #[arg(long, value_enum)]
    to: Target,
    /// JSON declaring the schemas, row identity and materialization boundary.
    #[arg(long)]
    contract: Option<PathBuf>,
    /// Output format. Default: text on a terminal, JSON otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

impl MapArgs {
    pub(crate) fn run(&self, default: Format) -> Result<ExitCode> {
        ensure!(
            !matches!(self.from, Source::Polars) || self.contract.is_none(),
            "--contract is only supported for pandas to Polars"
        );
        let contract = self.contract.as_deref().map(Contract::read).transpose()?;
        let report = match (self.from, self.to) {
            (Source::Pandas, Target::Polars) => plan(
                &self.file,
                &self.entry,
                "pandas",
                "polars",
                contract.as_ref(),
            )?,
            (Source::Polars, Target::Polars) => lazy::plan(&self.file, &self.entry)?,
        };
        match self.format.unwrap_or(default) {
            Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            Format::Text => print!("{}", report.text()?),
        }
        Ok(if report.unresolved.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        })
    }
}

#[derive(Debug, Serialize)]
struct Stage {
    name: String,
    line: usize,
    reads: Vec<String>,
    writes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    file: &'a Path,
    entry: &'a str,
    line: usize,
    from: &'a str,
    to: &'a str,
    status: &'static str,
    input_columns_observed: Vec<String>,
    contract: Option<&'a Contract>,
    stages: Vec<Stage>,
    dependencies: Vec<Link>,
    observed: Vec<Evidence>,
    proposed: Vec<Evidence>,
    unresolved: Vec<Evidence>,
}

impl Report<'_> {
    fn text(&self) -> Result<String> {
        let mut text = String::new();
        writeln!(
            text,
            "{}:{}: {} ({} to {}, {})",
            self.file.display(),
            self.line,
            self.entry,
            self.from,
            self.to,
            self.status
        )?;
        writeln!(
            text,
            "Observed input columns: {}",
            self.input_columns_observed.join(", ")
        )?;
        if let Some(contract) = self.contract {
            writeln!(text, "Contract: {}", serde_json::to_string(contract)?)?;
        }
        for stage in &self.stages {
            writeln!(
                text,
                "{}: {} (reads: {}; writes: {})",
                stage.line,
                stage.name,
                stage.reads.join(", "),
                stage.writes.join(", ")
            )?;
        }
        for link in &self.dependencies {
            writeln!(
                text,
                "{} -> {} ({})",
                link.from,
                link.to,
                link.columns.join(", ")
            )?;
        }
        for (label, items) in [
            ("observed", &self.observed),
            ("proposed", &self.proposed),
            ("unresolved", &self.unresolved),
        ] {
            for item in items {
                writeln!(
                    text,
                    "{}: {label} [{}] {}",
                    item.line, item.code, item.message
                )?;
            }
        }
        Ok(text)
    }
}

fn read_python(file: &Path) -> Result<syntax::Root> {
    let source =
        fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
    let root = syntax::parse(&source);
    if let Some(error) = root
        .root()
        .dfs()
        .find(|node| node.is_error() || node.is_missing())
    {
        anyhow::bail!(
            "{}:{}: cannot parse Python",
            file.display(),
            error.start_pos().line() + 1
        );
    }
    Ok(root)
}

fn plan<'a>(
    file: &'a Path,
    entry: &'a str,
    from: &'a str,
    to: &'a str,
    contract: Option<&'a Contract>,
) -> Result<Report<'a>> {
    let root = read_python(file)?;
    let pipeline = steps::read_entry(&root, entry)
        .with_context(|| format!("{}: no top-level function named {entry:?}", file.display()))?;
    let observations = observe(&root, &pipeline, contract)?;
    Ok(Report {
        file,
        entry,
        line: pipeline.line,
        from,
        to,
        status: if observations.unresolved.is_empty() {
            "complete"
        } else {
            "partial"
        },
        input_columns_observed: pipeline.source,
        contract,
        stages: pipeline
            .steps
            .into_iter()
            .map(|stage| Stage {
                name: stage.name,
                line: stage.line,
                reads: stage.reads,
                writes: stage.writes,
            })
            .collect(),
        dependencies: pipeline.links,
        observed: observations.observed,
        proposed: observations.proposed,
        unresolved: observations.unresolved,
    })
}

fn observe(
    root: &syntax::Root,
    pipeline: &Pipeline,
    contract: Option<&Contract>,
) -> Result<Observations> {
    let mut observations = Observations::default();
    let stages: Vec<_> = pipeline
        .steps
        .iter()
        .filter(|stage| stage.kind == StepKind::Function)
        .map(|stage| stage.name.as_str())
        .collect();
    let module = outline::python_module(root);
    let pandas = crate::names::LibraryNames::new("pandas");
    let module_values: std::collections::BTreeMap<_, _> = module
        .assignments
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    let constants: BTreeSet<_> = module_values
        .into_iter()
        .filter(|(_, value)| {
            matches!(
                value.kind().as_ref(),
                "integer" | "float" | "true" | "false" | "none"
            ) || syntax::string_literal(value).is_some()
        })
        .map(|(name, _)| name)
        .collect();
    for name in std::iter::once(pipeline.name.as_str())
        .chain(stages.iter().copied().filter(|name| *name != pipeline.name))
    {
        let function = module
            .functions
            .iter()
            .rfind(|function| function.name == name)
            .context("pipeline step has no definition")?;
        if function.parameters.len() > 1 {
            observations.block(function.definition.start_pos().line() + 1, "extra-inputs", "Determine whether additional arguments are scalar values or carry data and alignment requirements.");
        }
        for decorator in &function.decorators {
            observations.block(
                decorator.node.start_pos().line() + 1,
                "decorator",
                "Check how this decorator changes the inputs, return value or side effects.",
            );
        }
        observations.function(&function.definition, &stages, &pandas, &constants);
    }
    if pipeline.steps.is_empty() {
        observations.block(
            pipeline.line,
            "dataflow",
            "Identify the frame input and its column dependencies. No frame stages were resolved.",
        );
    }
    for stage in &pipeline.steps {
        if stage.mutates_input {
            observations.block(stage.line, "input-mutation", "Determine whether this stage's writes are visible outside the pipeline and preserve those effects at the boundary.");
        }
        if stage.computed {
            observations.block(
                stage.line,
                "computed-columns",
                "Identify the columns selected or assigned at runtime.",
            );
        }
    }
    for finding in &pipeline.findings {
        if finding.level == Level::Error {
            observations.block(finding.line, "dataflow", &finding.message);
        }
    }
    observe_contract(&mut observations, pipeline, contract)?;
    observations.suggest(pipeline.line, "output-contract", "Reuse intermediate expressions while preserving every output column and its declared type and position.");
    observations.suggest(pipeline.line, "verification", "Run xpd equiv with the same contract and the same inputs on both implementations. Inspect the native code and optimized plan.");
    Ok(observations)
}

fn observe_contract(
    observations: &mut Observations,
    pipeline: &Pipeline,
    contract: Option<&Contract>,
) -> Result<()> {
    if let Some(contract) = contract {
        for name in &pipeline.source {
            ensure!(
                contract
                    .input_schema
                    .iter()
                    .any(|column| column.name == *name),
                "input schema does not declare observed column {name:?}",
            );
        }
        for stage in &pipeline.steps {
            for name in &stage.writes {
                if !contract
                    .output_schema
                    .iter()
                    .any(|column| column.name == *name)
                {
                    observations.block(stage.line, "output-schema", &format!("Stage writes {name:?}, which is absent from the declared output. Establish where it is dropped or include it in the contract."));
                }
            }
        }
        let boundary = match contract.materialization {
            Materialization::Lazy => {
                "Keep the plan lazy through the return boundary. Collect only in the consumer or the parity script."
            }
            Materialization::Eager => {
                "Use a lazy scan for supported file inputs. Materialize once at the declared eager output boundary."
            }
        };
        observations.suggest(pipeline.line, "materialization", boundary);
    } else {
        observations.block(pipeline.line, "contract", "Supply --contract with ordered input and output schemas, row identity and the caller's materialization boundary.");
    }
    Ok(())
}
