//! Reads pipelines of steps: functions that take a frame and return it with columns added.
//!
//! A pipeline is a function that passes one frame through other functions of the same module,
//! as `frame.pipe(a).pipe(b)` or as `frame = a(frame)` followed by `frame = b(frame)`. Each of
//! those functions is a step. A step reads columns with `frame["name"]` and `pd.col("name")` and
//! adds them with `.assign(name=...)`, with `frame.join(...)`, or with `frame["name"] = ...`.
//! The names are read from the source, so the columns each step needs from an earlier step are
//! known without running anything.
//!
//! A method chain is a pipeline too. Each `.assign(...)` of a chain with two or more steps is a
//! step, in a function or in a statement of the module, as in
//! `orders.assign(gross=...).assign(share=...)`.

use std::collections::{BTreeMap, BTreeSet};

use ast_grep_core::Node;

use crate::nodes::{Finding, Level, literal};
use crate::outline::{self, Function};
use crate::scan;

/// A node of the syntax tree of a Python file.
type Syntax<'r> = scan::Node<'r>;

/// Methods that keep the columns of the frame they are called on.
const KEEPS_COLUMNS: &[&str] = &["astype", "clip", "fillna", "mask", "round", "where"];

/// How deep a name is followed through the names it stands for.
const DEEPEST_NAME: usize = 16;

/// How much of a formula a message quotes.
const QUOTED_CHARS: usize = 90;

/// One pipeline and its steps, in the order they run.
#[derive(Debug)]
pub(crate) struct Pipeline {
    /// The function that runs the steps.
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) steps: Vec<Step>,
    pub(crate) links: Vec<Link>,
    pub(crate) findings: Vec<Finding>,
    /// The columns the steps read that no earlier step adds, so the frame the pipeline starts
    /// from must have them. They are the columns to read from a file, as `columns=` of
    /// `read_parquet`.
    pub(crate) source: Vec<String>,
}

impl Pipeline {
    /// Whether a step is a function of the module. A pipeline of chain links alone leaves the
    /// functions of its file to be read as before.
    pub(crate) fn runs_functions(&self) -> bool {
        (self.steps.iter()).any(|step| step.kind == StepKind::Function)
    }

    /// Whether `source` holds every column the pipeline reads. A step with computed names reads
    /// columns that the file does not name.
    pub(crate) fn source_is_complete(&self) -> bool {
        !self.steps.iter().any(|step| step.computed)
    }

    /// The steps that share no column with another step, so they can run anywhere. A step with
    /// computed names is left out, because not all of its columns are known.
    pub(crate) fn free_steps(&self) -> Vec<&str> {
        let shares = |a: &Step, b: &Step| {
            a.writes
                .iter()
                .any(|w| b.reads.contains(w) || b.writes.contains(w))
                || b.writes.iter().any(|w| a.reads.contains(w))
        };
        self.steps
            .iter()
            .filter(|step| !step.computed)
            .filter(|step| {
                !self
                    .steps
                    .iter()
                    .any(|other| !std::ptr::eq(other, *step) && shares(step, other))
            })
            .map(|step| step.name.as_str())
            .collect()
    }
}

/// One step of a pipeline.
#[derive(Debug)]
pub(crate) struct Step {
    /// The name of the function, or the call of a chain link, as in `.assign(gross)`.
    pub(crate) name: String,
    pub(crate) kind: StepKind,
    pub(crate) line: usize,
    /// The columns the step reads by name.
    pub(crate) reads: Vec<String>,
    /// The columns the step adds by name.
    pub(crate) writes: Vec<String>,
    /// The step also reads or adds columns whose names the file computes.
    pub(crate) computed: bool,
    /// Names read as `frame.name`, which are columns when another step adds them.
    attributes: Vec<String>,
}

impl Step {
    /// The step as a message names it: `totals()` for a function, `.assign(gross)` for a link.
    fn shown(&self) -> String {
        self.kind.shown(&self.name)
    }
}

/// What a step is in the code of its pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKind {
    /// A function of the module.
    Function,
    /// One `.assign(...)` call of a method chain.
    Link,
}

impl StepKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Link => "link",
        }
    }

    /// A step of this kind as a message names it.
    fn shown(self, name: &str) -> String {
        match self {
            Self::Function => format!("{name}()"),
            Self::Link => name.to_owned(),
        }
    }
}

/// One step as the code of a pipeline names it.
enum StepRef<'r> {
    /// A function of the module, by name.
    Function(String),
    /// One `.assign(...)` call of a method chain.
    Link(Syntax<'r>),
}

/// A step that reads columns an earlier step adds.
#[derive(Debug)]
pub(crate) struct Link {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) columns: Vec<String>,
}

/// How a step adds a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// `frame.join(new)`, which raises when the column exists.
    Join,
    /// `frame.assign(name=...)` or a write to a copy, which replaces a column that exists.
    Replace,
    /// `frame["name"] = ...` on the frame the step was given.
    InPlace,
}

/// One column a step adds.
#[derive(Debug)]
struct Write {
    name: String,
    line: usize,
    way: Way,
}

/// One named formula in a step: a column it adds, or a local name.
#[derive(Debug)]
struct Formula {
    name: String,
    line: usize,
    is_column: bool,
    /// The formula with local names replaced and the terms of each sum sorted.
    text: String,
    /// The formula as the file writes it, on one line.
    source: String,
}

/// What a formula names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Column,
    Local,
}

/// What reading one step function has found so far.
#[derive(Debug, Default)]
struct Found {
    reads: BTreeSet<String>,
    writes: Vec<Write>,
    formulas: Vec<Formula>,
    findings: Vec<Finding>,
    /// The step reads or adds columns whose names the file computes.
    computed: bool,
}

/// What one step does, before the steps are compared.
#[derive(Debug)]
struct StepBody {
    step: Step,
    writes: Vec<Write>,
    formulas: Vec<Formula>,
    findings: Vec<Finding>,
    /// For a chain link, the columns it reads from the name the chain starts at, as in
    /// `orders["gross"]`, each with its line. That name holds the frame from before the chain.
    start_reads: Vec<(String, usize)>,
    /// For a chain link, the name the chain starts at.
    start: Option<String>,
}

