use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};

use crate::names::LibraryNames;
use crate::outline;
use crate::syntax::{Node, string_literal};

use super::evidence::Observations;
use super::{Report, Stage, read_python};

pub(super) fn plan<'a>(file: &'a Path, entry: &'a str) -> Result<Report<'a>> {
    let root = read_python(file)?;
    let module = outline::python_module(&root);
    let function = module
        .functions
        .iter()
        .rfind(|function| function.name == entry)
        .with_context(|| format!("{}: no top-level function named {entry:?}", file.display()))?;
    let mut planner = Planner::default();
    let line = line(&function.definition);
    if !function.decorators.is_empty()
        || function
            .definition
            .children()
            .any(|node| node.kind() == "async")
    {
        planner.evidence.block(
            line,
            "entry-boundary",
            "Resolve how decorators or async execution affect the call boundary.",
        );
    }
    if let Some(body) = function.definition.field("body") {
        planner.body(&body, &LibraryNames::new("polars"));
    }
    planner.finish(line);
    Ok(Report {
        file,
        entry,
        line,
        from: "polars",
        to: "polars",
        status: if planner.evidence.unresolved.is_empty() {
            "complete"
        } else {
            "partial"
        },
        input_columns_observed: planner.columns.into_iter().collect(),
        contract: None,
        stages: planner.stages,
        dependencies: Vec::new(),
        observed: planner.evidence.observed,
        proposed: planner.evidence.proposed,
        unresolved: planner.evidence.unresolved,
    })
}

#[derive(Clone, Copy)]
enum Value {
    Eager,
    Lazy,
    Collected,
    Consumed,
}

#[derive(Default)]
struct Planner {
    evidence: Observations,
    stages: Vec<Stage>,
    columns: BTreeSet<String>,
    read: Option<usize>,
    pushdown: bool,
    boundary: Option<(usize, &'static str)>,
}

impl Planner {
    fn body<'tree>(&mut self, body: &Node<'tree>, polars: &LibraryNames<'tree>) {
        let mut current = None;
        for statement in body
            .named_children()
            .filter(|node| node.kind() != "comment")
        {
            let value = statement.named_children().next();
            match (statement.kind().as_ref(), value) {
                ("expression_statement", Some(value)) if text_literal(&value).is_some() => {}
                ("expression_statement", Some(value)) if value.kind() == "assignment" => {
                    let result = value
                        .field("left")
                        .filter(|target| target.kind() == "identifier")
                        .zip(value.field("right"))
                        .and_then(|(target, rhs)| {
                            self.value(&rhs, current.as_ref(), polars)
                                .map(|value| (target.text().into_owned(), value))
                        });
                    if result.is_none() {
                        self.unknown(&statement);
                    }
                    current = result;
                }
                ("expression_statement", Some(value))
                    if matches!(
                        self.value(&value, current.as_ref(), polars),
                        Some(Value::Consumed)
                    ) =>
                {
                    current = None;
                }
                ("return_statement", Some(value)) => {
                    match self.value(&value, current.as_ref(), polars) {
                        Some(Value::Eager) => {
                            self.boundary = Some((
                                line(&statement),
                                "Collect once at the return to preserve the DataFrame result.",
                            ));
                        }
                        Some(Value::Lazy) => {
                            self.evidence.block(
                                line(&statement),
                                "file-lifetime",
                                "Confirm that callers can defer reading the file until they collect.",
                            );
                            self.boundary = Some((
                                line(&statement),
                                "Keep the LazyFrame return. Collect at the existing consumer when verifying.",
                            ));
                        }
                        Some(Value::Collected | Value::Consumed) => {}
                        None => self.unknown(&statement),
                    }
                    break;
                }
                ("return_statement", None) if self.boundary.is_some() => break,
                _ => {
                    self.unknown(&statement);
                    current = None;
                }
            }
        }
    }

