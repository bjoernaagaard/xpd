//! Reports column lineage and the same-file uses that can be resolved without running Python.

mod callers;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::{ArgGroup, Args};
use serde::Serialize;

use crate::graph_diagnostic::Level;
use crate::outline;
use crate::output::Format;
use crate::steps::{self, Pipeline, Step};
use crate::syntax::{self, Node, string_literal};

#[derive(Debug, Args)]
#[command(
    group(ArgGroup::new("change").required(true).args(["column", "transformation"])),
    after_help = "Reports direct callers in FILE. Check unresolved boundaries for runtime and external uses."
)]
pub(crate) struct ImpactArgs {
    /// Python file containing the pipeline and any callers to inspect.
    file: PathBuf,
    /// Name of the top-level function that runs the pipeline.
    #[arg(long)]
    entry: String,
    /// Name of an observed source column or a column produced in the pipeline.
    #[arg(long)]
    column: Option<String>,
    /// Name of a step in the pipeline, as shown by graph.
    #[arg(long)]
    transformation: Option<String>,
    /// Output format. Default: text on a terminal, JSON otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

impl ImpactArgs {
    pub(crate) fn run(&self, default: Format) -> Result<()> {
        let query = match (&self.column, &self.transformation) {
            (Some(column), None) => Change::Column(column),
            (None, Some(step)) => Change::Transformation(step),
            _ => anyhow::bail!("provide exactly one of --column or --transformation"),
        };
        let source = fs::read_to_string(&self.file)
            .with_context(|| format!("cannot read {}", self.file.display()))?;
        let root = syntax::parse(&source);
        if let Some(error) = root.root().dfs().find(|n| n.is_error() || n.is_missing()) {
            anyhow::bail!(
                "{}:{}: cannot parse Python",
                self.file.display(),
                line(&error)
            );
        }
        let report = report(&self.file, &root, &self.entry, query)?;
        match self.format.unwrap_or(default) {
            Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            Format::Text => print!("{}", report.text()?),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
enum Change<'a> {
    Column(&'a str),
    Transformation(&'a str),
}

#[derive(Debug, Clone, Serialize)]
struct Hop {
    transformation: String,
    column: String,
    line: usize,
}

type Paths = BTreeMap<String, Vec<Hop>>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Boundary {
    line: usize,
    code: &'static str,
    message: String,
}

impl Boundary {
    fn new(line: usize, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            line,
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    file: &'a Path,
    entry: &'a str,
    line: usize,
    change: Change<'a>,
    producers: Vec<Hop>,
    outputs: Paths,
    callers: Vec<callers::Caller>,
    unresolved: BTreeSet<Boundary>,
}

impl Report<'_> {
    fn text(&self) -> Result<String> {
        let mut out = String::new();
        let file = self.file.display();
        let (kind, name) = match self.change {
            Change::Column(name) => ("column", name),
            Change::Transformation(name) => ("transformation", name),
        };
        writeln!(
            out,
            "{file}:{}: impact of {kind} {name:?} in {}()",
            self.line, self.entry
        )?;
        for hop in &self.producers {
            writeln!(
                out,
                "{file}:{}: producer {} [{}]",
                hop.line, hop.transformation, hop.column
            )?;
        }
        if self.outputs.is_empty() {
            writeln!(out, "No affected output columns resolved.")?;
        }
        for (column, path) in &self.outputs {
            let path: Vec<_> = path
                .iter()
                .map(|hop| {
                    format!(
                        "{} [{}] ({file}:{})",
                        hop.transformation, hop.column, hop.line
                    )
                })
                .collect();
            writeln!(out, "Output {column:?}: {}", path.join(" -> "))?;
        }
        for caller in &self.callers {
            writeln!(
                out,
                "{file}:{}: {} calls {}",
                caller.line, caller.name, caller.call
            )?;
            for (line, columns) in &caller.uses {
                writeln!(
                    out,
                    "{file}:{line}: uses {}",
                    columns.iter().cloned().collect::<Vec<_>>().join(", ")
                )?;
            }
            writeln!(out, "  Verify: {}", caller.verification)?;
        }
        for item in &self.unresolved {
            writeln!(
                out,
                "{file}:{}: unresolved [{}] {}",
                item.line, item.code, item.message
            )?;
        }
        Ok(out)
    }
}

fn report<'a>(
    file: &'a Path,
    root: &syntax::Root,
    entry: &'a str,
    change: Change<'a>,
) -> Result<Report<'a>> {
    let pipeline = steps::read_entry(root, entry)
        .with_context(|| format!("{}: no top-level function named {entry:?}", file.display()))?;
    ensure!(
        !pipeline.steps.is_empty(),
        "no pipeline stages resolved for {entry:?}"
    );
    let module = outline::python_module(root);
    let mut unresolved = BTreeSet::from([Boundary::new(
        pipeline.line,
        "external-callers",
        "Look for imports and callers outside this file.",
    )]);
    let shapes = return_shapes(&pipeline, &module);
    let trace = lineage(&pipeline, &shapes, change, &mut unresolved)?;
    let outputs = trace
        .frame
        .into_iter()
        .filter_map(|(name, column)| column.path.map(|path| (name, path)))
        .collect();
    let mut callers = callers::find(
        root,
        &module,
        entry,
        pipeline.line,
        &outputs,
        &mut unresolved,
    );
    for caller in &mut callers {
        caller.verification = verification(caller, &trace.verification_inputs);
    }
    Ok(Report {
        file,
        entry,
        line: pipeline.line,
        change,
        producers: trace.producers,
        outputs,
        callers,
        unresolved,
    })
}

fn verification(caller: &callers::Caller, inputs: &BTreeSet<String>) -> String {
    let columns: BTreeSet<_> = caller.uses.values().flatten().collect();
    let names: Vec<_> = columns.iter().map(|column| format!("{column:?}")).collect();
    let site = if caller.name == "(module)" {
        "the module"
    } else {
        &caller.name
    };
    let perturbation = if inputs.is_empty() {
        "Unknown input to perturb. Find an input row that changes the producer, or compare the old and new producer on the same rows.".to_owned()
    } else {
        let names: Vec<_> = inputs.iter().map(|column| format!("{column:?}")).collect();
        format!(
            "Inferred input candidates: {}. Change one of these columns to a different valid value in one row of {}, keeping all other input values fixed.",
            names.join(", "),
            caller.input
        )
    };
    format!(
        "{perturbation} Run {site} on both inputs and compare {} at the listed reads, matching the same rows. Inferred: the caller can observe a change if the producer changes. Exact values and direction are unknown.",
        names.join(", "),
    )
}

#[derive(Debug)]
struct ShapeAt {
    offset: usize,
    shape: Shape,
}

#[derive(Debug)]
enum Shape {
    Project(Vec<String>),
    Rename {
        line: usize,
        columns: Vec<(String, String)>,
    },
    Unresolved {
        line: usize,
        expression: String,
    },
}

fn return_shapes(
    pipeline: &Pipeline,
    module: &outline::Module<'_>,
) -> BTreeMap<String, Vec<ShapeAt>> {
    let mut shapes = BTreeMap::new();
    for function in &module.functions {
        if function.name != pipeline.name && !pipeline.steps.iter().any(|s| s.name == function.name)
        {
            continue;
        }
        if let Some(body) = function.definition.field("body") {
            shapes.insert(function.name.clone(), return_shape(&body));
        }
    }
    shapes
}

fn return_shape(body: &Node<'_>) -> Vec<ShapeAt> {
    let mut returns = body.dfs().filter(|n| n.kind() == "return_statement");
    let Some(mut value) = returns.next().and_then(|n| n.named_children().next()) else {
        return Vec::new();
    };
    if returns.next().is_some() {
        return Vec::new();
    }
    let mut shapes = Vec::new();
    let binders = crate::names::Binders::default();
    loop {
        if value.kind() == "subscript" {
            if let Some(columns) = value.field("subscript").and_then(|n| literal_columns(&n))
                && let Some(frame) = value.field("value")
            {
                shapes.push(ShapeAt {
                    offset: frame.range().end,
                    shape: Shape::Project(columns),
                });
                value = frame;
                continue;
            }
            let Some(frame) = value.field("value") else {
                break;
            };
            shapes.push(ShapeAt {
                offset: frame.range().end,
                shape: unknown_shape(&value, line(&value)),
            });
            value = frame;
        } else if value.kind() == "call" {
            let Some(target) = value.field("function") else {
                break;
            };
            let Some(method) = target.field("attribute") else {
                break;
            };
            let Some(frame) = target.field("object") else {
                break;
            };
            let shape = match method.text().as_ref() {
                "rename" => Some(literal_rename(&value).map_or_else(
                    || unknown_shape(&value, line(&method)),
                    |columns| Shape::Rename {
                        line: line(&method),
                        columns,
                    },
                )),
                "drop" | "set_axis" | "filter" => Some(unknown_shape(&value, line(&method))),
                "assign" | "pipe" | "join" | "copy" => None,
                _ => break,
            };
            if let Some(shape) = shape {
                shapes.push(ShapeAt {
                    offset: method.range().start,
                    shape,
                });
            }
            value = frame;
        } else if value.kind() == "parenthesized_expression" {
            let Some(inner) = value.named_children().next() else {
                break;
            };
            value = inner;
        } else if value.kind() == "identifier"
            && let Some(statement) = crate::names::statement_around(value.clone())
            && let Some(origin) = binders.origin_before(&statement, &value.text())
            && let crate::names::Binding::Assigned(Some(assigned)) = origin.binding
            && assigned.range().start < value.range().start
        {
            value = assigned;
        } else {
            break;
        }
    }
    shapes.reverse();
    shapes
}

fn unknown_shape(value: &Node<'_>, line: usize) -> Shape {
    Shape::Unresolved {
        line,
        expression: value.text().into_owned(),
    }
}

fn literal_rename(value: &Node<'_>) -> Option<Vec<(String, String)>> {
    let args = value.field("arguments")?;
    let mut args = args.named_children().filter(|n| n.kind() != "comment");
    let columns = args.next()?;
    if columns.field("name")?.text() != "columns" || args.next().is_some() {
        return None;
    }
    let dictionary = columns.field("value")?;
    if dictionary.kind() != "dictionary" {
        return None;
    }
    dictionary
        .named_children()
        .filter(|n| n.kind() != "comment")
        .map(|pair| {
            Some((
                string_literal(&pair.field("key")?)?,
                string_literal(&pair.field("value")?)?,
            ))
        })
        .collect()
}

fn literal_columns(node: &Node<'_>) -> Option<Vec<String>> {
    match node.kind().as_ref() {
        "string" => string_literal(node).map(|name| vec![name]),
        "list" => node
            .named_children()
            .filter(|n| n.kind() != "comment")
            .map(|n| string_literal(&n))
            .collect(),
        _ => None,
    }
}

fn lineage<'a>(
    pipeline: &Pipeline,
    shapes: &BTreeMap<String, Vec<ShapeAt>>,
    change: Change<'a>,
    unresolved: &mut BTreeSet<Boundary>,
) -> Result<Trace<'a>> {
    let mut trace = Trace {
        change,
        frame: pipeline
            .source
            .iter()
            .map(|name| (name.clone(), Column::source(name)))
            .collect(),
        producers: Vec::new(),
        verification_inputs: BTreeSet::new(),
    };
    if let Change::Column(name) = change
        && is_source_column(name, pipeline, shapes)
    {
        trace.record(
            Hop {
                transformation: "pipeline input".to_owned(),
                column: name.to_owned(),
                line: pipeline.line,
            },
            Column::source(name),
        );
    }
    let has_entry_step = pipeline.steps.iter().any(|step| step.name == pipeline.name);
    let entry_shapes = if has_entry_step {
        &[][..]
    } else {
        shape_list(shapes, &pipeline.name)
    };
    let mut entry_shapes = entry_shapes.iter().peekable();
    for step in &pipeline.steps {
        trace.shapes_before(&mut entry_shapes, step.call_offset, ".rename", unresolved);
        trace.step(step, shape_list(shapes, &step.name), unresolved);
    }
    for shape in entry_shapes {
        trace.shape(".rename", &shape.shape, unresolved);
    }
    for finding in pipeline.findings.iter().filter(|f| f.level == Level::Error) {
        unresolved.insert(Boundary::new(finding.line, "dataflow", &finding.message));
    }
    match change {
        Change::Column(name) => {
            ensure!(
                !trace.producers.is_empty(),
                "no column {name:?} resolved in {}",
                pipeline.name
            );
            ensure!(
                trace.producers.len() == 1,
                "column {name:?} has multiple producers. Select a step with --transformation"
            );
        }
        Change::Transformation(name) => {
            let step = pipeline
                .steps
                .iter()
                .find(|s| s.name == name)
                .with_context(|| format!("no step {name:?} resolved in {}", pipeline.name))?;
            if trace.producers.is_empty() {
                unresolved.insert(Boundary::new(
                    step.line,
                    "unknown-step-output",
                    format!("Identify the output columns of {name}."),
                ));
            }
        }
    }
    Ok(trace)
}

fn is_source_column(
    name: &str,
    pipeline: &Pipeline,
    shapes: &BTreeMap<String, Vec<ShapeAt>>,
) -> bool {
    let mut observed = pipeline.source.iter().any(|source| source == name);
    if pipeline
        .steps
        .iter()
        .flat_map(|step| &step.writes)
        .any(|write| write == name)
    {
        return false;
    }
    for at in shapes.values().flatten() {
        match &at.shape {
            Shape::Rename { columns, .. } => {
                if columns.iter().any(|(_, to)| to == name) {
                    return false;
                }
                observed |= columns.iter().any(|(from, _)| from == name);
            }
            Shape::Project(columns) => observed |= columns.iter().any(|column| column == name),
            Shape::Unresolved { .. } => {}
        }
    }
    observed
}

fn shape_list<'a>(shapes: &'a BTreeMap<String, Vec<ShapeAt>>, name: &str) -> &'a [ShapeAt] {
    shapes.get(name).map_or(&[], Vec::as_slice)
}