/// The pipelines of the module in `src`. Empty when the module has none.
pub(crate) fn read(src: &str) -> Vec<Pipeline> {
    let root = scan::parse(src);
    let module = root.root();
    // Python binds each name to its last definition.
    let outline::Module {
        functions,
        assignments,
    } = outline::python_module(&root);
    let functions: BTreeMap<String, Function<'_>> = (functions.into_iter())
        .map(|function| (function.name.clone(), function))
        .collect();
    let constants: BTreeMap<String, Syntax<'_>> = assignments.into_iter().collect();
    let tree = docstring_tree(&module);
    let mut orders: Vec<(&str, &Function<'_>, Vec<StepRef<'_>>)> = functions
        .iter()
        .filter_map(|(name, function)| {
            let body = function.definition.field("body")?;
            let order = step_order(&body, Some(name), &functions);
            Some((name.as_str(), function, order))
        })
        .collect();
    // A chain in a statement of the module, as in `result = orders.assign(...).assign(...)`.
    let statements: Vec<(Syntax<'_>, Vec<StepRef<'_>>)> = module
        .children()
        .filter(|statement| statement.kind() == "expression_statement")
        .map(|statement| {
            let order = step_order(&statement, None, &functions);
            (statement, order)
        })
        .filter(|(_, order)| {
            order.len() >= MIN_STEPS && order.iter().any(|s| matches!(s, StepRef::Link(_)))
        })
        .collect();
    // A function that another pipeline runs as a step is read as that step, so its own chain
    // links are no steps of a second pipeline.
    let run_as_step: BTreeSet<String> = orders
        .iter()
        .flat_map(|(_, _, order)| order)
        .chain(statements.iter().flat_map(|(_, order)| order))
        .filter_map(|step| match step {
            StepRef::Function(name) => Some(name.clone()),
            StepRef::Link(_) => None,
        })
        .collect();
    for (name, _, order) in &mut orders {
        if run_as_step.contains(*name) {
            order.retain(|step| matches!(step, StepRef::Function(_)));
        }
    }
    let mut pipelines: Vec<Pipeline> = orders
        .iter()
        .filter(|(_, _, order)| order.len() >= MIN_STEPS)
        .map(|(name, function, order)| {
            let line = function.definition.start_pos().line() + 1;
            pipeline(name, line, order, &functions, &constants)
        })
        .collect();
    for (statement, order) in &statements {
        let name = simple_assignment(statement).map_or_else(|| "(module)".to_owned(), |a| a.0);
        let line = statement.start_pos().line() + 1;
        pipelines.push(pipeline(&name, line, order, &functions, &constants));
    }
    pipelines.sort_by_key(|p| p.line);
    let mut with_functions: Vec<&mut Pipeline> =
        (pipelines.iter_mut().filter(|p| p.runs_functions())).collect();
    if let ([only], Some(tree)) = (with_functions.as_mut_slice(), tree) {
        // The tree lists functions, so the links of a chain are left out of the comparison.
        let ran: Vec<&str> = (only.steps.iter().filter(|s| s.kind == StepKind::Function))
            .map(|s| s.name.as_str())
            .collect();
        if tree != ran {
            only.findings.push(Finding {
                line: only.line,
                level: Level::Warning,
                message: format!(
                    "The tree in the module docstring lists {} and {}() runs {}. Change the one that is out of date.",
                    tree.join(", "),
                    only.name,
                    ran.join(", ")
                ),
            });
        }
    }
    pipelines
}

/// `name = value` as a statement of its own.
fn simple_assignment<'r>(statement: &Syntax<'r>) -> Option<(String, Syntax<'r>)> {
    if statement.kind() != "expression_statement" {
        return None;
    }
    let assignment = statement.children().find(|c| c.kind() == "assignment")?;
    let left = assignment
        .field("left")
        .filter(|l| l.kind() == "identifier")?;
    Some((left.text().into_owned(), assignment.field("right")?))
}

/// The step names in a tree drawn in the module docstring, as in `├── step()  what it does`.
fn docstring_tree(module: &Syntax<'_>) -> Option<Vec<String>> {
    let first = module.children().find(Node::is_named)?;
    let docstring = first.children().find(|c| c.kind() == "string")?;
    let text = docstring.text();
    let names: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let rest = line.split_once("── ")?.1;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty() && rest[name.len()..].starts_with('(')).then_some(name)
        })
        .collect();
    (!names.is_empty()).then_some(names)
}

/// The fewest steps that make a pipeline, and the fewest links that make a chain of steps.
const MIN_STEPS: usize = 2;

/// The steps that the code in `body` passes a frame through, in the order they run: the module
/// functions, and the `.assign(...)` links of each method chain with two or more steps. `own`
/// is the function that holds `body`, which is no step of itself.
fn step_order<'r>(
    body: &Syntax<'r>,
    own: Option<&str>,
    functions: &BTreeMap<String, Function<'r>>,
) -> Vec<StepRef<'r>> {
    let is_step = |name: &str| functions.contains_key(name) && own != Some(name);
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut links: Vec<(usize, Syntax<'r>)> = Vec::new();
    for call in body.dfs().filter(|n| n.kind() == "call") {
        if !heads_method_chain(&call) {
            let chain = chain_calls(&call);
            let is_link = |c: &Syntax<'r>| is_method_call(c, "assign") && !is_new_frame(c);
            let steps = (chain.iter())
                .filter(|c| is_link(c) || piped_step(c).is_some_and(|step| is_step(&step.text())))
                .count();
            if steps >= MIN_STEPS {
                links.extend(chain.into_iter().filter(|c| is_link(c)).filter_map(|c| {
                    let at = c.field("arguments")?.range().start;
                    Some((at, c))
                }));
            }
        }
        let Some(target) = call.field("function") else {
            continue;
        };
        let first = call
            .field("arguments")
            .and_then(|a| a.children().find(Node::is_named));
        if target.kind() == "attribute" {
            // frame.pipe(step, ...)
            if let Some(step) = piped_step(&call).filter(|step| is_step(&step.text())) {
                found.push((step.range().start, step.text().into_owned()));
            }
        } else if target.kind() == "identifier" && is_step(&target.text()) {
            // step(frame) at the head of a pipe chain, or frame = step(frame).
            let heads_chain = call.parent().is_some_and(|p| {
                p.kind() == "attribute" && p.field("attribute").is_some_and(|a| a.text() == "pipe")
            });
            let reassigns = call.parent().is_some_and(|p| {
                let left = p.field("left").filter(|l| l.kind() == "identifier");
                let passed = first.as_ref().filter(|f| f.kind() == "identifier");
                p.kind() == "assignment"
                    && left.zip(passed).is_some_and(|(l, f)| l.text() == f.text())
            });
            if heads_chain || reassigns {
                found.push((target.range().start, target.text().into_owned()));
            }
        }
    }
    found.sort();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut order: Vec<(usize, StepRef<'r>)> = found
        .into_iter()
        .filter(|(_, name)| seen.insert(name.clone()))
        .map(|(at, name)| (at, StepRef::Function(name)))
        .collect();
    order.extend(
        links
            .into_iter()
            .map(|(at, link)| (at, StepRef::Link(link))),
    );
    order.sort_by_key(|(at, _)| *at);
    order.into_iter().map(|(_, step)| step).collect()
}

/// The function that `call` pipes a frame through, when `call` is `frame.pipe(step, ...)`.
fn piped_step<'r>(call: &Syntax<'r>) -> Option<Syntax<'r>> {
    if !is_method_call(call, "pipe") {
        return None;
    }
    let first = call.field("arguments")?.children().find(Node::is_named)?;
    (first.kind() == "identifier").then_some(first)
}