    fn value<'tree>(
        &mut self,
        node: &Node<'tree>,
        current: Option<&(String, Value)>,
        polars: &LibraryNames<'tree>,
    ) -> Option<Value> {
        match node.kind().as_ref() {
            "identifier" => {
                let (name, value) = current?;
                (node.text() == name.as_str()).then_some(*value)
            }
            "parenthesized_expression" => {
                self.value(&node.named_children().next()?, current, polars)
            }
            "call" => {
                let function = node.field("function")?;
                let receiver = function.field("object")?;
                let method = function.field("attribute")?;
                let args = arguments(node)?;
                let at = line(&method);
                let name = method.text();
                if name == "read_parquet"
                    && receiver.kind() == "identifier"
                    && polars.binds(&receiver)
                {
                    return self.read_parquet(at, &args);
                }
                let value = self.value(&receiver, current, polars)?;
                self.method(at, &name, &args, value, polars)
            }
            _ => None,
        }
    }

    fn read_parquet(&mut self, at: usize, args: &[Node<'_>]) -> Option<Value> {
        self.evidence
            .note(at, "eager-read", "read_parquet materializes the input.");
        let source = match args {
            [source] if source.kind() == "keyword_argument" => source
                .field("name")
                .filter(|name| name.text() == "source")
                .and_then(|_| source.field("value")),
            [source] => Some(source.clone()),
            _ => None,
        };
        if self.read.is_some()
            || !source.is_some_and(|source| {
                source.kind() == "identifier" || text_literal(&source).is_some()
            })
        {
            self.evidence.block(at, "parquet-source", "Resolve multiple reads, read options or the source expression before choosing scan_parquet.");
            return None;
        }
        self.read = Some(at);
        self.stage(at, "read_parquet", Vec::new());
        Some(Value::Eager)
    }

    fn method<'tree>(
        &mut self,
        at: usize,
        name: &str,
        args: &[Node<'tree>],
        value: Value,
        polars: &LibraryNames<'tree>,
    ) -> Option<Value> {
        match (name, value) {
            ("select" | "filter", Value::Eager | Value::Lazy) => {
                self.evidence
                    .note(at, name_code(name), &format!("The pipeline calls {name}."));
                let columns = if name == "select" {
                    projection_columns(args)
                } else {
                    let mut columns = Vec::new();
                    let native = args
                        .iter()
                        .all(|arg| native_expr(arg, polars, &mut columns));
                    native.then_some(columns)
                };
                let Some(columns) = columns.filter(|columns| !columns.is_empty()) else {
                    self.evidence.block(at, "expression", "Resolve dynamic columns, row-aligned inputs or Python callbacks before moving this operation into the lazy plan.");
                    return None;
                };
                self.pushdown = true;
                self.columns.extend(columns.iter().cloned());
                self.stage(at, name, columns);
                Some(value)
            }
            ("lazy", Value::Eager) if args.is_empty() => {
                self.stage(at, name, Vec::new());
                Some(Value::Lazy)
            }
            ("collect", Value::Lazy) if args.is_empty() => {
                self.evidence
                    .note(at, "collect", "This collect materializes the query.");
                self.stage(at, name, Vec::new());
                self.boundary = Some((at, "Keep this collect as the materialization boundary."));
                Some(Value::Collected)
            }
            ("rows" | "iter_rows" | "write_parquet", Value::Eager | Value::Collected)
                if simple_consumer(name, args) =>
            {
                self.evidence.note(
                    at,
                    "eager-consumer",
                    &format!("{name} consumes a DataFrame."),
                );
                self.stage(at, name, Vec::new());
                if matches!(value, Value::Eager) {
                    self.boundary = Some((
                        at,
                        "Collect immediately before this DataFrame consumer. Preserve its arguments and side effects.",
                    ));
                }
                Some(Value::Consumed)
            }
            _ => {
                self.evidence.block(at, "operation", &format!("Resolve {name}, its arguments and any ordering or materialization requirements."));
                None
            }
        }
    }

    fn stage(&mut self, at: usize, name: &str, reads: Vec<String>) {
        self.stages.push(Stage {
            name: name.to_owned(),
            line: at,
            reads,
            writes: Vec::new(),
        });
    }

    fn unknown(&mut self, statement: &Node<'_>) {
        self.evidence.block(
            line(statement),
            "dataflow",
            "Trace this statement and determine whether it needs the eager frame.",
        );
    }

    fn finish(&mut self, line: usize) {
        if let Some(read) = self.read.filter(|_| self.pushdown) {
            self.evidence.suggest(read, "scan-parquet", "For a file path, start with scan_parquet and keep the supported filters and projections in one lazy plan. A scan replaces an immediate lazy() call.");
        } else {
            self.evidence.block(line, "pushdown", "Show a single Parquet read followed by a native filter or a literal column selection.");
        }
        if let Some((at, message)) = self.boundary {
            self.evidence.suggest(at, "materialization", message);
        } else {
            self.evidence.block(
                line,
                "materialization",
                "Locate the return or first consumer that needs materialized data.",
            );
        }
        self.evidence.suggest(line, "verification", "Check column names, dtypes, values and row order on the same file. Inspect the optimized plan and time both implementations with the same settings through the materialized result.");
    }
}

