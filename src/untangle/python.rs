//! What `xpd untangle` reads from Python: the facts of one module, and the findings that
//! follow from the modules of a project together.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::imports::{self, Import};
use crate::syntax::{Node, Root};

use super::{Finding, Kind, Place};

/// The fewest commits of an imported module before its importer is compared with it. Below
/// this, the history is too short to call the module volatile.
const VOLATILE_COMMITS: usize = 8;
/// The fewest calls of a function before one caller's argument is called a special case.
const CALLS_FOR_ONE_CALLER: usize = 3;
/// How much of a condition is quoted in the evidence.
const QUOTED_CHARS: usize = 60;

/// One Python file of the project.
#[derive(Debug)]
pub(super) struct Module {
    /// The path as the ledger prints it.
    pub(super) shown: String,
    /// The absolute path, which an import resolves to.
    pub(super) file: PathBuf,
    /// The path below the top of the repository, which the history names.
    pub(super) tracked: Option<String>,
    pub(super) facts: Facts,
}

impl Module {
    /// The path that names the module in the id of a finding: the one git tracks, so that the
    /// id is the same at another commit.
    pub(super) fn key(&self) -> &str {
        self.tracked.as_deref().unwrap_or(&self.shown)
    }
}

/// What one Python file holds, read without the other files.
#[derive(Debug, Default)]
pub(super) struct Facts {
    imports: Vec<Import>,
    /// How often each identifier is written in the file.
    mentions: HashMap<String, usize>,
    /// How often each identifier is written in an import statement.
    import_mentions: HashMap<String, usize>,
    /// The strings that hold one identifier, as `__all__` and `getattr` name a definition.
    quoted: HashSet<String>,
    definitions: Vec<Definition>,
    /// The name of every `def`, also of a method and of a nested function.
    function_names: Vec<String>,
    calls: Vec<Call>,
    branches: Vec<Branch>,
    /// The findings that need no other file, each with its line and its evidence.
    pub(super) local: Vec<Local>,
}

/// A finding of one file: its kind, its line, its evidence and how strong the evidence is.
pub(super) type Local = (Kind, usize, String, usize);

/// A function or class at the top of a module.
#[derive(Debug)]
struct Definition {
    name: String,
    line: usize,
    decorated: bool,
    /// `None` for a class, and for a function with `*args` or `**kwargs`, whose calls cannot be
    /// matched to its parameters.
    parameters: Option<Vec<Parameter>>,
}

#[derive(Debug)]
struct Parameter {
    name: String,
    default: bool,
    /// The parameter can be passed by position.
    positional: bool,
}

/// An `if` that runs for one string value.
#[derive(Debug)]
struct Branch {
    line: usize,
    /// The comparison, as the source writes it.
    condition: String,
    /// The string it compares with, with its quotes.
    value: String,
    /// The other side is a plain name, such as a parameter.
    named: bool,
}

/// One call, by the last part of the name it calls.
#[derive(Debug)]
struct Call {
    name: String,
    line: usize,
    /// The text of each argument that is a literal, and `None` for any other argument.
    positional: Vec<Option<String>>,
    keywords: Vec<(String, Option<String>)>,
    /// The call unpacks `*args` or `**kwargs`, so its arguments cannot be matched.
    splat: bool,
}

/// Whether `path` is a test file, by the names pytest collects.
pub(super) fn is_test(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name == "conftest.py"
        || name.starts_with("test_")
        || name.ends_with("_test.py")
        || path.components().any(|part| part.as_os_str() == "tests")
}

fn private(name: &str) -> bool {
    name.starts_with('_') && !name.starts_with("__")
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}

/// `text` on one line, cut to the length the evidence quotes.
fn quoted(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= QUOTED_CHARS {
        return one_line;
    }
    let cut: String = one_line.chars().take(QUOTED_CHARS).collect();
    format!("{cut}…")
}