/// Whether a method is called on the result of `call`, so that `call` is not the last call of
/// its chain.
fn heads_method_chain(call: &Syntax<'_>) -> bool {
    call.parent().is_some_and(|attribute| {
        attribute.kind() == "attribute"
            && attribute
                .field("object")
                .is_some_and(|o| o.range() == call.range())
            && attribute
                .parent()
                .is_some_and(|outer| outer.kind() == "call")
    })
}

/// The method calls of the chain that ends in `last`, from the first call to `last`.
fn chain_calls<'r>(last: &Syntax<'r>) -> Vec<Syntax<'r>> {
    let mut calls = vec![last.clone()];
    let mut call = last.clone();
    while let Some(object) = receiver(&call).filter(|o| o.kind() == "call") {
        calls.push(object.clone());
        call = object;
    }
    calls.reverse();
    calls
}

/// What a method is called on: `frame` in `frame.assign(...)`, with any parentheses removed.
fn receiver<'r>(call: &Syntax<'r>) -> Option<Syntax<'r>> {
    let target = call.field("function").filter(|f| f.kind() == "attribute")?;
    let mut object = target.field("object")?;
    while object.kind() == "parenthesized_expression" {
        let inner = object.children().find(Node::is_named)?;
        object = inner;
    }
    Some(object)
}

/// Reads the steps of one pipeline and compares them.
fn pipeline<'r>(
    name: &str,
    line: usize,
    order: &[StepRef<'r>],
    functions: &BTreeMap<String, Function<'r>>,
    constants: &BTreeMap<String, Syntax<'r>>,
) -> Pipeline {
    let mut bodies: Vec<StepBody> = order
        .iter()
        .filter_map(|step| match step {
            StepRef::Function(step) => {
                Reader::new(step, functions.get(step)?, constants).map(Reader::read)
            }
            StepRef::Link(call) => Some(Reader::for_link(call, constants).read_link()),
        })
        .collect();
    name_links_apart(&mut bodies);
    let added: BTreeSet<String> = bodies
        .iter()
        .flat_map(|b| b.writes.iter().map(|w| w.name.clone()))
        .collect();
    for body in &mut bodies {
        let step = &mut body.step;
        for name in step.attributes.iter().filter(|n| added.contains(*n)) {
            if !step.reads.contains(name) {
                step.reads.push(name.clone());
            }
        }
        step.reads.sort();
    }
    let mut findings: Vec<Finding> = Vec::new();
    let mut links: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();
    for (index, body) in bodies.iter().enumerate() {
        for column in &body.step.reads {
            let writers: Vec<(usize, &Write)> = bodies
                .iter()
                .enumerate()
                .flat_map(|(at, b)| b.writes.iter().map(move |w| (at, w)))
                .filter(|(at, w)| w.name == *column && *at != index)
                .collect();
            if let Some((from, _)) = writers.iter().rev().find(|(at, _)| *at < index) {
                links
                    .entry((*from, index))
                    .or_default()
                    .insert(column.clone());
            } else if let Some((later, write)) = writers.first()
                && write.way == Way::Join
            {
                // A joined column cannot exist before its step, so this read finds nothing.
                findings.push(Finding {
                    line: body.step.line,
                    level: Level::Error,
                    message: format!(
                        "{} reads the column {column}, which {} adds later in {name}(). The read raises KeyError. Move {} after {}.",
                        body.step.shown(), bodies[*later].step.shown(), body.step.shown(), bodies[*later].step.shown()
                    ),
                });
            }
        }
        findings.extend(joined_again(index, &bodies));
        findings.extend(read_before_the_chain(index, &bodies));
    }
    findings.extend(repeated_formulas(&bodies));
    let source = source_columns(&bodies);
    let links = links
        .into_iter()
        .map(|((from, to), columns)| Link {
            from: bodies[from].step.name.clone(),
            to: bodies[to].step.name.clone(),
            columns: columns.into_iter().collect(),
        })
        .collect();
    let mut steps = Vec::new();
    for body in bodies {
        findings.extend(body.findings);
        steps.push(body.step);
    }
    findings.sort_by_key(|f| (f.line, f.level));
    Pipeline {
        name: name.to_owned(),
        line,
        steps,
        links,
        findings,
        source,
    }
}

/// Gives two links of one chain that add the same columns different names, by their lines.
fn name_links_apart(bodies: &mut [StepBody]) {
    let names: Vec<String> = bodies.iter().map(|b| b.step.name.clone()).collect();
    for body in bodies.iter_mut().filter(|b| b.step.kind == StepKind::Link) {
        if names.iter().filter(|name| **name == body.step.name).count() > 1 {
            body.step.name = format!("{} at line {}", body.step.name, body.step.line);
        }
    }
}

/// The columns the steps read that no earlier step adds. A column that a step both reads and
/// adds counts when its formula reads the column itself, as in `price=frame["price"] * 2`. In
/// any other case the step reads what it has just added.
fn source_columns(bodies: &[StepBody]) -> Vec<String> {
    let mut source: BTreeSet<String> = BTreeSet::new();
    for (index, body) in bodies.iter().enumerate() {
        for column in &body.step.reads {
            let adds = |b: &StepBody| b.writes.iter().any(|w| w.name == *column);
            let own = adds(body)
                && !(body.formulas.iter()).any(|f| {
                    f.is_column && f.name == *column && f.text.contains(&format!("col[{column}]"))
                });
            // A column that a later step joins cannot be in the frame at the start.
            let joined_later = (bodies[index + 1..].iter()).any(|b| {
                b.writes
                    .iter()
                    .any(|w| w.name == *column && w.way == Way::Join)
            });
            if !own && !joined_later && !bodies[..index].iter().any(adds) {
                source.insert(column.clone());
            }
        }
    }
    source.into_iter().collect()
}