#[derive(Clone, Default)]
struct Column {
    path: Option<Vec<Hop>>,
    sources: BTreeSet<String>,
}

impl Column {
    fn source(name: &str) -> Self {
        Self {
            path: None,
            sources: BTreeSet::from([name.to_owned()]),
        }
    }
}

struct Trace<'a> {
    change: Change<'a>,
    frame: BTreeMap<String, Column>,
    producers: Vec<Hop>,
    verification_inputs: BTreeSet<String>,
}

impl Trace<'_> {
    fn record(&mut self, hop: Hop, mut column: Column) {
        let changed = match self.change {
            Change::Column(name) => name == hop.column,
            Change::Transformation(name) => name == hop.transformation,
        };
        if changed {
            self.producers.push(hop.clone());
            self.verification_inputs
                .extend(column.sources.iter().cloned());
            column.path = Some(Vec::new());
        }
        let name = hop.column.clone();
        if let Some(path) = &mut column.path {
            path.push(hop);
        }
        self.frame.insert(name, column);
    }

    fn step(&mut self, step: &Step, shapes: &[ShapeAt], unresolved: &mut BTreeSet<Boundary>) {
        let mut shapes = shapes.iter().peekable();
        let inputs = self.frame.clone();
        for write in &step.column_writes {
            self.shapes_before(&mut shapes, write.offset, &step.name, unresolved);
            let reads = if write.reads_current_frame {
                &self.frame
            } else {
                &inputs
            };
            let dependencies = write.reads.iter().filter_map(|name| reads.get(name));
            let column = Column {
                path: dependencies.clone().find_map(|column| column.path.clone()),
                sources: dependencies
                    .flat_map(|column| column.sources.iter().cloned())
                    .collect(),
            };
            self.record(
                Hop {
                    transformation: step.name.clone(),
                    column: write.name.clone(),
                    line: write.line,
                },
                column,
            );
        }
        for shape in shapes {
            self.shape(&step.name, &shape.shape, unresolved);
        }
        if step.computed {
            unresolved.insert(Boundary::new(
                step.line,
                "computed-columns",
                format!(
                    "Resolve the runtime column names or return shape in {}.",
                    step.name
                ),
            ));
        }
    }

    fn shapes_before(
        &mut self,
        shapes: &mut std::iter::Peekable<std::slice::Iter<'_, ShapeAt>>,
        offset: usize,
        step: &str,
        unresolved: &mut BTreeSet<Boundary>,
    ) {
        while let Some(at) = shapes.next_if(|at| at.offset < offset) {
            self.shape(step, &at.shape, unresolved);
        }
    }

    fn shape(&mut self, step: &str, shape: &Shape, unresolved: &mut BTreeSet<Boundary>) {
        match shape {
            Shape::Project(columns) => self.frame.retain(|column, _| columns.contains(column)),
            Shape::Unresolved { line, expression } => {
                self.frame.clear();
                unresolved.insert(Boundary::new(
                    *line,
                    "output-shape",
                    format!("Resolve which columns {expression} returns."),
                ));
            }
            Shape::Rename { line, columns } => {
                let before: Vec<_> = columns
                    .iter()
                    .map(|(from, to)| (to, self.frame.remove(from)))
                    .collect();
                for (to, column) in before {
                    self.record(
                        Hop {
                            transformation: step.to_owned(),
                            column: to.clone(),
                            line: *line,
                        },
                        column.unwrap_or_default(),
                    );
                }
            }
        }
    }
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}