/// The facts of the Python file at `path`, whose parsed source is `root`. In a test, a branch
/// for one value is left out, because a test names the situations it checks, and a wait and a
/// retry are left to the rules of `xpd check`, which the ledger takes over.
pub(super) fn facts(path: &Path, root: &Root) -> Facts {
    let mut facts = Facts {
        imports: imports::imported(path, root),
        definitions: definitions(root),
        ..Facts::default()
    };
    for import in &facts.imports {
        if let Some(name) = import.name.as_deref().filter(|name| private(name)) {
            facts.local.push((
                Kind::PrivateReach,
                import.line,
                format!("This imports the private name {name} of another module."),
                1,
            ));
        }
    }
    let modules: HashSet<&str> = (facts.imports.iter())
        .filter(|import| import.name.is_none())
        .map(|import| import.bound.as_str())
        .collect();
    let in_test = is_test(path);
    let mut local = Vec::new();
    for node in root.root().dfs() {
        match node.kind().as_ref() {
            "identifier" => {
                let name = node.text().into_owned();
                let imported = node.ancestors().any(|above| {
                    matches!(
                        above.kind().as_ref(),
                        "import_statement" | "import_from_statement"
                    )
                });
                if imported {
                    *facts.import_mentions.entry(name.clone()).or_default() += 1;
                }
                *facts.mentions.entry(name).or_default() += 1;
            }
            "string_content" => {
                let content = node.text();
                if !content.is_empty() && content.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    facts.quoted.insert(content.into_owned());
                }
            }
            "function_definition" => {
                if let Some(name) = node.field("name") {
                    facts.function_names.push(name.text().into_owned());
                }
            }
            "call" => facts.calls.extend(call(&node)),
            "if_statement" if !in_test => facts.branches.extend(one_value(&node)),
            _ => {}
        }
        let waits = if in_test { None } else { wait(&node) };
        let found = waits
            .or_else(|| special_case(&node))
            .or_else(|| link(&node, &modules));
        local.extend(found.map(|(kind, evidence, weight)| (kind, line(&node), evidence, weight)));
    }
    facts.local.extend(local);
    facts
}

/// The functions and classes at the top of the module in `root`.
fn definitions(root: &Root) -> Vec<Definition> {
    let mut definitions = Vec::new();
    for item in root.root().children() {
        let decorated = item.kind() == "decorated_definition";
        let definition = if decorated {
            item.field("definition")
        } else {
            Some(item.clone())
        };
        let Some(definition) = definition else {
            continue;
        };
        let class = definition.kind() == "class_definition";
        if !class && definition.kind() != "function_definition" {
            continue;
        }
        let Some(name) = definition.field("name") else {
            continue;
        };
        definitions.push(Definition {
            name: name.text().into_owned(),
            line: line(&definition),
            decorated,
            parameters: if class { None } else { parameters(&definition) },
        });
    }
    definitions
}

/// The fixed wait or the retry that `node` is, with its evidence and the strength of it.
fn wait(node: &Node<'_>) -> Option<(Kind, String, usize)> {
    let evidence = match node.kind().as_ref() {
        "call" if sleeps(node) => "This waits a fixed time for something else to finish.",
        "decorator" => {
            let written = node.text().to_lowercase();
            (written.contains("retry") || written.contains("backoff"))
                .then_some("This runs the function again when it fails.")?
        }
        _ => return None,
    };
    Some((Kind::SleepOrRetry, evidence.to_owned(), 1))
}

/// The special case that `node` is, with its evidence and the strength of it.
fn special_case(node: &Node<'_>) -> Option<(Kind, String, usize)> {
    match node.kind().as_ref() {
        "except_clause" if swallows(node) => Some((
            Kind::SwallowedException,
            "This catches an exception and does nothing with it.".to_owned(),
            1,
        )),
        "with_item" => {
            let suppresses = node.field("value").is_some_and(|value| {
                value.kind() == "call"
                    && value.field("function").is_some_and(|function| {
                        matches!(function.text().as_ref(), "suppress" | "contextlib.suppress")
                    })
            });
            suppresses.then(|| {
                (
                    Kind::SwallowedException,
                    "This suppresses an exception and does nothing with it.".to_owned(),
                    1,
                )
            })
        }
        _ => None,
    }
}

