//! The work that Python code does again or for nothing, read from the syntax of one file at a
//! time: a call that a function computes more than once, a call that a loop repeats, a keyword
//! argument that a library may already default to, and a matrix that is built to be reduced.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use crate::source;
use crate::syntax::{Node, Root};

use super::Kind;

/// How many characters of code the evidence quotes.
const QUOTED_CHARS: usize = 60;

/// Calls that cost nothing worth saving, or that build a value each caller changes on its own.
const CHEAP: &[&str] = &[
    "abs",
    "bool",
    "callable",
    "cast",
    "decode",
    "dict",
    "encode",
    "endswith",
    "enumerate",
    "filter",
    "float",
    "format",
    "frozenset",
    "get",
    "getattr",
    "hasattr",
    "id",
    "int",
    "isinstance",
    "issubclass",
    "items",
    "keys",
    "len",
    "list",
    "lower",
    "lstrip",
    "map",
    "max",
    "min",
    "range",
    "repr",
    "reversed",
    "round",
    "rstrip",
    "set",
    "startswith",
    "str",
    "strip",
    "super",
    "tuple",
    "type",
    "upper",
    "values",
    "zip",
];

/// Calls whose result differs from one call to the next, or that are made for their effect.
const IMPURE: &[&str] = &[
    "acquire",
    "add",
    "append",
    "call",
    "check_call",
    "check_output",
    "choice",
    "clear",
    "close",
    "commit",
    "connect",
    "copy",
    "critical",
    "debug",
    "deepcopy",
    "delete",
    "emit",
    "error",
    "exception",
    "execute",
    "executemany",
    "extend",
    "fetch",
    "flush",
    "get_event_loop",
    "info",
    "input",
    "insert",
    "iter",
    "log",
    "monotonic",
    "next",
    "normal",
    "now",
    "open",
    "perf_counter",
    "pop",
    "popleft",
    "post",
    "print",
    "put",
    "rand",
    "randint",
    "randn",
    "random",
    "read",
    "readline",
    "recv",
    "release",
    "remove",
    "request",
    "run",
    "sample",
    "seed",
    "send",
    "setdefault",
    "shuffle",
    "sleep",
    "sort",
    "start",
    "submit",
    "system",
    "time",
    "today",
    "uniform",
    "update",
    "urlopen",
    "uuid4",
    "wait",
    "warn",
    "warning",
    "write",
];

/// Calls that give each caller a new value to fill.
const FRESH: &[&str] = &[
    "DataFrame",
    "Series",
    "array",
    "defaultdict",
    "deque",
    "empty",
    "empty_like",
    "full",
    "full_like",
    "ones",
    "ones_like",
    "zeros",
    "zeros_like",
];

/// Calls that reduce an array along an axis.
const REDUCERS: &[&str] = &[
    "all",
    "any",
    "argmax",
    "argmin",
    "average",
    "count_nonzero",
    "logsumexp",
    "max",
    "mean",
    "median",
    "min",
    "nanmax",
    "nanmean",
    "nanmin",
    "nansum",
    "prod",
    "std",
    "sum",
    "var",
];

/// Calls that contract an array with another, like matrix multiplication.
const CONTRACTORS: &[&str] = &["dot", "einsum", "matmul", "tensordot"];

/// Calls that return a boolean mask over an array or frame.
const MASK_CALLS: &[&str] = &[
    "between",
    "contains",
    "duplicated",
    "endswith",
    "fullmatch",
    "isfinite",
    "isin",
    "isinf",
    "isna",
    "isnan",
    "isnull",
    "match",
    "notna",
    "notnull",
    "startswith",
];

/// Fewest elements in a list literal inside a loop before membership testing is worth a set.
const LINEAR_LIST_LEN: usize = 4;

/// A finding that one file holds on its own.
#[derive(Debug)]
pub(super) struct Local {
    pub(super) kind: Kind,
    /// The lines of its places, the first of them first.
    pub(super) lines: Vec<usize>,
    /// What it is about, without a line, so that it has the same key at another commit.
    pub(super) key: String,
    pub(super) evidence: String,
    pub(super) weight: usize,
}

/// What a call names as the thing it calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Callee {
    /// A name of a library with its module, as `numpy.linspace`.
    Symbol(String),
    /// A method of a value whose class the file does not say, as `groupby`.
    Method(String),
}

/// A keyword argument with a literal value, which may repeat the default of the callee.
#[derive(Debug)]
pub(super) struct Keyword {
    pub(super) line: usize,
    pub(super) callee: Callee,
    pub(super) name: String,
    pub(super) value: String,
}

/// What one Python file gives the ledger.
#[derive(Debug, Default)]
pub(super) struct Facts {
    pub(super) local: Vec<Local>,
    pub(super) keywords: Vec<Keyword>,
    /// The packages the file imports, by the first part of the module name.
    pub(super) packages: BTreeSet<String>,
    /// The names of the functions the file defines.
    pub(super) functions: Vec<String>,
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}

/// The text of `node` on one line.
fn one_line(node: &Node<'_>) -> String {
    node.text().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `text` cut to the length the evidence quotes.
fn quoted(text: &str) -> String {
    if text.chars().count() <= QUOTED_CHARS {
        return text.to_owned();
    }
    let cut: String = text.chars().take(QUOTED_CHARS).collect();
    format!("{cut}…")
}

/// `lines` as the evidence names them: `line 12`, `lines 12 and 15`, or `lines 12, 15 and 20`.
fn listed(lines: &[usize]) -> String {
    let distinct: BTreeSet<usize> = lines.iter().copied().collect();
    let text: Vec<String> = distinct.iter().map(usize::to_string).collect();
    match text.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("lines {} and {last}", rest.join(", ")),
        _ => format!("line {}", text.join("")),
    }
}