fn name_code(name: &str) -> &'static str {
    if name == "select" {
        "projection"
    } else {
        "filter"
    }
}

fn native_expr<'tree>(
    node: &Node<'tree>,
    polars: &LibraryNames<'tree>,
    columns: &mut Vec<String>,
) -> bool {
    match node.kind().as_ref() {
        "integer" | "float" | "true" | "false" | "none" => true,
        "string" => text_literal(node).is_some(),
        "comparison_operator"
        | "binary_operator"
        | "parenthesized_expression"
        | "unary_operator" => {
            let children: Vec<_> = node
                .named_children()
                .filter(|node| node.kind() != "comment")
                .collect();
            children.len() <= 2
                && children
                    .iter()
                    .all(|child| native_expr(child, polars, columns))
        }
        "call" => {
            let Some(function) = node.field("function") else {
                return false;
            };
            if function
                .field("attribute")
                .is_none_or(|member| member.text() != "col")
                || !function
                    .field("object")
                    .is_some_and(|object| object.kind() == "identifier" && polars.binds(&object))
            {
                return false;
            }
            let Some(args) = arguments(node) else {
                return false;
            };
            if let [name] = args.as_slice()
                && let Some(column) = column_literal(name)
            {
                columns.push(column);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn simple_consumer(name: &str, args: &[Node<'_>]) -> bool {
    match (name, args) {
        ("rows" | "iter_rows", []) => true,
        ("write_parquet", [path]) => path.kind() == "identifier" || text_literal(path).is_some(),
        _ => false,
    }
}

fn text_literal(node: &Node<'_>) -> Option<String> {
    let start = node
        .child(0)
        .filter(|child| child.kind() == "string_start")?;
    if start.text().starts_with(['b', 'B']) {
        return None;
    }
    string_literal(node)
}

fn column_literal(node: &Node<'_>) -> Option<String> {
    text_literal(node).filter(|name| !(name == "*" || name.starts_with('^') && name.ends_with('$')))
}

fn projection_columns(args: &[Node<'_>]) -> Option<Vec<String>> {
    if let [columns] = args
        && matches!(columns.kind().as_ref(), "list" | "tuple")
    {
        return columns
            .named_children()
            .filter(|column| column.kind() != "comment")
            .map(|column| column_literal(&column))
            .collect();
    }
    args.iter().map(column_literal).collect()
}

fn arguments<'tree>(call: &Node<'tree>) -> Option<Vec<Node<'tree>>> {
    let args = call.field("arguments")?;
    (args.kind() == "argument_list").then(|| {
        args.named_children()
            .filter(|arg| arg.kind() != "comment")
            .collect()
    })
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}