/// A chain link that reads a column from the name its chain starts at, though an earlier link
/// of the chain adds that column. The name still holds the frame from before the chain.
fn read_before_the_chain(index: usize, bodies: &[StepBody]) -> Vec<Finding> {
    let body = &bodies[index];
    let Some(start) = &body.start else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (column, line) in &body.start_reads {
        if !seen.insert(column) {
            continue;
        }
        let earlier = (bodies[..index].iter().rev())
            .find(|b| b.step.kind == StepKind::Link && b.writes.iter().any(|w| w.name == *column));
        if let Some(earlier) = earlier {
            findings.push(Finding {
                line: *line,
                level: Level::Warning,
                message: format!(
                    "{} reads {start}[\"{column}\"], and {start} is the frame from before the chain, without the column {column} that {} adds. The read raises KeyError, or reads old values when {start} has such a column. Read it as pd.col(\"{column}\").",
                    body.step.shown(),
                    earlier.step.shown()
                ),
            });
        }
    }
    findings
}

/// A column that the step at `index` joins and an earlier step added already.
fn joined_again(index: usize, bodies: &[StepBody]) -> Vec<Finding> {
    let body = &bodies[index];
    let mut findings = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for write in body.writes.iter().filter(|w| w.way == Way::Join) {
        let earlier = bodies[..index]
            .iter()
            .find(|b| b.writes.iter().any(|w| w.name == write.name));
        if let Some(earlier) = earlier.filter(|_| seen.insert(&write.name)) {
            findings.push(Finding {
                line: write.line,
                level: Level::Error,
                message: format!(
                    "{} joins the column {}, which {} added already. The join raises \"columns overlap\".",
                    body.step.shown(), write.name, earlier.step.shown()
                ),
            });
        }
    }
    findings
}

/// The same formula under two names, or written out in two steps.
fn repeated_formulas(bodies: &[StepBody]) -> Vec<Finding> {
    let mut by_text: BTreeMap<&str, Vec<(usize, &Formula)>> = BTreeMap::new();
    for (index, body) in bodies.iter().enumerate() {
        for formula in &body.formulas {
            by_text
                .entry(&formula.text)
                .or_default()
                .push((index, formula));
        }
    }
    let mut findings = Vec::new();
    for places in by_text.into_values() {
        let columns: Vec<&(usize, &Formula)> = places.iter().filter(|(_, f)| f.is_column).collect();
        let first_at = places[0].0;
        let step = |at: usize| bodies[at].step.shown();
        if let Some((at, twin)) = columns.iter().find(|(_, f)| f.name != columns[0].1.name) {
            findings.push(Finding {
                line: twin.line,
                level: Level::Warning,
                message: format!(
                    "The column {} from {} has the same formula as {} from {}, so the result holds one column twice. Keep one of them.",
                    twin.name, step(*at), columns[0].1.name, step(columns[0].0)
                ),
            });
        } else if let Some((at, column)) = columns.first() {
            for (other, again) in places.iter().filter(|(o, f)| o != at && !f.is_column) {
                let advice = if at < other {
                    format!("Read the column {} there.", column.name)
                } else {
                    format!("{} adds it as the column {} later.", step(*at), column.name)
                };
                findings.push(Finding {
                    line: again.line,
                    level: Level::Warning,
                    message: format!(
                        "{} computes {} with the formula of the column {} from {}. {advice}",
                        step(*other),
                        again.name,
                        column.name,
                        step(*at)
                    ),
                });
            }
        } else if let Some((other, again)) = places.iter().find(|(at, _)| *at != first_at) {
            findings.push(Finding {
                line: again.line,
                level: Level::Warning,
                message: format!(
                    "{} and {} both compute {}. Compute it in one step and read it in the other.",
                    step(first_at),
                    step(*other),
                    quoted(&again.source)
                ),
            });
        }
    }
    findings
}

/// The start of a formula, for a message.
fn quoted(text: &str) -> String {
    if text.chars().count() <= QUOTED_CHARS {
        return text.to_owned();
    }
    let short: String = text.chars().take(QUOTED_CHARS).collect();
    format!("{short}...")
}

/// Reads one step: a function, or one `.assign(...)` link of a chain.
struct Reader<'r, 'a> {
    /// The name of the function, or the link as it is called, as in `.assign(gross)`.
    name: String,
    kind: StepKind,
    /// The function of a step, or the call of a link.
    function: &'a Syntax<'r>,
    constants: &'a BTreeMap<String, Syntax<'r>>,
    /// The frame parameter. For a link, the name its chain starts at, or nothing.
    frame: String,
    /// The frame parameter and the local names that stand for it or for a copy of it, and the
    /// parameter of each lambda that `.assign(...)` calls with the frame.
    frames: BTreeSet<String>,
    /// Local names with one assignment, and their values.
    locals: BTreeMap<String, Syntax<'r>>,
    /// The step gives the frame parameter a new value, so later writes go to that value.
    rebinds_frame: bool,
}

impl<'r, 'a> Reader<'r, 'a> {
    fn new(
        name: &'a str,
        step: &'a Function<'r>,
        constants: &'a BTreeMap<String, Syntax<'r>>,
    ) -> Option<Self> {
        let frame = (step.parameters.first())
            .map(|first| first.name.clone())
            .filter(|first| first != "self" && first != "cls")?;
        let function = &step.definition;
        let body = function.field("body")?;
        let mut assigned: BTreeMap<String, Vec<Syntax<'r>>> = BTreeMap::new();
        for assignment in body.dfs().filter(|n| n.kind() == "assignment") {
            if let (Some(left), Some(right)) = (assignment.field("left"), assignment.field("right"))
                && left.kind() == "identifier"
            {
                assigned
                    .entry(left.text().into_owned())
                    .or_default()
                    .push(right);
            }
        }
        let rebinds_frame = assigned.contains_key(&frame);
        let mut frames = BTreeSet::from([frame.clone()]);
        frames.extend(
            body.dfs()
                .filter(|n| is_method_call(n, "assign"))
                .flat_map(|a| lambda_frames(&a)),
        );
        let mut locals = BTreeMap::new();
        for (local, mut values) in assigned {
            let Some(value) = values.pop().filter(|_| values.is_empty()) else {
                continue;
            };
            let text = value.text();
            if local != frame && (text == frame || text == format!("{frame}.copy()")) {
                frames.insert(local);
            } else if local != frame {
                locals.insert(local, value);
            }
        }
        Some(Self {
            name: name.to_owned(),
            kind: StepKind::Function,
            function,
            constants,
            frame,
            frames,
            locals,
            rebinds_frame,
        })
    }