/// The nodes below `node` that run in its own scope, without the bodies of the functions,
/// classes and lambdas inside it.
fn scope<'t>(node: &Node<'t>, nodes: &mut Vec<Node<'t>>) {
    for child in node.children() {
        if matches!(
            child.kind().as_ref(),
            "function_definition" | "class_definition" | "lambda"
        ) {
            continue;
        }
        nodes.push(child.clone());
        scope(&child, nodes);
    }
}

/// The name at the left end of `node`, as `frame` in `frame.loc[rows].sum`.
fn root_name(node: &Node<'_>) -> Option<String> {
    match node.kind().as_ref() {
        "identifier" => Some(node.text().into_owned()),
        "attribute" => root_name(&node.field("object")?),
        "subscript" => root_name(&node.field("value")?),
        "call" => root_name(&node.field("function")?),
        "parenthesized_expression" => root_name(&node.child(1)?),
        _ => None,
    }
}

/// The names that `target`, the left side of an assignment or a loop, binds or changes.
fn targets(target: &Node<'_>, names: &mut Vec<String>) {
    match target.kind().as_ref() {
        "identifier" | "attribute" | "subscript" => names.extend(root_name(target)),
        _ => {
            for child in target.children() {
                targets(&child, names);
            }
        }
    }
}

/// The last name of what `call` calls, as `groupby` in `frame.groupby(keys)`.
fn called(call: &Node<'_>) -> Option<String> {
    let function = call.field("function")?;
    match function.kind().as_ref() {
        "identifier" => Some(function.text().into_owned()),
        "attribute" => Some(function.field("attribute")?.text().into_owned()),
        _ => None,
    }
}

/// The names that `node` changes when it runs, apart from the ones it only reads.
fn writes(node: &Node<'_>) -> Vec<String> {
    let mut names = Vec::new();
    match node.kind().as_ref() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
            if let Some(left) = node.field("left") {
                targets(&left, &mut names);
            }
        }
        "named_expression" => names.extend(node.field("name").and_then(|name| root_name(&name))),
        "as_pattern" => {
            if let Some(alias) = node.field("alias") {
                targets(&alias, &mut names);
            }
        }
        "delete_statement" | "global_statement" | "nonlocal_statement" => {
            for child in node.children() {
                targets(&child, &mut names);
            }
        }
        "import_statement" | "import_from_statement" => {
            for (bound, _) in imports_of(node) {
                names.push(bound);
            }
        }
        "expression_statement" => {
            let called = node.child(0).filter(|call| call.kind() == "call");
            let object = called
                .and_then(|call| call.field("function"))
                .filter(|function| function.kind() == "attribute");
            names.extend(object.and_then(|function| root_name(&function)));
        }
        "call" => {
            let changes = called(node).is_some_and(|name| IMPURE.contains(&name.as_str()));
            let object = (node.field("function")).filter(|function| function.kind() == "attribute");
            if changes {
                names.extend(object.and_then(|function| root_name(&function)));
            }
        }
        _ => {}
    }
    names
}

/// The names that `node` reads: each identifier in it that is not the name of an attribute or
/// of a keyword argument.
fn reads(node: &Node<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    for inner in node.dfs().filter(|inner| inner.kind() == "identifier") {
        let Some(parent) = inner.parent() else {
            continue;
        };
        let field = |name: &str| {
            parent
                .field(name)
                .is_some_and(|held| held.range() == inner.range())
        };
        let label = match parent.kind().as_ref() {
            "attribute" => field("attribute"),
            "keyword_argument" => field("name"),
            _ => false,
        };
        if !label {
            names.insert(inner.text().into_owned());
        }
    }
    names
}

/// Whether `call` is worth computing once: it is no cheap built-in, it builds no object for its
/// caller, as a class does, and its name does not say that its result changes from one call to
/// the next.
fn pure(call: &Node<'_>) -> bool {
    let Some(name) = called(call) else {
        return false;
    };
    let written = call
        .field("function")
        .map_or_else(String::new, |function| function.text().to_lowercase());
    !CHEAP.contains(&name.as_str())
        && !IMPURE.contains(&name.as_str())
        && !FRESH.contains(&name.as_str())
        && !name.starts_with("__")
        && !name.starts_with(char::is_uppercase)
        && ![
            "random",
            "rng",
            "uuid",
            "mock",
            "logger",
            "logging",
            "subprocess",
            "requests",
            "httpx",
            "socket",
            "cursor",
            "session",
            "client",
        ]
        .iter()
        .any(|word| written.contains(word))
        && call.parent().is_none_or(|parent| parent.kind() != "await")
}