/// The link that `node` makes to state outside its function or to the inside of another
/// module. `modules` holds the names that the file gives to the project modules it imports.
fn link(node: &Node<'_>, modules: &HashSet<&str>) -> Option<(Kind, String, usize)> {
    let of_module = |attribute: &Node<'_>| {
        let object = attribute.field("object")?;
        let name = attribute.field("attribute")?;
        modules
            .contains(object.text().as_ref())
            .then(|| (object.text().into_owned(), name.text().into_owned()))
    };
    match node.kind().as_ref() {
        "global_statement" => {
            let names: Vec<_> = (node.children())
                .filter(|child| child.kind() == "identifier")
                .map(|child| child.text().into_owned())
                .collect();
            Some((
                Kind::SharedState,
                format!(
                    "This function writes the module level name {}, which its callers do not pass.",
                    names.join(", ")
                ),
                1,
            ))
        }
        "assignment" | "augmented_assignment" => {
            let target = node
                .field("left")
                .filter(|left| left.kind() == "attribute")?;
            let (module, name) = of_module(&target)?;
            Some((
                Kind::SharedState,
                format!("This writes {name} of the module {module} from outside it."),
                2,
            ))
        }
        "attribute" => {
            let (module, name) = of_module(node).filter(|(_, name)| private(name))?;
            Some((
                Kind::PrivateReach,
                format!("This reads the private name {name} of the module {module}."),
                1,
            ))
        }
        _ => None,
    }
}

/// The parameters of the function `definition`. `None` when it takes `*args` or `**kwargs`.
fn parameters(definition: &Node<'_>) -> Option<Vec<Parameter>> {
    let mut parameters = Vec::new();
    let mut positional = true;
    for parameter in definition.field("parameters")?.children() {
        let (name, default) = match parameter.kind().as_ref() {
            "identifier" => (parameter.clone(), false),
            "typed_parameter" => {
                let name = parameter.child(0)?;
                if name.kind() != "identifier" {
                    return None;
                }
                (name, false)
            }
            "default_parameter" | "typed_default_parameter" => (parameter.field("name")?, true),
            "list_splat_pattern" | "dictionary_splat_pattern" => return None,
            "keyword_separator" => {
                positional = false;
                continue;
            }
            _ => continue,
        };
        parameters.push(Parameter {
            name: name.text().into_owned(),
            default,
            positional,
        });
    }
    Some(parameters)
}

/// The text of `node` when it is a literal.
fn literal(node: &Node<'_>) -> Option<String> {
    let kind = node.kind();
    let number = |node: &Node<'_>| matches!(node.kind().as_ref(), "integer" | "float");
    let plain = matches!(
        kind.as_ref(),
        "integer" | "float" | "true" | "false" | "none"
    ) || (kind == "string" && crate::syntax::string_literal(node).is_some())
        || (kind == "unary_operator" && node.field("argument").is_some_and(|n| number(&n)));
    plain.then(|| node.text().into_owned())
}

fn call(node: &Node<'_>) -> Option<Call> {
    let function = node.field("function")?;
    let name = match function.kind().as_ref() {
        "identifier" => function.text().into_owned(),
        "attribute" => function.field("attribute")?.text().into_owned(),
        _ => return None,
    };
    let mut call = Call {
        name,
        line: line(node),
        positional: Vec::new(),
        keywords: Vec::new(),
        splat: false,
    };
    let arguments = node.field("arguments")?;
    if arguments.kind() != "argument_list" {
        call.positional.push(None);
        return Some(call);
    }
    for argument in arguments.children().filter(Node::is_named) {
        match argument.kind().as_ref() {
            "comment" => {}
            "list_splat" | "dictionary_splat" => call.splat = true,
            "keyword_argument" => {
                let name = argument.field("name")?.text().into_owned();
                let value = argument.field("value").and_then(|value| literal(&value));
                call.keywords.push((name, value));
            }
            _ => call.positional.push(literal(&argument)),
        }
    }
    Some(call)
}