    /// A reader for one `.assign(...)` call of a chain. Its frames are the parameters of the
    /// lambdas it passes and the name the chain starts at.
    fn for_link(link: &'a Syntax<'r>, constants: &'a BTreeMap<String, Syntax<'r>>) -> Self {
        let mut columns: Vec<String> = keywords(link).into_iter().map(|(name, _)| name).collect();
        if columns.len() > NAMED_COLUMNS {
            columns.truncate(NAMED_COLUMNS);
            columns.push("...".to_owned());
        }
        let start = chain_start(link);
        let mut frames: BTreeSet<String> = lambda_frames(link).into_iter().collect();
        frames.extend(start.clone());
        Self {
            name: format!(".assign({})", columns.join(", ")),
            kind: StepKind::Link,
            function: link,
            constants,
            frame: start.unwrap_or_default(),
            frames,
            locals: BTreeMap::new(),
            rebinds_frame: false,
        }
    }

    /// The step as a message names it.
    fn shown(&self) -> String {
        self.kind.shown(&self.name)
    }

    fn line(node: &Syntax<'_>) -> usize {
        node.start_pos().line() + 1
    }

    /// The column names an index stands for, or `None` when the file computes them.
    fn names(&self, node: &Syntax<'r>, depth: usize) -> Option<Vec<String>> {
        if depth > DEEPEST_NAME {
            return None;
        }
        match node.kind().as_ref() {
            "string" => literal(node).map(|name| vec![name]),
            "identifier" => {
                let name = node.text();
                let value = self
                    .locals
                    .get(name.as_ref())
                    .or_else(|| self.constants.get(name.as_ref()))?;
                self.names(value, depth + 1)
            }
            "parenthesized_expression" => {
                let inner = node.children().find(Node::is_named)?;
                self.names(&inner, depth + 1)
            }
            "list" => {
                let mut all = Vec::new();
                for item in node.children().filter(Node::is_named) {
                    all.extend(self.names(&item, depth + 1)?);
                }
                Some(all)
            }
            "binary_operator" if node.field("operator").is_some_and(|o| o.text() == "+") => {
                let mut all = self.names(&node.field("left")?, depth + 1)?;
                all.extend(self.names(&node.field("right")?, depth + 1)?);
                Some(all)
            }
            _ => None,
        }
    }

    /// The column an indexed frame names: `frame[...]`, or `frame.loc[..., column]`.
    fn frame_index(&self, subscript: &Syntax<'r>) -> Option<Syntax<'r>> {
        let value = subscript.field("value")?;
        let index = subscript.field("subscript")?;
        if value.kind() == "identifier" && self.frames.contains(value.text().as_ref()) {
            return Some(index);
        }
        let through_indexer = value.kind() == "attribute"
            && value
                .field("attribute")
                .is_some_and(|a| matches!(a.text().as_ref(), "loc" | "at"))
            && value
                .field("object")
                .is_some_and(|o| self.frames.contains(o.text().as_ref()));
        if !through_indexer {
            return None;
        }
        // frame.loc[rows, column]: the column is the last of the two indexes.
        subscript.field_children("subscript").last()
    }

    /// Whether an index picks rows, as a mask or a slice does, and so names no column.
    fn selects_rows(&self, index: &Syntax<'r>) -> bool {
        match index.kind().as_ref() {
            "slice"
            | "comparison_operator"
            | "boolean_operator"
            | "not_operator"
            | "unary_operator" => true,
            "binary_operator" => index
                .field("operator")
                .is_some_and(|o| matches!(o.text().as_ref(), "&" | "|" | "^")),
            "identifier" => self
                .locals
                .get(index.text().as_ref())
                .is_some_and(|value| self.selects_rows(value)),
            "parenthesized_expression" => index
                .children()
                .find(Node::is_named)
                .is_some_and(|inner| self.selects_rows(&inner)),
            _ => false,
        }
    }

    fn read(self) -> StepBody {
        let line = Self::line(self.function);
        let mut found = Found::default();
        if let Some(body) = self.function.field("body") {
            let targets = self.writes_in_place(&body, &mut found);
            self.column_reads(&body, &targets, &mut found);
            let joined = self.returned_columns(&body, &mut found);
            self.unjoined_columns(&body, &joined, &mut found);
            self.assigned_twice(line, &mut found);
            found.findings.extend(self.chains_read_before(&body));
        }
        // One formula per column name is enough to compare steps.
        let mut kept: BTreeSet<(String, String)> = BTreeSet::new();
        found
            .formulas
            .retain(|f| kept.insert((f.name.clone(), f.text.clone())));
        for (local, value) in &self.locals {
            found.formulas.push(self.formula(local, value, Kind::Local));
        }
        found.formulas.retain(|f| is_computation(&f.text));
        let mut names: Vec<String> = Vec::new();
        for write in &found.writes {
            if !names.contains(&write.name) {
                names.push(write.name.clone());
            }
        }
        StepBody {
            step: Step {
                name: self.name.clone(),
                kind: self.kind,
                line,
                reads: found.reads.into_iter().collect(),
                writes: names,
                computed: found.computed,
                attributes: self.attributes(),
            },
            writes: found.writes,
            formulas: found.formulas,
            findings: found.findings,
            start_reads: Vec::new(),
            start: None,
        }
    }

    /// For each chain of `.assign(...)` calls in a step function, the links that read a column
    /// from the name the chain starts at, though an earlier link of the chain adds it.
    fn chains_read_before(&self, body: &Syntax<'r>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for last in (body.dfs()).filter(|n| n.kind() == "call" && !heads_method_chain(n)) {
            let links: Vec<Syntax<'r>> = (chain_calls(&last).into_iter())
                .filter(|c| is_method_call(c, "assign") && !is_new_frame(c))
                .collect();
            if links.len() < MIN_STEPS {
                continue;
            }
            let bodies: Vec<StepBody> = (links.iter())
                .map(|link| Reader::for_link(link, self.constants).read_link())
                .collect();
            findings.extend((0..bodies.len()).flat_map(|at| read_before_the_chain(at, &bodies)));
        }
        findings
    }