/// The arms of the branches around `node` and below `function`, each with the place of its
/// statement. A node in the condition of a branch is in none of its arms.
fn arms(node: &Node<'_>, function: &Node<'_>) -> Vec<(usize, usize)> {
    let mut arms = Vec::new();
    let mut below = node.clone();
    for above in node.ancestors() {
        if above.range() == function.range() {
            break;
        }
        let inside =
            |field: &str| (above.field(field)).is_some_and(|held| held.range() == below.range());
        let arm = match above.kind().as_ref() {
            "if_statement" if inside("consequence") => Some((above.range().start, 0)),
            "elif_clause" | "else_clause" | "case_clause" if !inside("condition") => {
                let statement = above
                    .parent()
                    .map_or(0, |statement| statement.range().start);
                Some((statement, above.range().start))
            }
            "conditional_expression" => {
                let at = (above.children()).position(|child| child.range() == below.range());
                at.filter(|at| *at != 2).map(|at| (above.range().start, at))
            }
            _ => None,
        };
        arms.extend(arm);
        below = above;
    }
    arms
}

/// Whether two nodes are in different arms of one branch, so that one run reaches one of them.
fn exclusive(a: &[(usize, usize)], b: &[(usize, usize)]) -> bool {
    (a.iter()).any(|(statement, arm)| {
        b.iter()
            .any(|(other, taken)| statement == other && arm != taken)
    })
}

/// Whether `node` is on the left side of an assignment, loop target or delete statement.
fn is_lhs(node: &Node<'_>) -> bool {
    let mut child = node.clone();
    for above in node.ancestors() {
        match above.kind().as_ref() {
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                if above
                    .field("left")
                    .is_some_and(|left| left.range() == child.range())
                {
                    return true;
                }
                return false;
            }
            "delete_statement" => return true,
            "pattern_list" | "tuple_pattern" | "list_pattern" | "parenthesized_expression" => {
                child = above;
            }
            _ => return false,
        }
    }
    false
}

/// Whether `node` builds a boolean mask over an array, series or frame.
fn is_mask_expr(node: &Node<'_>) -> bool {
    match node.kind().as_ref() {
        "comparison_operator" | "boolean_operator" | "not_operator" => true,
        "binary_operator" => node
            .field("operator")
            .is_some_and(|op| matches!(op.text().as_ref(), "&" | "|" | "^")),
        "unary_operator" => node
            .field("operator")
            .is_some_and(|op| op.text().as_ref() == "~"),
        "parenthesized_expression" => node
            .children()
            .find(Node::is_named)
            .is_some_and(|inner| is_mask_expr(&inner)),
        "call" => called(node).is_some_and(|name| MASK_CALLS.contains(&name.as_str())),
        _ => false,
    }
}

/// Whether `node` allocates a Python list value.
fn is_list_expr(node: &Node<'_>) -> bool {
    match node.kind().as_ref() {
        "list" | "list_comprehension" => true,
        "call" => called(node).is_some_and(|name| matches!(name.as_str(), "list" | "tolist")),
        _ => false,
    }
}

/// Whether `inner` lies completely inside the byte range of `around`.
fn within(inner: &Node<'_>, around: &Node<'_>) -> bool {
    let (inner, around) = (inner.range(), around.range());
    around.start <= inner.start && inner.end <= around.end
}

/// One function, with the nodes of its own scope.
struct Function<'t> {
    node: Node<'t>,
    name: String,
    nodes: Vec<Node<'t>>,
    /// Each name the function changes, with the line of the change.
    writes: Vec<(String, usize)>,
}

impl<'t> Function<'t> {
    fn new(node: Node<'t>) -> Option<Self> {
        let name = node.field("name")?.text().into_owned();
        let mut nodes = Vec::new();
        scope(&node.field("body")?, &mut nodes);
        let writes = (nodes.iter())
            .flat_map(|inner| (writes(inner).into_iter()).map(|name| (name, line(inner))))
            .collect();
        Some(Self {
            node,
            name,
            nodes,
            writes,
        })
    }