/// Whether `call` is `time.sleep(...)` or `asyncio.sleep(...)`, under either name.
fn sleeps(call: &Node<'_>) -> bool {
    call.field("function").is_some_and(|function| {
        matches!(
            function.text().as_ref(),
            "time.sleep" | "sleep" | "asyncio.sleep"
        )
    })
}

/// The `if` with no `elif` that compares something with one string. An `if` with `elif`
/// branches is a switch over the values, which is no special case.
fn one_value(statement: &Node<'_>) -> Option<Branch> {
    if (statement.children()).any(|child| child.kind() == "elif_clause") {
        return None;
    }
    let condition = statement.field("condition")?;
    condition.dfs().find_map(|node| {
        if node.kind() != "comparison_operator" {
            return None;
        }
        let children: Vec<_> = node.children().collect();
        let equality = (children.iter()).any(|child| matches!(child.kind().as_ref(), "==" | "!="));
        let main_guard = children.iter().any(|child| child.text() == "__name__");
        let value = (children.iter()).find(|child| child.kind() == "string")?;
        (equality && !main_guard).then(|| Branch {
            line: line(statement),
            condition: node.text().into_owned(),
            value: value.text().into_owned(),
            named: children.iter().any(|child| child.kind() == "identifier"),
        })
    })
}

/// Whether the `except` clause only passes, continues or holds `...`.
fn swallows(clause: &Node<'_>) -> bool {
    let Some(block) = clause.children().find(|child| child.kind() == "block") else {
        return false;
    };
    let mut statements = block
        .children()
        .filter(|child| child.is_named() && child.kind() != "comment")
        .peekable();
    statements.peek().is_some()
        && statements.all(|statement| match statement.kind().as_ref() {
            "pass_statement" | "continue_statement" => true,
            "expression_statement" => statement.text() == "...",
            _ => false,
        })
}

/// The findings that follow from the modules together. `changes` holds the commits that changed
/// each tracked file, newest first, and is empty without a history.
pub(super) fn findings(modules: &[Module], changes: &HashMap<&str, Vec<usize>>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let graph = Graph::new(modules);
    findings.extend(graph.volatile(changes));
    findings.extend(graph.cycles());
    let project = Project::new(modules);
    findings.extend(branches(modules));
    findings.extend(project.unreferenced());
    findings.extend(project.parameters());
    findings
}

/// The branches for one value, one finding for each value with every place that checks it. A
/// value that many places check is one decision of the design that is written out in each. The
/// same value in either kind of quotes is one value.
fn branches(modules: &[Module]) -> Vec<Finding> {
    let mut by_value: BTreeMap<String, Vec<(&Module, &Branch)>> = BTreeMap::new();
    for module in modules {
        for branch in &module.facts.branches {
            let value = branch.value.replace('\'', "\"");
            by_value.entry(value).or_default().push((module, branch));
        }
    }
    (by_value.into_iter())
        .map(|(value, found)| {
            let (_, first) = found[0];
            let files: BTreeSet<&str> = found
                .iter()
                .map(|(module, _)| module.shown.as_str())
                .collect();
            let evidence = if found.len() == 1 {
                format!(
                    "This branch runs for one value: {}.",
                    quoted(&first.condition)
                )
            } else {
                format!(
                    "{} branches in {} run for the value {value}, such as {}.",
                    found.len(),
                    if files.len() == 1 {
                        "one file".to_owned()
                    } else {
                        format!("{} files", files.len())
                    },
                    quoted(&first.condition)
                )
            };
            Finding {
                kind: Kind::OneValueBranch,
                key: value,
                places: (found.iter())
                    .map(|(module, branch)| Place::line(&module.shown, branch.line))
                    .collect(),
                evidence,
                weight: found.len() * 2 + usize::from(found.iter().any(|(_, branch)| branch.named)),
            }
        })
        .collect()
}