    /// Reads one `.assign(...)` call of a chain: each keyword adds a column, and its value reads
    /// columns through the parameter of its lambda, through `pd.col`, or through the name the
    /// chain starts at.
    fn read_link(self) -> StepBody {
        let call = self.function;
        let line = (call.field("function").and_then(|f| f.field("attribute")))
            .map_or_else(|| Self::line(call), |method| Self::line(&method));
        let mut found = Found::default();
        let mut start_reads = Vec::new();
        for (name, value) in keywords(call) {
            let value = lambda_body(&value);
            self.column_reads(&value, &BTreeSet::new(), &mut found);
            start_reads.extend(self.reads_from_start(&value));
            found
                .formulas
                .push(self.formula(&name, &value, Kind::Column));
            found.writes.push(Write {
                name,
                line: Self::line(&value),
                way: Way::Replace,
            });
        }
        // assign(**columns) adds columns whose names the file computes.
        found.computed |= call
            .field("arguments")
            .is_some_and(|a| a.children().any(|c| c.kind() == "dictionary_splat"));
        found.formulas.retain(|f| is_computation(&f.text));
        StepBody {
            step: Step {
                name: self.name.clone(),
                kind: self.kind,
                line,
                reads: found.reads.into_iter().collect(),
                writes: found.writes.iter().map(|w| w.name.clone()).collect(),
                computed: found.computed,
                attributes: Vec::new(),
            },
            writes: found.writes,
            formulas: found.formulas,
            findings: found.findings,
            start_reads,
            start: (!self.frame.is_empty()).then(|| self.frame.clone()),
        }
    }

    /// The columns that `value` reads from the name the chain starts at, each with its line. A
    /// lambda with a parameter of that name reads its own frame, so it is left out.
    fn reads_from_start(&self, value: &Syntax<'r>) -> Vec<(String, usize)> {
        let mut reads = Vec::new();
        if self.frame.is_empty() {
            return reads;
        }
        for subscript in value.dfs().filter(|n| n.kind() == "subscript") {
            let on_start = subscript
                .field("value")
                .is_some_and(|v| v.kind() == "identifier" && v.text() == self.frame.as_str());
            let in_own_lambda = subscript.ancestors().any(|a| {
                a.kind() == "lambda" && lambda_parameter(&a).as_deref() == Some(self.frame.as_str())
            });
            if !on_start || in_own_lambda {
                continue;
            }
            let names = subscript.field("subscript").and_then(|i| self.names(&i, 0));
            reads.extend(
                names
                    .into_iter()
                    .flatten()
                    .map(|name| (name, Self::line(&subscript))),
            );
        }
        reads
    }

    /// Records `frame["name"] = value` and `frame.loc[rows, "name"] = value`, and returns where
    /// those targets start, so that they are not read as reads.
    fn writes_in_place(&self, body: &Syntax<'r>, found: &mut Found) -> BTreeSet<usize> {
        let mut targets: BTreeSet<usize> = BTreeSet::new();
        let mut warned = false;
        for assignment in body
            .dfs()
            .filter(|n| matches!(n.kind().as_ref(), "assignment" | "augmented_assignment"))
        {
            let Some(left) = assignment.field("left").filter(|l| l.kind() == "subscript") else {
                continue;
            };
            let Some(index) = self.frame_index(&left) else {
                continue;
            };
            targets.insert(left.range().start);
            let on_given = left
                .dfs()
                .find(|n| n.kind() == "identifier")
                .is_some_and(|n| n.text() == self.frame.as_str())
                && !self.rebinds_frame;
            if on_given && !warned {
                warned = true;
                found.findings.push(Finding {
                    line: Self::line(&assignment),
                    level: Level::Warning,
                    message: format!(
                        "{} writes to the frame it was given. Called as {}({}), it changes the caller's frame too. Return {}.assign(...) or write to {}.copy().",
                        self.shown(), self.name, self.frame, self.frame, self.frame
                    ),
                });
            }
            let way = if on_given { Way::InPlace } else { Way::Replace };
            let Some(names) = self.names(&index, 0) else {
                found.computed = true;
                continue;
            };
            for name in names {
                if assignment.kind() == "augmented_assignment" {
                    found.reads.insert(name.clone());
                } else if let Some(value) = assignment.field("right") {
                    found
                        .formulas
                        .push(self.formula(&name, &value, Kind::Column));
                }
                found.writes.push(Write {
                    name,
                    line: Self::line(&assignment),
                    way,
                });
            }
        }
        targets
    }

    /// Records every `frame["name"]` that is no write target.
    fn column_reads(&self, body: &Syntax<'r>, targets: &BTreeSet<usize>, found: &mut Found) {
        for subscript in body.dfs().filter(|n| n.kind() == "subscript") {
            if targets.contains(&subscript.range().start) {
                continue;
            }
            let Some(index) = self.frame_index(&subscript) else {
                continue;
            };
            match self.names(&index, 0) {
                Some(names) => found.reads.extend(names),
                None => found.computed |= !self.selects_rows(&index),
            }
        }
        // pd.col("name") reads a column of the frame the expression is evaluated on.
        for call in body.dfs().filter(is_col_call) {
            match col_name(&call) {
                Some(name) => {
                    found.reads.insert(name);
                }
                None => found.computed = true,
            }
        }
        // frame.filter(...) picks columns by a pattern.
        found.computed |= body.dfs().any(|n| {
            n.kind() == "call"
                && n.field("function").is_some_and(|f| {
                    f.kind() == "attribute"
                        && f.field("attribute").is_some_and(|a| a.text() == "filter")
                        && f.field("object")
                            .is_some_and(|o| self.frames.contains(o.text().as_ref()))
                })
        });
    }

    /// Records the columns the returned frame gets from `.assign(...)` and `.join(...)`, and
    /// returns where those `.assign(...)` calls start.
    fn returned_columns(&self, body: &Syntax<'r>, found: &mut Found) -> BTreeSet<usize> {
        let mut joined: BTreeSet<usize> = BTreeSet::new();
        for returned in body
            .dfs()
            .filter(|n| n.kind() == "return_statement")
            .filter_map(|r| r.children().find(Node::is_named))
        {
            let computed = self.returned(&returned, &mut joined, found, 0);
            found.computed |= computed;
        }
        joined
    }