    fn calls(&self) -> impl Iterator<Item = &Node<'t>> {
        (self.nodes.iter()).filter(|node| node.kind() == "call" && pure(node))
    }

    /// Whether a node of `kinds` around `node` in this function has one of the texts `found`.
    fn inside_any(&self, node: &Node<'_>, kinds: &[&str], found: &HashSet<String>) -> bool {
        (node.ancestors())
            .take_while(|above| above.range() != self.node.range())
            .any(|above| {
                kinds.contains(&above.kind().as_ref()) && found.contains(&one_line(&above))
            })
    }

    /// Lines of the first run of occurrences of one expression with nothing it reads written in
    /// between and not split across mutually exclusive branch arms.
    fn repeated_lines(&self, nodes: Vec<&Node<'t>>) -> Option<Vec<usize>> {
        if nodes.len() < 2 {
            return None;
        }
        let read = reads(nodes[0]);
        let changed: Vec<usize> = (self.writes.iter())
            .filter(|(name, _)| read.contains(name))
            .map(|(_, line)| *line)
            .collect();
        let mut runs: Vec<Vec<&Node<'t>>> = vec![Vec::new()];
        for node in nodes {
            let split = runs
                .last()
                .and_then(|run| run.last())
                .is_some_and(|before| {
                    (changed.iter()).any(|at| line(before) <= *at && *at < line(node))
                });
            if split {
                runs.push(Vec::new());
            }
            runs.last_mut().expect("runs starts with one").push(node);
        }
        runs.into_iter().find_map(|run| {
            let paths: Vec<_> = run.iter().map(|node| arms(node, &self.node)).collect();
            (0..run.len()).find_map(|first| {
                let lines: Vec<usize> = (first..run.len())
                    .filter(|other| !exclusive(&paths[first], &paths[*other]))
                    .map(|other| line(run[other]))
                    .collect();
                (lines.len() > 1).then_some(lines)
            })
        })
    }

    /// The calls that the function computes more than once with nothing changed in between.
    fn repeated(&self) -> (Vec<Local>, HashSet<String>) {
        let mut by_text: HashMap<String, Vec<&Node<'t>>> = HashMap::new();
        for call in self.calls() {
            by_text.entry(one_line(call)).or_default().push(call);
        }
        let mut found: Vec<(String, Vec<usize>)> = Vec::new();
        for (text, calls) in by_text {
            if let Some(lines) = self.repeated_lines(calls) {
                found.push((text, lines));
            }
        }
        let texts: HashSet<String> = found.iter().map(|(text, _)| text.clone()).collect();
        let outermost = |text: &str| {
            (self.calls())
                .filter(|call| one_line(call) == text)
                .any(|call| !self.inside_any(call, &["call"], &texts))
        };
        let locals = (found.iter())
            .filter(|(text, _)| outermost(text))
            .map(|(text, lines)| Local {
                kind: Kind::RepeatedCall,
                lines: lines.clone(),
                key: format!("{}\n{text}", self.name),
                evidence: format!(
                    "{} computes `{}` {} times, on {}, and changes nothing that the call reads in between.",
                    self.name,
                    quoted(text),
                    lines.len(),
                    listed(lines)
                ),
                weight: lines.len() * text.len(),
            })
            .collect();
        (locals, texts)
    }

    /// Boolean masks and masked subscripts computed more than once with nothing written in
    /// between. Plain integer indexing such as `arr[i]` is left out.
    fn repeated_masks(&self, covered_calls: &HashSet<String>) -> Vec<Local> {
        let mut mask_vars: HashSet<String> = HashSet::new();
        for node in &self.nodes {
            if node.kind() == "assignment"
                && let Some(left) = node.field("left").filter(|l| l.kind() == "identifier")
                && let Some(right) = node.field("right")
                && is_mask_expr(&right)
            {
                mask_vars.insert(left.text().into_owned());
            }
        }
        let mut candidates: Vec<&Node<'t>> = Vec::new();
        for node in &self.nodes {
            match node.kind().as_ref() {
                "subscript" if !is_lhs(node) => {
                    let Some(idx) = node.field("subscript") else {
                        continue;
                    };
                    let fancy = is_mask_expr(&idx)
                        || (idx.kind() == "identifier" && mask_vars.contains(idx.text().as_ref()))
                        || idx.kind() == "list";
                    if fancy {
                        candidates.push(node);
                    }
                }
                "comparison_operator" => {
                    let in_mask_use = node
                        .ancestors()
                        .find(|above| above.kind() != "parenthesized_expression")
                        .is_some_and(|parent| {
                            matches!(
                                parent.kind().as_ref(),
                                "subscript" | "argument_list" | "keyword_argument"
                            )
                        });
                    if in_mask_use {
                        candidates.push(node);
                    }
                }
                _ => {}
            }
        }
        let mut by_text: HashMap<String, Vec<&Node<'t>>> = HashMap::new();
        for node in &candidates {
            by_text.entry(one_line(node)).or_default().push(node);
        }
        let mut found: Vec<(String, Vec<usize>)> = Vec::new();
        for (text, nodes) in by_text {
            if let Some(lines) = self.repeated_lines(nodes) {
                found.push((text, lines));
            }
        }
        let mut covered: HashSet<String> = found.iter().map(|(text, _)| text.clone()).collect();
        covered.extend(covered_calls.iter().cloned());
        let kinds = ["call", "subscript", "comparison_operator"];
        let outermost = |text: &str| {
            (candidates.iter())
                .filter(|node| one_line(node) == text)
                .any(|node| !self.inside_any(node, &kinds, &covered))
        };
        (found.iter())
            .filter(|(text, _)| outermost(text))
            .map(|(text, lines)| Local {
                kind: Kind::RepeatedMask,
                lines: lines.clone(),
                key: format!("{}\n{text}", self.name),
                evidence: format!(
                    "{} evaluates mask or masked slice `{}` {} times, on {}, and changes nothing it reads in between.",
                    self.name,
                    quoted(text),
                    lines.len(),
                    listed(lines)
                ),
                weight: lines.len() * text.len(),
            })
            .collect()
    }

    /// The `for` loop or comprehension of this function that repeats `node`. A call in a `raise`
    /// or a `return` ends the loop, so it runs once. A `while` loop waits for something outside
    /// it to change, so a node in it is left out.
    fn loop_around(&self, node: &Node<'t>) -> Option<Node<'t>> {
        let mut below = node.clone();
        for above in node.ancestors() {
            if above.range() == self.node.range() {
                return None;
            }
            if matches!(
                above.kind().as_ref(),
                "raise_statement" | "return_statement"
            ) {
                return None;
            }
            match above.kind().as_ref() {
                "for_statement" => {
                    let once =
                        (above.field("right")).is_some_and(|right| right.range() == below.range());
                    if !once {
                        return Some(above);
                    }
                }
                "list_comprehension"
                | "dictionary_comprehension"
                | "set_comprehension"
                | "generator_expression" => {
                    let first_iterable = (above.children())
                        .find(|child| child.kind() == "for_in_clause")
                        .and_then(|clause| clause.field("right"));
                    let in_first_iterable = first_iterable.is_some_and(|iter| within(node, &iter));
                    if !in_first_iterable {
                        return Some(above);
                    }
                }
                "while_statement" => return None,
                _ => {}
            }
            below = above;
        }
        None
    }

    /// The calls that a loop or comprehension repeats though it changes nothing they read. A call
    /// with no argument counts only at the end of a chain of calls, as `.mean()` after a
    /// `groupby`, because such a call on a plain name often reads state that the loop changes.
    fn invariant(&self) -> Vec<Local> {
        let mut found: Vec<(Node<'t>, usize)> = Vec::new();
        for call in self.calls() {
            let used = call
                .parent()
                .is_none_or(|parent| parent.kind() != "expression_statement");
            let chained = (call.field("function"))
                .filter(|function| function.kind() == "attribute")
                .and_then(|function| function.field("object"))
                .is_some_and(|object| {
                    object
                        .dfs()
                        .any(|inner| matches!(inner.kind().as_ref(), "call" | "subscript"))
                });
            let arguments = chained
                || (call.field("arguments")).is_some_and(|arguments| {
                    arguments.children().any(|argument| argument.is_named())
                });
            if !used || !arguments {
                continue;
            }
            let Some(repeating) = self.loop_around(call) else {
                continue;
            };
            let mut inside = vec![repeating.clone()];
            scope(&repeating, &mut inside);
            let changed: HashSet<String> = inside.iter().flat_map(writes).collect();
            if reads(call).is_disjoint(&changed) {
                found.push((call.clone(), line(&repeating)));
            }
        }
        let texts: HashSet<String> = found.iter().map(|(call, _)| one_line(call)).collect();
        let mut seen = HashSet::new();
        (found.iter())
            .filter(|(call, _)| !self.inside_any(call, &["call"], &texts))
            .filter(|(call, _)| seen.insert(one_line(call)))
            .map(|(call, repeating)| {
                let text = one_line(call);
                Local {
                    kind: Kind::LoopInvariantCall,
                    lines: vec![line(call)],
                    key: format!("{}\n{text}", self.name),
                    evidence: format!(
                        "The loop on line {repeating} of {} computes `{}` on every pass, and changes nothing that the call reads.",
                        self.name,
                        quoted(&text)
                    ),
                    weight: text.len(),
                }
            })
            .collect()
    }

    /// Full diffs, sorts, argsorts or cumulative sums built only to read one element or a small
    /// head/tail slice.
    fn materialized_for_one(&self) -> Vec<Local> {
        let mut found = Vec::new();
        for node in &self.nodes {
            if node.kind() == "subscript" && !is_lhs(node) {
                let Some(value) = node.field("value") else {
                    continue;
                };
                let Some(idx) = node.field("subscript") else {
                    continue;
                };
                let idx_text = one_line(&idx);
                let reason = if value.kind() == "call" {
                    match (called(&value).as_deref(), idx_text.as_str()) {
                        (Some("diff"), "0" | "-1") => Some(format!(
                            "builds the full diff array only to take element {idx_text}"
                        )),
                        (Some("sorted" | "sort"), "0" | "-1") => Some(format!(
                            "sorts the whole input only to take element {idx_text}, which min or max finds in one pass"
                        )),
                        (Some("argsort"), "0" | "-1") => Some(format!(
                            "computes a full argsort only to take element {idx_text}, which argmin or argmax finds in one pass"
                        )),
                        (Some("cumsum"), "-1") => Some(
                            "builds the full cumulative sum only to take its last element, which sum computes directly"
                                .to_owned(),
                        ),
                        _ => None,
                    }
                } else if value.kind() == "attribute"
                    && value
                        .field("attribute")
                        .is_some_and(|attr| attr.text().as_ref() == "iloc")
                    && value.field("object").is_some_and(|obj| {
                        obj.kind() == "call" && called(&obj).as_deref() == Some("sort_values")
                    })
                    && matches!(idx_text.as_str(), "0" | "-1")
                {
                    Some(format!(
                        "sorts the whole frame only to take row {idx_text}, which idxmin, idxmax, nsmallest or nlargest avoids"
                    ))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    let text = one_line(node);
                    found.push(Local {
                        kind: Kind::MaterializedForOne,
                        lines: vec![line(node)],
                        key: format!("{}\n{text}", self.name),
                        evidence: format!("`{}` in {} {reason}.", quoted(&text), self.name),
                        weight: text.len(),
                    });
                }
            } else if node.kind() == "call"
                && matches!(called(node).as_deref(), Some("head" | "tail"))
                && let Some(function) = node.field("function")
                && function.kind() == "attribute"
                && function.field("object").is_some_and(|obj| {
                    obj.kind() == "call" && called(&obj).as_deref() == Some("sort_values")
                })
                && let Some(args) = node.field("arguments")
            {
                let count = args
                    .children()
                    .find(Node::is_named)
                    .and_then(|arg| arg.text().parse::<usize>().ok())
                    .unwrap_or(5);
                if count <= 20 {
                    let text = one_line(node);
                    found.push(Local {
                        kind: Kind::MaterializedForOne,
                        lines: vec![line(node)],
                        key: format!("{}\n{text}", self.name),
                        evidence: format!(
                            "`{}` in {} sorts the whole frame only to keep {count} rows, which nsmallest or nlargest avoids.",
                            quoted(&text),
                            self.name
                        ),
                        weight: text.len(),
                    });
                }
            }
        }
        found
    }

    /// O(n*m) scans inside loops: membership tests against a list, or filtering a whole frame on
    /// every iteration instead of grouping once.
    fn linear_scans(&self) -> Vec<Local> {
        let mut list_vars: HashSet<String> = HashSet::new();
        for node in &self.nodes {
            if node.kind() == "assignment"
                && let Some(left) = node.field("left").filter(|l| l.kind() == "identifier")
                && let Some(right) = node.field("right")
                && is_list_expr(&right)
            {
                list_vars.insert(left.text().into_owned());
            }
        }
        let mut found = Vec::new();
        let mut seen = HashSet::new();
        for node in &self.nodes {
            let Some(repeating) = self.loop_around(node) else {
                continue;
            };
            let mut inside = vec![repeating.clone()];
            scope(&repeating, &mut inside);
            let changed: HashSet<String> = inside.iter().flat_map(writes).collect();
            if node.kind() == "comparison_operator" {
                let has_in = node
                    .children()
                    .any(|child| matches!(child.text().as_ref(), "in" | "not in"));
                let rhs = node.children().filter(Node::is_named).last();
                if has_in && let Some(rhs) = rhs {
                    let linear_literal = rhs.kind() == "list"
                        && rhs.children().filter(Node::is_named).count() >= LINEAR_LIST_LEN;
                    let linear_var = rhs.kind() == "identifier"
                        && list_vars.contains(rhs.text().as_ref())
                        && !changed.contains(rhs.text().as_ref());
                    if linear_literal || linear_var {
                        let text = one_line(node);
                        if seen.insert((line(node), text.clone())) {
                            found.push(Local {
                                kind: Kind::LinearScanInLoop,
                                lines: vec![line(node)],
                                key: format!("{}\n{text}", self.name),
                                evidence: format!(
                                    "The loop on line {} of {} tests `{}` against a list on every pass; a set checks membership in constant time.",
                                    line(&repeating),
                                    self.name,
                                    quoted(&text)
                                ),
                                weight: text.len(),
                            });
                        }
                    }
                }
            } else if node.kind() == "subscript"
                && !is_lhs(node)
                && let Some(value) = node.field("value")
                && let Some(idx) = node.field("subscript")
                && idx.kind() == "comparison_operator"
            {
                let frame = if value.kind() == "attribute"
                    && value
                        .field("attribute")
                        .is_some_and(|attr| attr.text().as_ref() == "loc")
                {
                    value.field("object").unwrap_or(value)
                } else {
                    value
                };
                let Some(frame_name) = root_name(&frame) else {
                    continue;
                };
                if changed.contains(&frame_name) {
                    continue;
                }
                let filters_self = idx.dfs().any(|inner| {
                    inner.kind() == "subscript"
                        && inner
                            .field("value")
                            .and_then(|v| root_name(&v))
                            .is_some_and(|name| name == frame_name)
                });
                let depends_on_loop = !reads(&idx).is_disjoint(&changed);
                if filters_self && depends_on_loop {
                    let text = one_line(node);
                    if seen.insert((line(node), text.clone())) {
                        found.push(Local {
                            kind: Kind::LinearScanInLoop,
                            lines: vec![line(node)],
                            key: format!("{}\n{text}", self.name),
                            evidence: format!(
                                "The loop on line {} of {} filters `{frame_name}` with `{}` on every pass; grouping once avoids rescanning the frame.",
                                line(&repeating),
                                self.name,
                                quoted(&text)
                            ),
                            weight: text.len() * 2,
                        });
                    }
                }
            }
        }
        found
    }

    /// Every outermost expression of the function that sets every element of one array against
    /// every element of another, as `prices[:, None] - bids`.
    fn outers(&self) -> Vec<Node<'t>> {
        let adds_axis = |subscript: &Node<'_>| {
            subscript
                .field_children("subscript")
                .any(|index| index.kind() == "none" || index.text().ends_with("newaxis"))
        };
        let mut found: Vec<Node<'t>> = Vec::new();
        for node in self
            .nodes
            .iter()
            .filter(|node| node.kind() == "subscript" && adds_axis(node))
        {
            let mut whole = None;
            for above in node.ancestors() {
                match above.kind().as_ref() {
                    "binary_operator" | "comparison_operator" => whole = Some(above),
                    "parenthesized_expression" | "unary_operator" => {}
                    _ => break,
                }
            }
            if let Some(whole) = whole
                && !found
                    .iter()
                    .any(|existing| existing.range() == whole.range())
            {
                found.push(whole);
            }
        }
        found
    }

    /// Whether `node` reduces or contracts an array (a reduction call or `@` matrix multiply).
    fn reduction_kind(node: &Node<'_>) -> Option<bool> {
        match node.kind().as_ref() {
            "binary_operator"
                if node
                    .field("operator")
                    .is_some_and(|op| op.text().as_ref() == "@") =>
            {
                Some(true)
            }
            "call" => {
                let name = called(node)?;
                if CONTRACTORS.contains(&name.as_str()) {
                    return Some(true);
                }
                if REDUCERS.contains(&name.as_str()) {
                    let has_axis = (node.field("arguments")).is_some_and(|arguments| {
                        (arguments.children())
                            .filter(|argument| argument.kind() == "keyword_argument")
                            .any(|argument| {
                                (argument.field("name")).is_some_and(|n| n.text() == "axis")
                            })
                    });
                    return Some(has_axis);
                }
                None
            }
            _ => None,
        }
    }
}

