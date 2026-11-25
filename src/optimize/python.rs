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

    /// Whether a call of this function around `call` has one of the texts `found`.
    fn inside(&self, call: &Node<'_>, found: &HashSet<String>) -> bool {
        (call.ancestors())
            .take_while(|above| above.range() != self.node.range())
            .any(|above| above.kind() == "call" && found.contains(&one_line(&above)))
    }

    /// The calls that the function computes more than once with nothing changed in between.
    fn repeated(&self) -> Vec<Local> {
        let mut by_text: HashMap<String, Vec<&Node<'t>>> = HashMap::new();
        for call in self.calls() {
            by_text.entry(one_line(call)).or_default().push(call);
        }
        let mut found: Vec<(String, Vec<usize>)> = Vec::new();
        for (text, calls) in by_text {
            if calls.len() < 2 {
                continue;
            }
            let read = reads(calls[0]);
            let changed: Vec<usize> = (self.writes.iter())
                .filter(|(name, _)| read.contains(name))
                .map(|(_, line)| *line)
                .collect();
            let mut runs: Vec<Vec<&Node<'t>>> = vec![Vec::new()];
            for call in calls {
                let split = runs
                    .last()
                    .and_then(|run| run.last())
                    .is_some_and(|before| {
                        (changed.iter()).any(|at| line(before) <= *at && *at < line(call))
                    });
                if split {
                    runs.push(Vec::new());
                }
                runs.last_mut().expect("runs starts with one").push(call);
            }
            let together = runs.into_iter().find_map(|run| {
                let paths: Vec<_> = run.iter().map(|call| arms(call, &self.node)).collect();
                (0..run.len()).find_map(|first| {
                    let lines: Vec<usize> = (first..run.len())
                        .filter(|other| !exclusive(&paths[first], &paths[*other]))
                        .map(|other| line(run[other]))
                        .collect();
                    (lines.len() > 1).then_some(lines)
                })
            });
            found.extend(together.map(|lines| (text, lines)));
        }
        let texts: HashSet<String> = found.iter().map(|(text, _)| text.clone()).collect();
        let outermost = |text: &str| {
            (self.calls())
                .filter(|call| one_line(call) == text)
                .any(|call| !self.inside(call, &texts))
        };
        (found.iter())
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
            .collect()
    }

    /// The `for` loop of this function that repeats `call`. A call in a `raise` or a `return`
    /// ends the loop, so it runs once. A `while` loop waits for something outside it to change,
    /// so a call in it is left out.
    fn loop_around(&self, call: &Node<'t>) -> Option<Node<'t>> {
        let mut below = call.clone();
        for above in call.ancestors() {
            if above.range() == self.node.range() {
                return None;
            }
            if matches!(
                above.kind().as_ref(),
                "raise_statement" | "return_statement"
            ) {
                return None;
            }
            let once = above.kind() == "for_statement"
                && (above.field("right")).is_some_and(|right| right.range() == below.range());
            match above.kind().as_ref() {
                "for_statement" if !once => return Some(above),
                "while_statement" => return None,
                _ => {}
            }
            below = above;
        }
        None
    }

    /// The calls that a loop repeats though it changes nothing they read. A call with no
    /// argument counts only at the end of a chain of calls, as `.mean()` after a `groupby`,
    /// because such a call on a plain name often reads state that the loop changes.
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
            .filter(|(call, _)| !self.inside(call, &texts))
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

    /// The first expression of the function that sets every element of one array against every
    /// element of another, as `prices[:, None] - bids`.
    fn outer(&self) -> Option<Node<'t>> {
        let adds_axis = |subscript: &Node<'_>| {
            subscript
                .field_children("subscript")
                .any(|index| index.kind() == "none" || index.text().ends_with("newaxis"))
        };
        (self.nodes.iter())
            .filter(|node| node.kind() == "subscript" && adds_axis(node))
            .find_map(|node| {
                let mut whole = None;
                for above in node.ancestors() {
                    match above.kind().as_ref() {
                        "binary_operator" | "comparison_operator" => whole = Some(above),
                        "parenthesized_expression" | "unary_operator" => {}
                        _ => break,
                    }
                }
                whole
            })
    }

    /// The calls of the function that reduce an array along an axis.
    fn reductions(&self) -> impl Iterator<Item = &Node<'t>> {
        (self.nodes.iter()).filter(|node| {
            node.kind() == "call"
                && called(node).is_some_and(|name| REDUCERS.contains(&name.as_str()))
                && (node.field("arguments")).is_some_and(|arguments| {
                    (arguments.children())
                        .filter(|argument| argument.kind() == "keyword_argument")
                        .any(|argument| {
                            (argument.field("name")).is_some_and(|name| name.text() == "axis")
                        })
                })
        })
    }
}

/// A function that returns a matrix of every element against every other: its name, the line
/// and the text of the expression.
type Outer = (String, usize, String);

/// The matrices that the functions build and reduce along an axis, in one function or in a
/// caller of the function that returns the matrix.
fn reduced(functions: &[Function<'_>]) -> Vec<Local> {
    let finding = |function: &Function<'_>, built: usize, text: &str, at: usize, by: &str| Local {
        kind: Kind::MatrixReduced,
        lines: vec![built, at],
        key: format!("{}\n{text}", function.name),
        evidence: format!(
            "`{}` holds one value for each pair of elements of its two sides, and line {at} of {} reduces it along an axis{by}.",
            quoted(text),
            function.name
        ),
        weight: text.len(),
    };
    let mut found = Vec::new();
    let mut returned: Vec<Outer> = Vec::new();
    for function in functions {
        let Some(outer) = function.outer() else {
            continue;
        };
        let text = one_line(&outer);
        let within = |inner: &Node<'_>, around: &Node<'_>| {
            let (inner, around) = (inner.range(), around.range());
            around.start <= inner.start && inner.end <= around.end
        };
        let reduction = function.reductions().find(|call| {
            within(&outer, call)
                || (line(call) > outer.end_pos().line() + 1 && !within(call, &outer))
        });
        if let Some(reduction) = reduction {
            found.push(finding(function, line(&outer), &text, line(reduction), ""));
        } else if (outer.ancestors()).any(|above| above.kind() == "return_statement") {
            returned.push((function.name.clone(), line(&outer), text));
        }
    }
    for (name, built, text) in &returned {
        for function in functions {
            let calls = |reduction: &&Node<'_>| {
                let written = reduction.text();
                written.contains(&format!(".{name}(")) || written.starts_with(&format!("{name}("))
            };
            if let Some(reduction) = function.reductions().find(calls) {
                let by = format!(", through {name}");
                found.push(finding(function, *built, text, line(reduction), &by));
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
            facts.local.extend(function.repeated());
            facts.local.extend(function.invariant());
        }
        facts.functions.push(function.name.clone());
    }
    facts.local.extend(reduced(&functions));
    facts
}