    /// A frame of new columns that never joins the result under those names.
    fn unjoined_columns(&self, body: &Syntax<'r>, joined: &BTreeSet<usize>, found: &mut Found) {
        for assign in body.dfs().filter(|n| is_method_call(n, "assign")) {
            if joined.contains(&assign.range().start) || !is_new_frame(&assign) {
                continue;
            }
            let names: Vec<String> = keywords(&assign)
                .into_iter()
                .map(|(name, _)| name)
                .filter(|name| !found.writes.iter().any(|w| w.name == *name))
                .collect();
            if !names.is_empty() {
                found.findings.push(Finding {
                    line: Self::line(&assign),
                    level: Level::Note,
                    message: format!(
                        "{} assigns {} to a frame that it does not join to the result, so the result has no column of that name.",
                        self.shown(),
                        names.join(", ")
                    ),
                });
            }
            for (name, value) in keywords(&assign) {
                found
                    .formulas
                    .push(self.formula(&name, &lambda_body(&value), Kind::Column));
            }
        }
    }

    /// A column assigned twice in one step with one formula. Two formulas are a value built in
    /// two stages, which is left alone.
    fn assigned_twice(&self, line: usize, found: &mut Found) {
        let mut counted: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for formula in found.formulas.iter().filter(|f| f.is_column) {
            *counted.entry((&formula.name, &formula.text)).or_default() += 1;
        }
        let mut findings = Vec::new();
        for ((column, text), times) in counted.into_iter().filter(|(_, times)| *times > 1) {
            let at = found
                .formulas
                .iter()
                .rfind(|f| f.is_column && f.name == column && f.text == text);
            findings.push(Finding {
                line: at.map_or(line, |f| f.line),
                level: Level::Warning,
                message: format!(
                    "{} assigns the column {column} {times} times with the same formula. Assign it once.",
                    self.shown()
                ),
            });
        }
        found.findings.extend(findings);
    }

    /// The names read as `frame.name`, without the methods the step calls.
    fn attributes(&self) -> Vec<String> {
        let Some(body) = self.function.field("body") else {
            return Vec::new();
        };
        let names: BTreeSet<String> = body
            .dfs()
            .filter(|n| n.kind() == "attribute")
            .filter(|n| {
                n.field("object")
                    .is_some_and(|o| self.frames.contains(o.text().as_ref()))
            })
            .filter(|n| {
                !n.parent().is_some_and(|p| {
                    p.kind() == "call"
                        && p.field("function").is_some_and(|f| f.range() == n.range())
                })
            })
            .filter_map(|n| n.field("attribute").map(|a| a.text().into_owned()))
            .collect();
        names.into_iter().collect()
    }

    /// Walks the returned expression from its last method call back to the frame, and records
    /// the columns that `.assign(...)` and `.join(...)` add. Returns whether a part of it adds
    /// columns with names the file computes.
    fn returned(
        &self,
        node: &Syntax<'r>,
        joined: &mut BTreeSet<usize>,
        found: &mut Found,
        depth: usize,
    ) -> bool {
        if depth > DEEPEST_NAME {
            return true;
        }
        match node.kind().as_ref() {
            "parenthesized_expression" => node
                .children()
                .find(Node::is_named)
                .is_some_and(|inner| self.returned(&inner, joined, found, depth + 1)),
            "identifier" => {
                let name = node.text();
                if self.frames.contains(name.as_ref()) {
                    return false;
                }
                // A local name stands for its value. Any other name is a frame from elsewhere.
                self.locals
                    .get(name.as_ref())
                    .is_none_or(|value| self.returned(value, joined, found, depth + 1))
            }
            "call" => {
                let Some(target) = node.field("function").filter(|f| f.kind() == "attribute")
                else {
                    // step(frame) in a pipeline returns what that function returns.
                    return true;
                };
                let (Some(object), Some(method)) =
                    (target.field("object"), target.field("attribute"))
                else {
                    return true;
                };
                let mut computed = self.returned(&object, joined, found, depth + 1);
                match method.text().as_ref() {
                    "assign" => {
                        joined.insert(node.range().start);
                        self.record(node, Way::Replace, found);
                    }
                    "join" => {
                        let argument = node
                            .field("arguments")
                            .and_then(|a| a.children().find(Node::is_named));
                        computed |=
                            argument.is_none_or(|a| self.joined_frame(&a, joined, found, 0));
                    }
                    _ => {}
                }
                computed
            }
            _ => true,
        }
    }

    /// The frame passed to `.join(...)`: a frame of new columns from `.assign(...)`, or
    /// something whose column names the file computes.
    fn joined_frame(
        &self,
        node: &Syntax<'r>,
        joined: &mut BTreeSet<usize>,
        found: &mut Found,
        depth: usize,
    ) -> bool {
        if depth > DEEPEST_NAME {
            return true;
        }
        if node.kind() == "identifier" {
            return self
                .locals
                .get(node.text().as_ref())
                .is_none_or(|value| self.joined_frame(value, joined, found, depth + 1));
        }
        if node.kind() == "parenthesized_expression" {
            return node
                .children()
                .find(Node::is_named)
                .is_none_or(|inner| self.joined_frame(&inner, joined, found, depth + 1));
        }
        if is_method_call(node, "assign") && is_new_frame(node) {
            joined.insert(node.range().start);
            self.record(node, Way::Join, found);
            return false;
        }
        // frame.where(mask) and the like keep the columns of the frame they are called on.
        let kept = node
            .field("function")
            .filter(|f| f.kind() == "attribute")
            .filter(|f| {
                f.field("attribute")
                    .is_some_and(|a| KEEPS_COLUMNS.contains(&a.text().as_ref()))
            })
            .and_then(|f| f.field("object"));
        kept.is_none_or(|object| self.joined_frame(&object, joined, found, depth + 1))
    }

    /// Records the keyword names of one `.assign(...)` call as columns.
    fn record(&self, assign: &Syntax<'r>, way: Way, found: &mut Found) {
        for (name, value) in keywords(assign) {
            found
                .formulas
                .push(self.formula(&name, &lambda_body(&value), Kind::Column));
            found.writes.push(Write {
                name,
                line: Self::line(&value),
                way,
            });
        }
    }

