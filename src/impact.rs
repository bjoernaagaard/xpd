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
use crate::steps::{self, Pipeline};
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
    let (outputs, producers) = lineage(&pipeline, &shapes, change, &mut unresolved)?;
    let mut callers = callers::find(
        root,
        &module,
        entry,
        pipeline.line,
        &outputs,
        &mut unresolved,
    );
    for caller in &mut callers {
        let columns: BTreeSet<_> = caller.uses.values().flatten().collect();
        let names: Vec<_> = columns.iter().map(|column| format!("{column:?}")).collect();
        let changed = match change {
            Change::Column(column) => format!("column {column:?}"),
            Change::Transformation(step) => format!("step {step:?}"),
        };
        let site = if caller.name == "(module)" {
            "the module"
        } else {
            &caller.name
        };
        caller.verification = format!(
            "Run {site} before and after the change. For {}, use the same {} rows that exercise the change to {changed}. Compare {} at the listed reads by row.",
            caller.call,
            caller.input,
            names.join(", "),
        );
    }
    Ok(Report {
        file,
        entry,
        line: pipeline.line,
        change,
        producers,
        outputs,
        callers,
        unresolved,
    })
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
) -> BTreeMap<String, Vec<Shape>> {
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

fn return_shape(body: &Node<'_>) -> Vec<Shape> {
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
                shapes.push(Shape::Project(columns));
                value = frame;
                continue;
            }
            shapes.push(unknown_shape(&value));
            break;
        } else if let Some((frame, columns)) = literal_rename(&value) {
            shapes.push(Shape::Rename {
                line: line(&value),
                columns,
            });
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
            if value.kind() == "call"
                && value
                    .field("function")
                    .and_then(|f| f.field("attribute"))
                    .is_some_and(|method| {
                        matches!(
                            method.text().as_ref(),
                            "rename" | "drop" | "set_axis" | "filter"
                        )
                    })
            {
                shapes.push(unknown_shape(&value));
            }
            break;
        }
    }
    shapes.reverse();
    shapes
}

fn unknown_shape(value: &Node<'_>) -> Shape {
    Shape::Unresolved {
        line: line(value),
        expression: value.text().into_owned(),
    }
}

fn literal_rename<'r>(value: &Node<'r>) -> Option<(Node<'r>, Vec<(String, String)>)> {
    if value.kind() != "call" {
        return None;
    }
    let target = value.field("function")?;
    if target.field("attribute")?.text() != "rename" {
        return None;
    }
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
    let pairs: Option<Vec<_>> = dictionary
        .named_children()
        .filter(|n| n.kind() != "comment")
        .map(|pair| {
            Some((
                string_literal(&pair.field("key")?)?,
                string_literal(&pair.field("value")?)?,
            ))
        })
        .collect();
    Some((target.field("object")?, pairs?))
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

fn lineage(
    pipeline: &Pipeline,
    shapes: &BTreeMap<String, Vec<Shape>>,
    change: Change<'_>,
    unresolved: &mut BTreeSet<Boundary>,
) -> Result<(Paths, Vec<Hop>)> {
    let mut trace = Trace {
        change,
        paths: BTreeMap::new(),
        producers: Vec::new(),
    };
    if let Change::Column(name) = change
        && !pipeline.steps.iter().any(|s| s.writes.iter().any(|w| w == name))
        && !shapes.values().flatten().any(|shape| matches!(shape,
            Shape::Rename { columns, .. } if columns.iter().any(|(_, to)| to == name)))
        && (pipeline.source.iter().any(|source| source == name)
            || shapes.values().flatten().any(|shape| matches!(shape,
                Shape::Project(columns) if columns.iter().any(|column| column == name))
                || matches!(shape, Shape::Rename { columns, .. } if columns.iter().any(|(from, _)| from == name))))
    {
        trace.record(Hop { transformation: "pipeline input".to_owned(), column: name.to_owned(), line: pipeline.line }, None);
    }
    for step in &pipeline.steps {
        let inputs = trace.paths.clone();
        for write in &step.column_writes {
            let reads = if write.reads_current_frame {
                &trace.paths
            } else {
                &inputs
            };
            let parent = write.reads.iter().find_map(|name| reads.get(name)).cloned();
            trace.record(
                Hop {
                    transformation: step.name.clone(),
                    column: write.name.clone(),
                    line: write.line,
                },
                parent,
            );
        }
        trace.shape(&step.name, shapes, unresolved);
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
    if !pipeline.steps.iter().any(|step| step.name == pipeline.name) {
        trace.shape(&pipeline.name, shapes, unresolved);
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
    Ok((trace.paths, trace.producers))
}

struct Trace<'a> {
    change: Change<'a>,
    paths: Paths,
    producers: Vec<Hop>,
}

impl Trace<'_> {
    fn record(&mut self, hop: Hop, parent: Option<Vec<Hop>>) {
        let changed = match self.change {
            Change::Column(name) => name == hop.column,
            Change::Transformation(name) => name == hop.transformation,
        };
        let mut path = if changed {
            self.producers.push(hop.clone());
            Vec::new()
        } else if let Some(parent) = parent {
            parent
        } else {
            self.paths.remove(&hop.column);
            return;
        };
        let column = hop.column.clone();
        path.push(hop);
        self.paths.insert(column, path);
    }

    fn shape(
        &mut self,
        step: &str,
        shapes: &BTreeMap<String, Vec<Shape>>,
        unresolved: &mut BTreeSet<Boundary>,
    ) {
        for shape in shapes.get(step).into_iter().flatten() {
            match shape {
                Shape::Project(columns) => self.paths.retain(|column, _| columns.contains(column)),
                Shape::Unresolved { line, expression } => {
                    self.paths.clear();
                    unresolved.insert(Boundary::new(
                        *line,
                        "output-shape",
                        format!("Resolve which columns {expression} returns."),
                    ));
                }
                Shape::Rename { line, columns } => {
                    let before: Vec<_> = columns
                        .iter()
                        .map(|(from, to)| (to, self.paths.remove(from)))
                        .collect();
                    for (to, parent) in before {
                        self.record(
                            Hop {
                                transformation: step.to_owned(),
                                column: to.clone(),
                                line: *line,
                            },
                            parent,
                        );
                    }
                }
            }
        }
    }
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}