/// Each import between two tracked modules, as the importer and the imported module.
pub(super) fn imports(modules: &[Module]) -> HashSet<(&str, &str)> {
    let graph = Graph::new(modules);
    let tracked = |module: usize| modules[module].tracked.as_deref();
    (graph.imports.iter().enumerate())
        .flat_map(|(importer, imports)| {
            imports
                .keys()
                .filter_map(move |&imported| Some((tracked(importer)?, tracked(imported)?)))
        })
        .collect()
}

/// Which module imports which, by the index of each in the list of modules.
struct Graph<'m> {
    modules: &'m [Module],
    /// For each module, the modules it imports with the line of the first import of each.
    imports: Vec<BTreeMap<usize, usize>>,
}

impl<'m> Graph<'m> {
    fn new(modules: &'m [Module]) -> Self {
        let index: HashMap<&Path, usize> = (modules.iter().enumerate())
            .map(|(nth, module)| (module.file.as_path(), nth))
            .collect();
        let imports = (modules.iter().enumerate())
            .map(|(nth, module)| {
                let mut found = BTreeMap::new();
                for import in &module.facts.imports {
                    if let Some(&imported) = index.get(import.module.as_path())
                        && imported != nth
                    {
                        found.entry(imported).or_insert(import.line);
                    }
                }
                found
            })
            .collect();
        Self { modules, imports }
    }

    fn place(&self, module: usize, line: usize) -> Place {
        Place::line(&self.modules[module].shown, line)
    }

    /// Each import of a module that changed at least twice as often as its importer, counted
    /// from the first commit of the importer. A test is left out, because it follows the code it
    /// tests, and so is an `__init__.py`, which gives its own modules their public names.
    fn volatile(&self, changes: &HashMap<&str, Vec<usize>>) -> Vec<Finding> {
        let commits_of = |module: usize| {
            let tracked = self.modules[module].tracked.as_deref()?;
            changes.get(tracked)
        };
        let mut findings = Vec::new();
        for (importer, imports) in self.imports.iter().enumerate() {
            let file = &self.modules[importer].file;
            if is_test(file) || file.file_name().is_some_and(|name| name == "__init__.py") {
                continue;
            }
            for (&imported, &line) in imports {
                let (Some(stable), Some(volatile)) = (commits_of(importer), commits_of(imported))
                else {
                    continue;
                };
                let Some(&first) = stable.last() else {
                    continue;
                };
                let (stable, volatile) = (
                    stable.len(),
                    volatile.iter().filter(|&&commit| commit <= first).count(),
                );
                if volatile >= VOLATILE_COMMITS && volatile >= stable * 2 {
                    findings.push(Finding {
                        kind: Kind::VolatileImport,
                        key: format!(
                            "{}\n{}",
                            self.modules[importer].key(),
                            self.modules[imported].key()
                        ),
                        places: vec![self.place(importer, line)],
                        evidence: format!(
                            "{} changed in {stable} {} and imports {}, which changed in {volatile} commits in the same time.",
                            self.modules[importer].shown,
                            if stable == 1 { "commit" } else { "commits" },
                            self.modules[imported].shown
                        ),
                        weight: volatile / stable,
                    });
                }
            }
        }
        findings
    }