    fn formula(&self, name: &str, value: &Syntax<'r>, kind: Kind) -> Formula {
        Formula {
            name: name.to_owned(),
            line: Self::line(value),
            is_column: kind == Kind::Column,
            text: self.canonical(value, 0),
            source: value
                .text()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// The formula as text that is equal for equal formulas: local names are replaced by what
    /// they stand for, a column read is written `col[name]`, and the terms of a sum are sorted.
    fn canonical(&self, node: &Syntax<'r>, depth: usize) -> String {
        if depth > DEEPEST_NAME {
            return node.text().into_owned();
        }
        if let Some(name) = col_name(node) {
            return format!("col[{name}]");
        }
        match node.kind().as_ref() {
            "comment" => String::new(),
            "identifier" => match self.locals.get(node.text().as_ref()) {
                Some(value) => self.canonical(value, depth + 1),
                None => node.text().into_owned(),
            },
            "parenthesized_expression" => node
                .children()
                .find(Node::is_named)
                .map_or_else(String::new, |inner| self.canonical(&inner, depth + 1)),
            "subscript" => match self.frame_index(node).and_then(|i| self.names(&i, 0)) {
                Some(names) => format!("col[{}]", names.join(",")),
                None => self.joined_children(node, depth),
            },
            "binary_operator" => {
                let operator = node.field("operator").map(|o| o.text().into_owned());
                // a + b equals b + a, and a * b equals b * a, so their terms are sorted.
                if let Some(commutative) = operator.as_deref().filter(|o| matches!(*o, "+" | "*")) {
                    let mut terms = Vec::new();
                    self.terms(node, commutative, depth, &mut terms);
                    terms.sort();
                    return format!("({})", terms.join(&format!(" {commutative} ")));
                }
                match (node.field("left"), node.field("right"), operator) {
                    (Some(left), Some(right), Some(operator)) => format!(
                        "({} {operator} {})",
                        self.canonical(&left, depth + 1),
                        self.canonical(&right, depth + 1)
                    ),
                    _ => self.joined_children(node, depth),
                }
            }
            _ if node.is_leaf() || node.kind() == "string" => node.text().into_owned(),
            _ => self.joined_children(node, depth),
        }
    }

    fn joined_children(&self, node: &Syntax<'r>, depth: usize) -> String {
        node.children()
            .map(|child| self.canonical(&child, depth + 1))
            .collect()
    }

    /// The terms of `a + b + c`, or of `a * b * c`, each in canonical form.
    fn terms(&self, node: &Syntax<'r>, operator: &str, depth: usize, terms: &mut Vec<String>) {
        let inner = if node.kind() == "parenthesized_expression" {
            node.children().find(Node::is_named)
        } else {
            None
        };
        let node = inner.as_ref().unwrap_or(node);
        let local = (node.kind() == "identifier")
            .then(|| self.locals.get(node.text().as_ref()))
            .flatten();
        let node = local.unwrap_or(node);
        let same = node.kind() == "binary_operator"
            && node.field("operator").is_some_and(|o| o.text() == operator);
        match (same, node.field("left"), node.field("right")) {
            (true, Some(left), Some(right)) if depth <= DEEPEST_NAME => {
                self.terms(&left, operator, depth + 1, terms);
                self.terms(&right, operator, depth + 1, terms);
            }
            _ => terms.push(self.canonical(node, depth + 1)),
        }
    }
}

/// How many columns the name of a chain link lists before it ends in `...`.
const NAMED_COLUMNS: usize = 3;

/// Whether `node` is a call of `pd.col`.
fn is_col_call(node: &Syntax<'_>) -> bool {
    node.kind() == "call"
        && node
            .field("function")
            .is_some_and(|f| matches!(f.text().as_ref(), "pd.col" | "pandas.col"))
}

/// The column that `pd.col("name")` names. `None` for any other node, and for a name the file
/// computes.
fn col_name(node: &Syntax<'_>) -> Option<String> {
    if !is_col_call(node) {
        return None;
    }
    let arguments = node.field("arguments")?;
    let mut arguments = arguments.children().filter(Node::is_named);
    let name = arguments.next().filter(|a| a.kind() == "string")?;
    arguments.next().is_none().then(|| literal(&name)).flatten()
}

/// The one parameter of a lambda, as `d` in `lambda d: d["a"] * 2`.
fn lambda_parameter(lambda: &Syntax<'_>) -> Option<String> {
    let parameters = lambda.field("parameters")?;
    let mut parameters = parameters.children().filter(Node::is_named);
    let only = parameters.next().filter(|p| p.kind() == "identifier")?;
    parameters
        .next()
        .is_none()
        .then(|| only.text().into_owned())
}

/// The parameters of the lambdas that an `.assign(...)` call passes as column values. pandas
/// calls each of them with the frame.
fn lambda_frames(assign: &Syntax<'_>) -> Vec<String> {
    keywords(assign)
        .iter()
        .filter(|(_, value)| value.kind() == "lambda")
        .filter_map(|(_, value)| lambda_parameter(value))
        .collect()
}

/// What a column value computes: the body of a lambda, or the value itself.
fn lambda_body<'r>(value: &Syntax<'r>) -> Syntax<'r> {
    (value.kind() == "lambda")
        .then(|| value.field("body"))
        .flatten()
        .unwrap_or_else(|| value.clone())
}

/// The name a chain starts at, as `orders` in `orders.assign(...).assign(...)`.
fn chain_start(call: &Syntax<'_>) -> Option<String> {
    let mut object = receiver(call)?;
    while object.kind() == "call" {
        let below = receiver(&object)?;
        object = below;
    }
    (object.kind() == "identifier").then(|| object.text().into_owned())
}

/// Whether `node` is the call `something.method(...)`.
fn is_method_call(node: &Syntax<'_>, method: &str) -> bool {
    node.kind() == "call"
        && node.field("function").is_some_and(|f| {
            f.kind() == "attribute" && f.field("attribute").is_some_and(|a| a.text() == method)
        })
}

/// Whether an `.assign(...)` call starts from a frame with no columns, as in
/// `pd.DataFrame(index=frame.index).assign(...)`.
fn is_new_frame(assign: &Syntax<'_>) -> bool {
    assign
        .field("function")
        .and_then(|f| f.field("object"))
        .is_some_and(|object| {
            object.kind() == "call"
                && object
                    .field("function")
                    .is_some_and(|f| f.text().rsplit('.').next() == Some("DataFrame"))
        })
}

/// The `name=value` arguments of a call.
fn keywords<'r>(call: &Syntax<'r>) -> Vec<(String, Syntax<'r>)> {
    let Some(arguments) = call.field("arguments") else {
        return Vec::new();
    };
    arguments
        .children()
        .filter(|a| a.kind() == "keyword_argument")
        .filter_map(|a| Some((a.field("name")?.text().into_owned(), a.field("value")?)))
        .collect()
}

/// Whether a formula computes something from a column. A bare column read does not.
fn is_computation(text: &str) -> bool {
    let reads = text.matches("col[").count();
    let bare = text.starts_with("col[") && text.ends_with(']') && reads == 1;
    reads > 0 && !bare
}