/// A function that returns a matrix of every element against every other: its line and text.
type Outer = (usize, String);

/// Whether `text` calls the helper named `helper`, directly or as a method.
fn calls_helper(text: &str, helper: &str) -> bool {
    text.contains(&format!(".{helper}(")) || text.starts_with(&format!("{helper}("))
}

/// Whether `node` reads the tainted variable `name` as an array value (not as an attribute
/// container like `solved.fun`, where `solved` is a result object).
fn uses_array(node: &Node<'_>, name: &str) -> bool {
    for inner in node.dfs().filter(|inner| inner.kind() == "identifier") {
        if inner.text().as_ref() != name {
            continue;
        }
        let Some(parent) = inner.parent() else {
            return true;
        };
        if parent.kind() == "attribute"
            && parent
                .field("object")
                .is_some_and(|obj| obj.range() == inner.range())
        {
            let attr = parent
                .field("attribute")
                .map(|attr| attr.text().into_owned());
            if attr
                .as_deref()
                .is_some_and(|a| REDUCERS.contains(&a) || matches!(a, "T" | "real" | "imag"))
            {
                return true;
            }
            continue;
        }
        if parent.kind() == "keyword_argument"
            && parent
                .field("name")
                .is_some_and(|kw| kw.range() == inner.range())
        {
            continue;
        }
        return true;
    }
    false
}