    /// Each group of modules that import one another in a circle, found as the strongly
    /// connected parts of the graph.
    fn cycles(&self) -> Vec<Finding> {
        let mut state = Tarjan {
            imports: &self.imports,
            order: vec![None; self.modules.len()],
            low: vec![0; self.modules.len()],
            stack: Vec::new(),
            on_stack: vec![false; self.modules.len()],
            next: 0,
            parts: Vec::new(),
        };
        for module in 0..self.modules.len() {
            if state.order[module].is_none() {
                state.visit(module);
            }
        }
        (state.parts.into_iter())
            .filter(|part| part.len() > 1)
            .map(|mut part| {
                part.sort_unstable();
                let places = (part.iter())
                    .map(|&module| {
                        let line = (self.imports[module].iter())
                            .find(|(imported, _)| part.contains(imported))
                            .map_or(1, |(_, line)| *line);
                        self.place(module, line)
                    })
                    .collect();
                Finding {
                    kind: Kind::ImportCycle,
                    key: (part.iter().map(|&module| self.modules[module].key()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    places,
                    evidence: format!(
                        "These {} modules import one another in a circle, so none of them can change alone.",
                        part.len()
                    ),
                    weight: part.len(),
                }
            })
            .collect()
    }
}

/// Tarjan's search for the strongly connected parts of the import graph.
struct Tarjan<'g> {
    imports: &'g [BTreeMap<usize, usize>],
    order: Vec<Option<usize>>,
    low: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    next: usize,
    parts: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, module: usize) {
        let order = self.next;
        self.next += 1;
        self.order[module] = Some(order);
        self.low[module] = order;
        self.stack.push(module);
        self.on_stack[module] = true;
        let imports = self.imports;
        for &imported in imports[module].keys() {
            match self.order[imported] {
                None => {
                    self.visit(imported);
                    self.low[module] = self.low[module].min(self.low[imported]);
                }
                Some(seen) if self.on_stack[imported] => {
                    self.low[module] = self.low[module].min(seen);
                }
                Some(_) => {}
            }
        }
        if self.low[module] == order {
            let mut part = Vec::new();
            while let Some(member) = self.stack.pop() {
                self.on_stack[member] = false;
                part.push(member);
                if member == module {
                    break;
                }
            }
            self.parts.push(part);
        }
    }
}

/// The names of a project, counted over all of its modules.
struct Project<'m> {
    modules: &'m [Module],
    mentions: HashMap<&'m str, usize>,
    import_mentions: HashMap<&'m str, usize>,
    quoted: HashSet<&'m str>,
    /// How many `def` statements carry each name.
    defined: HashMap<&'m str, usize>,
    /// The calls of each name, with the module of each.
    calls: HashMap<&'m str, Vec<(&'m Module, &'m Call)>>,
}

impl<'m> Project<'m> {
    fn new(modules: &'m [Module]) -> Self {
        let mut project = Self {
            modules,
            mentions: HashMap::new(),
            import_mentions: HashMap::new(),
            quoted: HashSet::new(),
            defined: HashMap::new(),
            calls: HashMap::new(),
        };
        for module in modules {
            let facts = &module.facts;
            for (name, count) in &facts.mentions {
                *project.mentions.entry(name).or_default() += count;
            }
            for (name, count) in &facts.import_mentions {
                *project.import_mentions.entry(name).or_default() += count;
            }
            project
                .quoted
                .extend(facts.quoted.iter().map(String::as_str));
            for name in &facts.function_names {
                *project.defined.entry(name).or_default() += 1;
            }
            for call in &facts.calls {
                (project.calls.entry(&call.name).or_default()).push((module, call));
            }
        }
        project
    }

    /// Each function or class that no file mentions but the one line that defines it. A
    /// decorated definition is left out, because a decorator can register it, and so is a test
    /// and a name that a string holds, as `__all__` does.
    fn unreferenced(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        for module in self.modules {
            let skipped = is_test(&module.file)
                || module
                    .file
                    .file_name()
                    .is_some_and(|name| name == "__init__.py");
            if skipped {
                continue;
            }
            for definition in &module.facts.definitions {
                let name = definition.name.as_str();
                let known = definition.decorated
                    || name == "main"
                    || name.starts_with("__")
                    || self.quoted.contains(name);
                if !known && self.mentions.get(name) == Some(&1) {
                    findings.push(Finding {
                        kind: Kind::Unreferenced,
                        key: format!("{}\n{name}", module.key()),
                        places: vec![Place::line(&module.shown, definition.line)],
                        evidence: format!(
                            "No Python file that was read mentions {name}, apart from this definition."
                        ),
                        weight: 1,
                    });
                }
            }
        }
        findings
    }

    /// The parameters that every call gives the same value, and the ones that one call alone
    /// passes. Only a function whose every use is a call that can be matched is judged.
    fn parameters(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        for module in self.modules {
            for definition in &module.facts.definitions {
                let Some(parameters) = &definition.parameters else {
                    continue;
                };
                let name = definition.name.as_str();
                let calls = self.calls.get(name).map_or(&[][..], Vec::as_slice);
                let mentions = self.mentions.get(name).copied().unwrap_or(0);
                let imported = self.import_mentions.get(name).copied().unwrap_or(0);
                let only_called = self.defined.get(name) == Some(&1)
                    && mentions == 1 + imported + calls.len()
                    && !self.quoted.contains(name);
                if !only_called || calls.len() < 2 {
                    continue;
                }
                let Some(passed) = passed(parameters, calls) else {
                    continue;
                };
                let at = Place::line(&module.shown, definition.line);
                for (parameter, values) in parameters.iter().zip(&passed) {
                    let count = calls.len();
                    let parameter_name = &parameter.name;
                    let key = format!("{}\n{name}\n{parameter_name}", module.key());
                    if values.is_empty() && parameter.default {
                        findings.push(Finding {
                            kind: Kind::ConstantParameter,
                            key: key.clone(),
                            places: vec![at.clone()],
                            evidence: format!(
                                "None of the {count} calls of {name} passes {parameter_name}, so it always has its default."
                            ),
                            weight: count,
                        });
                    } else if values.len() == count
                        && let Some(value) = &values[0].1
                        && values
                            .iter()
                            .all(|(_, other)| other.as_ref() == Some(value))
                    {
                        findings.push(Finding {
                            kind: Kind::ConstantParameter,
                            key: key.clone(),
                            places: vec![at.clone()],
                            evidence: format!(
                                "Each of the {count} calls of {name} passes {parameter_name}={value}."
                            ),
                            weight: count,
                        });
                    } else if values.len() == 1
                        && parameter.default
                        && count >= CALLS_FOR_ONE_CALLER
                    {
                        let (module, call) = calls[values[0].0];
                        findings.push(Finding {
                            kind: Kind::OneCallerParameter,
                            key: key.clone(),
                            places: vec![at.clone(), Place::line(&module.shown, call.line)],
                            evidence: format!(
                                "One of the {count} calls of {name} passes {parameter_name}, and the others leave it at its default."
                            ),
                            weight: count,
                        });
                    }
                }
            }
        }
        findings
    }
}

/// One call that passes a parameter: the index of the call, and the literal it passes.
type Passed = (usize, Option<String>);

/// For each parameter, the calls that pass it. `None` when a call cannot be matched to the parameters.
fn passed(parameters: &[Parameter], calls: &[(&Module, &Call)]) -> Option<Vec<Vec<Passed>>> {
    let mut passed = vec![Vec::new(); parameters.len()];
    let positional = parameters.iter().filter(|p| p.positional).count();
    let names: BTreeSet<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    for (nth, (_, call)) in calls.iter().enumerate() {
        let fits = !call.splat
            && call.positional.len() <= positional
            && (call.keywords.iter()).all(|(name, _)| names.contains(name.as_str()));
        if !fits {
            return None;
        }
        for (slot, value) in call.positional.iter().enumerate() {
            passed[slot].push((nth, value.clone()));
        }
        for (name, value) in &call.keywords {
            let slot = parameters.iter().position(|p| p.name == *name)?;
            passed[slot].push((nth, value.clone()));
        }
    }
    Some(passed)
}