/// Whether `rhs` transforms a tainted matrix into another matrix of the same shape (rather than
/// passing it as an initial guess to a solver or optimizer).
fn preserves_matrix(rhs: &Node<'_>, name: &str) -> bool {
    if !uses_array(rhs, name) {
        return false;
    }
    match rhs.kind().as_ref() {
        "binary_operator"
        | "unary_operator"
        | "comparison_operator"
        | "subscript"
        | "parenthesized_expression" => true,
        "call" => !called(rhs).is_some_and(|c| {
            matches!(
                c.as_str(),
                "least_squares"
                    | "minimize"
                    | "linprog"
                    | "solve_ivp"
                    | "root"
                    | "brentq"
                    | "fit"
                    | "fit_transform"
            )
        }),
        _ => false,
    }
}

/// Functions that return an outer-broadcast matrix, mapped to its build line and text.
fn returned_outers(functions: &[Function<'_>]) -> HashMap<String, Outer> {
    let mut returned: HashMap<String, Outer> = HashMap::new();
    for function in functions {
        let outers = function.outers();
        if outers.is_empty() {
            continue;
        }
        let mut bound: HashMap<String, Outer> = HashMap::new();
        for node in &function.nodes {
            if node.kind() == "assignment"
                && let Some(left) = node.field("left").filter(|l| l.kind() == "identifier")
                && let Some(right) = node.field("right")
            {
                if let Some(outer) = outers.iter().find(|o| within(o, &right)) {
                    bound.insert(left.text().into_owned(), (line(outer), one_line(outer)));
                } else if let Some(hit) = bound
                    .iter()
                    .find_map(|(name, info)| preserves_matrix(&right, name).then(|| info.clone()))
                {
                    bound.insert(left.text().into_owned(), hit);
                }
            }
            if node.kind() == "return_statement" {
                if let Some(outer) = outers.iter().find(|o| within(o, node)) {
                    returned.insert(function.name.clone(), (line(outer), one_line(outer)));
                    break;
                }
                if let Some(hit) = bound
                    .iter()
                    .find_map(|(name, info)| uses_array(node, name).then(|| info.clone()))
                {
                    returned.insert(function.name.clone(), hit);
                    break;
                }
            }
        }
    }
    returned
}

/// The matrices that the functions build and reduce or contract, in one function or in a caller
/// of the function that returns the matrix, tracking local variable bindings in between.
fn reduced(functions: &[Function<'_>]) -> Vec<Local> {
    let returned = returned_outers(functions);
    let mut found = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for function in functions {
        let outers = function.outers();
        let mut tainted: HashMap<String, (usize, String, String)> = HashMap::new();
        for node in &function.nodes {
            if node.kind() == "assignment"
                && let Some(left) = node.field("left").filter(|l| l.kind() == "identifier")
                && let Some(right) = node.field("right")
            {
                let right_text = right.text();
                if let Some(outer) = outers.iter().find(|o| within(o, &right)) {
                    tainted.insert(
                        left.text().into_owned(),
                        (line(outer), one_line(outer), String::new()),
                    );
                } else if let Some((helper, (built, text))) = returned
                    .iter()
                    .find(|(helper, _)| calls_helper(&right_text, helper))
                {
                    tainted.insert(
                        left.text().into_owned(),
                        (*built, text.clone(), format!(", through {helper}")),
                    );
                } else if let Some(hit) = tainted
                    .iter()
                    .find_map(|(name, info)| preserves_matrix(&right, name).then(|| info.clone()))
                {
                    tainted.insert(left.text().into_owned(), hit);
                }
            }
            let Some(explicit_sink) = Function::reduction_kind(node) else {
                continue;
            };
            let node_text = node.text();
            let source = if let Some(outer) = outers.iter().find(|o| within(o, node)) {
                Some((line(outer), one_line(outer), String::new()))
            } else if let Some((helper, (built, text))) = returned
                .iter()
                .find(|(helper, _)| calls_helper(&node_text, helper))
            {
                Some((*built, text.clone(), format!(", through {helper}")))
            } else if let Some(hit) = tainted
                .iter()
                .find_map(|(name, info)| uses_array(node, name).then(|| info.clone()))
            {
                Some(hit)
            } else if explicit_sink && node.kind() == "call" {
                outers
                    .iter()
                    .find(|outer| line(node) > outer.end_pos().line() + 1 && !within(node, outer))
                    .map(|outer| (line(outer), one_line(outer), String::new()))
            } else {
                None
            };
            if let Some((built, text, by)) = source
                && seen.insert((function.name.clone(), text.clone()))
            {
                let at = line(node);
                found.push(Local {
                    kind: Kind::MatrixReduced,
                    lines: vec![built, at],
                    key: format!("{}\n{text}", function.name),
                    evidence: format!(
                        "`{}` holds one value for each pair of elements of its two sides, and line {at} of {} reduces it{by}.",
                        quoted(&text),
                        function.name
                    ),
                    weight: text.len(),
                });
            }
        }
    }
    found
}

/// The names that the import statement `import` binds, each with the module or the name of a
/// module that it stands for. A relative import is left out.
fn imports_of(import: &Node<'_>) -> Vec<(String, String)> {
    let dotted = |name: &Node<'_>| -> Option<(String, Option<String>)> {
        match name.kind().as_ref() {
            "dotted_name" => Some((name.text().into_owned(), None)),
            "aliased_import" => Some((
                name.field("name")?.text().into_owned(),
                Some(name.field("alias")?.text().into_owned()),
            )),
            _ => None,
        }
    };
    let names = import
        .field_children("name")
        .filter_map(|name| dotted(&name));
    match import.kind().as_ref() {
        "import_statement" => names
            .map(|(module, alias)| {
                let package = module.split('.').next().unwrap_or(&module).to_owned();
                alias.map_or_else(
                    || (package.clone(), package.clone()),
                    |alias| (alias, module.clone()),
                )
            })
            .collect(),
        "import_from_statement" => {
            let module = (import.field("module_name"))
                .filter(|module| module.kind() == "dotted_name")
                .map(|module| module.text().into_owned());
            let Some(module) = module else {
                return Vec::new();
            };
            names
                .map(|(name, alias)| {
                    let full = format!("{module}.{name}");
                    (alias.unwrap_or(name), full)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The text of `node` when it is a literal that a default could equal.
fn literal(node: &Node<'_>) -> Option<String> {
    let number = |node: &Node<'_>| matches!(node.kind().as_ref(), "integer" | "float");
    let plain = match node.kind().as_ref() {
        "integer" | "float" | "true" | "false" | "none" => true,
        "string" => crate::syntax::string_literal(node).is_some(),
        "unary_operator" => node.field("argument").is_some_and(|inner| number(&inner)),
        _ => false,
    };
    plain.then(|| node.text().into_owned())
}

/// What `call` calls, by the imports of the file. `None` for a call of the object itself.
fn callee(call: &Node<'_>, imports: &HashMap<String, String>) -> Option<Callee> {
    let function = call.field("function")?;
    let dotted = function.text();
    let plain = (dotted.split('.'))
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '_'));
    let root = root_name(&function)?;
    if let (true, Some(module)) = (plain, imports.get(&root)) {
        let rest = dotted.strip_prefix(root.as_str()).unwrap_or("");
        return Some(Callee::Symbol(format!("{module}{rest}")));
    }
    let method = function.kind() == "attribute" && !matches!(root.as_str(), "self" | "cls");
    method.then(|| called(call).map(Callee::Method)).flatten()
}

/// The facts of the Python file at `path`, whose parsed source is `root`. A test repeats a call
/// to say what it checks, so the calls of a test file are left out.
pub(super) fn facts(path: &Path, root: &Root) -> Facts {
    let mut facts = Facts::default();
    let mut imports = HashMap::new();
    let mut functions = Vec::new();
    for node in root.root().dfs() {
        match node.kind().as_ref() {
            "import_statement" | "import_from_statement" => {
                for (bound, module) in imports_of(&node) {
                    facts
                        .packages
                        .extend(module.split('.').next().map(str::to_owned));
                    imports.insert(bound, module);
                }
            }
            "function_definition" => functions.extend(Function::new(node)),
            _ => {}
        }
    }
    for call in root.root().dfs().filter(|node| node.kind() == "call") {
        let Some(arguments) = call.field("arguments") else {
            continue;
        };
        let keywords: Vec<(String, String)> = (arguments.children())
            .filter(|argument| argument.kind() == "keyword_argument")
            .filter_map(|argument| {
                let value = literal(&argument.field("value")?)?;
                Some((argument.field("name")?.text().into_owned(), value))
            })
            .collect();
        if keywords.is_empty() {
            continue;
        }
        let Some(callee) = callee(&call, &imports) else {
            continue;
        };
        facts
            .keywords
            .extend(keywords.into_iter().map(|(name, value)| Keyword {
                line: line(&call),
                callee: callee.clone(),
                name,
                value,
            }));
    }
    let test = source::is_test(path);
    for function in &functions {
        if !test {
            let (repeated, covered_calls) = function.repeated();
            facts.local.extend(repeated);
            facts.local.extend(function.repeated_masks(&covered_calls));
            facts.local.extend(function.invariant());
            facts.local.extend(function.materialized_for_one());
            facts.local.extend(function.linear_scans());
        }
        facts.functions.push(function.name.clone());
    }
    facts.local.extend(reduced(&functions));
    facts
}
