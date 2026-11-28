//! What `xpd untangle` reads from Python: the facts of one module, and the findings that
//! follow from the modules of a project together.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::imports::{self, Import, ImportScope};
use crate::ledger::python::local_nodes;
use crate::names::{is_bound, is_parameter_name};
use crate::syntax::{Node, Root};

use super::zones::{EntryPoints, Zone, classify};
use super::{Finding, Kind, Place};

/// The fewest commits of an imported module before its importer is compared with it. Below
/// this, the history is too short to call the module volatile.
const VOLATILE_COMMITS: usize = 8;
/// The fewest calls of a function before one caller's argument is called a special case.
const CALLS_FOR_ONE_CALLER: usize = 3;
/// How much of a condition is quoted in the evidence.
const QUOTED_CHARS: usize = 60;
/// Minimum lines before a module with zero importers is flagged as orphaned.
const ORPHANED_MIN_LOC: usize = 10;
/// Maximum importers before a re-export facade is treated as an established public surface.
const FACADE_MAX_IMPORTERS: usize = 20;
/// Minimum lines for a single-use helper module to be flagged as an inlining candidate.
const SINGLE_USE_MIN_LOC: usize = 20;
/// Maximum lines for a single-use helper module to be flagged as an inlining candidate.
const SINGLE_USE_MAX_LOC: usize = 300;
/// Minimum lines before a module is checked for disconnected responsibility clusters.
const COHESION_MIN_LOC: usize = 300;
/// Minimum top-level functions before a module is checked for disconnected responsibility clusters.
const COHESION_MIN_FUNCTIONS: usize = 10;

/// Call names that construct mutable containers at module level.
const MUTABLE_CONSTRUCTORS: &[&str] = &[
    "set",
    "list",
    "dict",
    "defaultdict",
    "OrderedDict",
    "Counter",
    "deque",
];

/// Methods that mutate a container in place without rebinding its name.
const MUTATING_METHODS: &[&str] = &[
    "append",
    "extend",
    "insert",
    "pop",
    "remove",
    "clear",
    "update",
    "setdefault",
    "add",
    "discard",
    "popitem",
    "sort",
    "reverse",
];

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
#[derive(Debug)]
pub(super) struct Facts {
    zone: Zone,
    module_name: Option<String>,
    loc: usize,
    has_dunder_all: bool,
    has_main_guard: bool,
    is_facade: bool,
    imports: Vec<Import>,
    /// How often each identifier is written in the file.
    mentions: HashMap<String, usize>,
    /// How often each identifier is written in an import statement.
    import_mentions: HashMap<String, usize>,
    /// Identifiers referenced by module top-level code outside top-level definitions.
    top_level_mentions: HashSet<String>,
    /// The strings that hold one identifier, as `__all__` and `getattr` name a definition.
    quoted: HashSet<String>,
    /// Dotted module paths written as strings, such as `"pkg.plugins.foo"`.
    quoted_modules: HashSet<String>,
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

/// Whether a top-level definition is a function or a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefKind {
    Function,
    Class,
}

/// A function or class at the top of a module.
#[derive(Debug)]
struct Definition {
    kind: DefKind,
    name: String,
    line: usize,
    decorated: bool,
    /// `None` for a class, and for a function with `*args` or `**kwargs`, whose calls cannot be
    /// matched to its parameters.
    parameters: Option<Vec<Parameter>>,
    /// Identifiers referenced inside this definition or its decorators.
    references: HashSet<String>,
    /// Top-level import root packages referenced inside a function definition.
    import_roots: BTreeSet<String>,
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

fn private(name: &str) -> bool {
    name.starts_with('_') && !name.starts_with("__")
}

fn is_init_py(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "__init__.py")
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}

/// `text` on one line, cut to the length the evidence quotes.
fn quoted(text: &str) -> String {
    crate::output::quote_one_line(text, QUOTED_CHARS)
}

/// The facts of the Python file at `path`, whose parsed source is `root` and text is `text`.
/// `exists` checks whether a resolved import target file exists in the surveyed tree or commit.
pub(super) fn facts(path: &Path, root: &Root, text: &str, exists: impl Fn(&Path) -> bool) -> Facts {
    let zone = classify(path);
    let module_root = root.root();
    let import_alias_roots = top_level_import_roots(&module_root);
    let top_summary = inspect_top_level(&module_root);
    let mut facts = Facts {
        zone,
        module_name: imports::module_name(path, &exists),
        loc: text.lines().count(),
        has_dunder_all: top_summary.has_dunder_all,
        has_main_guard: top_summary.has_main_guard,
        is_facade: top_summary.is_facade,
        imports: imports::imported_with(path, root, &exists),
        mentions: HashMap::new(),
        import_mentions: HashMap::new(),
        top_level_mentions: top_summary.mentions,
        quoted: HashSet::new(),
        quoted_modules: HashSet::new(),
        definitions: definitions(root, &import_alias_roots),
        function_names: Vec::new(),
        calls: Vec::new(),
        branches: Vec::new(),
        local: Vec::new(),
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
    let skip_branches = zone == Zone::Test || zone.is_generated_or_vendor();
    let mut local = Vec::new();
    for node in module_root.dfs() {
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
                let trimmed = content.trim_start_matches('.');
                if is_identifier_like(trimmed) {
                    facts.quoted.insert(trimmed.to_owned());
                } else if is_dotted_module_string(trimmed) {
                    facts.quoted_modules.insert(trimmed.to_owned());
                }
            }
            "function_definition" => {
                if let Some(name) = node.field("name") {
                    facts.function_names.push(name.text().into_owned());
                }
            }
            "call" => facts.calls.extend(call(&node)),
            "if_statement" if !skip_branches => facts.branches.extend(one_value(&node)),
            _ => {}
        }
        let found = link(&node, &modules, zone);
        local.extend(found.map(|(kind, evidence, weight)| (kind, line(&node), evidence, weight)));
    }
    local.extend(mutable_container_mutations(&module_root, zone));
    facts.local.extend(local);
    facts
}

fn is_identifier_like(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_dotted_module_string(text: &str) -> bool {
    text.contains('.') && text.split('.').all(is_identifier_like)
}

/// Summary of top-level statements in one Python module.
struct TopSummary {
    has_dunder_all: bool,
    has_main_guard: bool,
    is_facade: bool,
    mentions: HashSet<String>,
}

/// Inspects top-level statements for `__all__`, `__main__` guards, facade shape, and references
/// outside top-level function and class definitions.
fn inspect_top_level(module: &Node<'_>) -> TopSummary {
    let mut has_dunder_all = false;
    let mut has_main_guard = false;
    let mut has_reexport_import = false;
    let mut only_facade_items = true;
    let mut mentions = HashSet::new();

    for child in module.children() {
        let kind = child.kind();
        match kind.as_ref() {
            "comment" | "future_import_statement" => {}
            "import_statement" | "import_from_statement" => {
                has_reexport_import = true;
            }
            "function_definition" | "class_definition" | "decorated_definition" => {
                only_facade_items = false;
            }
            "expression_statement" => {
                collect_identifiers(&child, &mut mentions);
                let Some(expr) = child.named_children().next() else {
                    only_facade_items = false;
                    continue;
                };
                match expr.kind().as_ref() {
                    "string" => {}
                    "assignment" | "augmented_assignment"
                        if expr.field("left").is_some_and(|left| {
                            left.kind() == "identifier" && left.text() == "__all__"
                        }) =>
                    {
                        has_dunder_all = true;
                    }
                    _ => {
                        only_facade_items = false;
                    }
                }
            }
            "if_statement" => {
                only_facade_items = false;
                if is_main_guard_if(&child) {
                    has_main_guard = true;
                }
                collect_identifiers(&child, &mut mentions);
            }
            _ => {
                if child.is_named() {
                    only_facade_items = false;
                    collect_identifiers(&child, &mut mentions);
                }
            }
        }
    }

    TopSummary {
        has_dunder_all,
        has_main_guard,
        is_facade: has_reexport_import && only_facade_items,
        mentions,
    }
}

fn collect_identifiers(node: &Node<'_>, into: &mut HashSet<String>) {
    for item in node.dfs() {
        if item.kind() == "identifier" {
            into.insert(item.text().into_owned());
        }
    }
}

fn is_main_guard_if(statement: &Node<'_>) -> bool {
    let Some(condition) = statement.field("condition") else {
        return false;
    };
    let mut saw_name = false;
    let mut saw_main = false;
    for node in condition.dfs() {
        if node.kind() == "identifier" && node.text() == "__name__" {
            saw_name = true;
        } else if node.kind() == "string_content" && node.text() == "__main__" {
            saw_main = true;
        }
    }
    saw_name && saw_main
}

/// Maps each local name bound by a top-level import to its root package or module name.
fn top_level_import_roots(module: &Node<'_>) -> HashMap<String, String> {
    let mut roots = HashMap::new();
    for child in module.children() {
        match child.kind().as_ref() {
            "import_statement" => {
                for name in child.field_children("name") {
                    if let Some((local, root)) = import_alias_root(&name, None) {
                        roots.insert(local, root);
                    }
                }
            }
            "import_from_statement" => {
                let from_root = child.field("module_name").and_then(|node| {
                    let raw = node.text();
                    let trimmed = raw.trim_start_matches('.');
                    let first = trimmed.split('.').next().unwrap_or("");
                    (!first.is_empty()).then(|| first.to_owned())
                });
                for name in child.field_children("name") {
                    if let Some((local, root)) = import_alias_root(&name, from_root.as_deref()) {
                        roots.insert(local, root);
                    }
                }
            }
            _ => {}
        }
    }
    roots
}

fn import_alias_root(node: &Node<'_>, from_root: Option<&str>) -> Option<(String, String)> {
    match node.kind().as_ref() {
        "dotted_name" => {
            let text = node.text();
            let first = text.split('.').next()?.trim();
            let root = from_root.unwrap_or(first);
            Some((first.to_owned(), root.to_owned()))
        }
        "aliased_import" => {
            let target = node.field("name")?.text();
            let first = target.split('.').next()?.trim();
            let alias = node.field("alias")?.text().into_owned();
            let root = from_root.unwrap_or(first);
            Some((alias, root.to_owned()))
        }
        _ => None,
    }
}

/// The functions and classes at the top of the module in `root`.
fn definitions(root: &Root, import_alias_roots: &HashMap<String, String>) -> Vec<Definition> {
    root.root()
        .children()
        .filter_map(|item| {
            let decorated = item.kind() == "decorated_definition";
            let definition = if decorated {
                item.field("definition")?
            } else {
                item.clone()
            };
            let kind = match definition.kind().as_ref() {
                "function_definition" => DefKind::Function,
                "class_definition" => DefKind::Class,
                _ => return None,
            };
            let name_node = definition.field("name")?;
            let name_id = name_node.node_id();
            let name = name_node.text().into_owned();
            let mut references = HashSet::new();
            let mut import_roots = BTreeSet::new();
            for node in item.dfs() {
                if node.kind() == "identifier" && node.node_id() != name_id {
                    let text = node.text();
                    if kind == DefKind::Function
                        && let Some(root_pkg) = import_alias_roots.get(text.as_ref())
                    {
                        import_roots.insert(root_pkg.clone());
                    }
                    references.insert(text.into_owned());
                }
            }
            Some(Definition {
                kind,
                name,
                line: line(&definition),
                decorated,
                parameters: if kind == DefKind::Class {
                    None
                } else {
                    parameters(&definition)
                },
                references,
                import_roots,
            })
        })
        .collect()
}

/// Detects module-level mutable containers mutated inside functions without `global`.
fn mutable_container_mutations(module: &Node<'_>, zone: Zone) -> Vec<Local> {
    if zone == Zone::Test || zone.is_generated_or_vendor() {
        return Vec::new();
    }
    let mutables = module_level_mutable_containers(module);
    if mutables.is_empty() {
        return Vec::new();
    }
    let mut mutated: BTreeMap<String, usize> = BTreeMap::new();
    for function in module
        .dfs()
        .filter(|node| node.kind() == "function_definition")
    {
        let mut globals = HashSet::new();
        let mut shadowed = HashSet::new();
        for node in local_nodes(&function) {
            if node.kind() == "global_statement" {
                for child in node.children().filter(|child| child.kind() == "identifier") {
                    globals.insert(child.text().into_owned());
                }
                continue;
            }
            if node.kind() == "identifier"
                && (is_parameter_name(&node) || is_local_rebinding(&node))
            {
                shadowed.insert(node.text().into_owned());
            }
        }
        for name in &globals {
            shadowed.remove(name);
        }
        for node in local_nodes(&function) {
            let hit = match node.kind().as_ref() {
                "assignment" | "augmented_assignment" => node
                    .field("left")
                    .filter(|left| left.kind() == "subscript")
                    .and_then(|sub| subscript_target_name(&sub)),
                "delete_statement" => node
                    .named_children()
                    .find(|child| child.kind() == "subscript")
                    .and_then(|sub| subscript_target_name(&sub)),
                "call" => mutating_method_target(&node),
                _ => None,
            };
            if let Some(name) = hit
                && mutables.contains(&name)
                && !shadowed.contains(&name)
            {
                let at = line(&node);
                mutated
                    .entry(name)
                    .and_modify(|first| *first = (*first).min(at))
                    .or_insert(at);
            }
        }
    }
    mutated
        .into_iter()
        .map(|(name, at)| {
            (
                Kind::SharedState,
                at,
                format!(
                    "This function mutates the module level container {name}, which its callers do not pass."
                ),
                1,
            )
        })
        .collect()
}

fn is_local_rebinding(identifier: &Node<'_>) -> bool {
    if !is_bound(identifier) {
        return false;
    }
    !identifier.parent().is_some_and(|parent| {
        matches!(
            parent.kind().as_ref(),
            "global_statement" | "nonlocal_statement" | "delete_statement"
        )
    })
}

fn module_level_mutable_containers(module: &Node<'_>) -> HashSet<String> {
    let mut containers = HashSet::new();
    for statement in module.children() {
        if statement.kind() != "expression_statement" {
            continue;
        }
        let Some(assignment) = statement
            .named_children()
            .next()
            .filter(|node| node.kind() == "assignment")
        else {
            continue;
        };
        let Some(left) = assignment
            .field("left")
            .filter(|left| left.kind() == "identifier")
        else {
            continue;
        };
        let Some(right) = assignment.field("right") else {
            continue;
        };
        if is_mutable_container(&right) {
            containers.insert(left.text().into_owned());
        }
    }
    containers
}

fn is_mutable_container(value: &Node<'_>) -> bool {
    match value.kind().as_ref() {
        "list" | "dictionary" | "set" => true,
        "call" => value
            .field("function")
            .and_then(|function| match function.kind().as_ref() {
                "identifier" => Some(function.text().into_owned()),
                "attribute" => Some(function.field("attribute")?.text().into_owned()),
                _ => None,
            })
            .is_some_and(|name| MUTABLE_CONSTRUCTORS.contains(&name.as_str())),
        _ => false,
    }
}

fn subscript_target_name(node: &Node<'_>) -> Option<String> {
    let mut current = node.clone();
    while current.kind() == "subscript" {
        current = current.field("value")?;
    }
    (current.kind() == "identifier").then(|| current.text().into_owned())
}

fn mutating_method_target(call: &Node<'_>) -> Option<String> {
    let function = call
        .field("function")
        .filter(|node| node.kind() == "attribute")?;
    let method = function.field("attribute")?.text();
    if !MUTATING_METHODS.contains(&method.as_ref()) {
        return None;
    }
    let object = function.field("object")?;
    (object.kind() == "identifier").then(|| object.text().into_owned())
}

/// The link that `node` makes to state outside its function or to the inside of another
/// module. `modules` holds the names that the file gives to the project modules it imports.
fn link(node: &Node<'_>, modules: &HashSet<&str>, zone: Zone) -> Option<(Kind, String, usize)> {
    let skip_shared_state = zone == Zone::Test || zone.is_generated_or_vendor();
    let of_module = |attribute: &Node<'_>| {
        let object = attribute.field("object")?;
        let name = attribute.field("attribute")?;
        modules
            .contains(object.text().as_ref())
            .then(|| (object.text().into_owned(), name.text().into_owned()))
    };
    match node.kind().as_ref() {
        "global_statement" if !skip_shared_state => {
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
        "assignment" | "augmented_assignment" if !skip_shared_state => {
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

/// The findings that follow from the modules together. `changes` holds the commits that changed
/// each tracked file, newest first, and is empty without a history.
pub(super) fn findings(
    modules: &[Module],
    changes: &HashMap<&str, Vec<usize>>,
    entry_points: &EntryPoints,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let graph = Graph::new(modules);
    let project = Project::new(modules);
    findings.extend(graph.volatile(changes));
    findings.extend(graph.cycles());
    findings.extend(graph.orphaned(entry_points, &project));
    findings.extend(graph.facades());
    findings.extend(graph.single_use(entry_points));
    findings.extend(cohesion(modules));
    findings.extend(branches(modules));
    findings.extend(project.unreferenced(entry_points));
    findings.extend(project.parameters());
    findings
}

/// Large production modules whose top-level functions split into disconnected clusters.
fn cohesion(modules: &[Module]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for module in modules {
        if module.facts.zone != Zone::Production || module.facts.loc < COHESION_MIN_LOC {
            continue;
        }
        let functions: Vec<&Definition> = module
            .facts
            .definitions
            .iter()
            .filter(|def| def.kind == DefKind::Function)
            .collect();
        if functions.len() < COHESION_MIN_FUNCTIONS {
            continue;
        }
        let by_name: HashMap<&str, usize> = functions
            .iter()
            .enumerate()
            .map(|(idx, def)| (def.name.as_str(), idx))
            .collect();
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); functions.len()];
        let mut families: HashSet<&str> = HashSet::new();
        let mut import_sets: BTreeSet<&BTreeSet<String>> = BTreeSet::new();

        for (idx, def) in functions.iter().enumerate() {
            families.insert(name_family(&def.name));
            if !def.import_roots.is_empty() {
                import_sets.insert(&def.import_roots);
            }
            for reference in &def.references {
                if let Some(&other) = by_name.get(reference.as_str())
                    && other != idx
                {
                    adjacency[idx].push(other);
                    adjacency[other].push(idx);
                }
            }
        }

        let mut comp_sizes = component_sizes(&adjacency);
        comp_sizes.sort_unstable_by(|a, b| b.cmp(a));
        let largest = comp_sizes.first().copied().unwrap_or(0);
        let min_component_size = (functions.len() / 10).max(2);
        let significant = comp_sizes
            .iter()
            .filter(|&&size| size >= min_component_size)
            .count();

        let disconnected = comp_sizes.len() >= 3 && largest * 2 <= functions.len();
        let mixed_families = families.len() >= 5 && significant >= 3;
        let import_divergence = import_sets.len() >= 4 && significant >= 3;
        if !(disconnected || mixed_families || import_divergence) {
            continue;
        }

        findings.push(Finding {
            kind: Kind::MixedResponsibilities,
            key: module.key().to_owned(),
            places: vec![Place::file(&module.shown)],
            evidence: format!(
                "{} has {} top-level functions split across {} disconnected groups ({} name families, {} import sets).",
                module.shown,
                functions.len(),
                comp_sizes.len(),
                families.len(),
                import_sets.len(),
            ),
            weight: comp_sizes.len() * 10 + families.len(),
        });
    }
    findings
}

fn name_family(name: &str) -> &str {
    let trimmed = name.trim_start_matches('_');
    trimmed
        .split_once('_')
        .map_or(trimmed, |(prefix, _)| prefix)
}

fn component_sizes(adjacency: &[Vec<usize>]) -> Vec<usize> {
    let mut visited = vec![false; adjacency.len()];
    let mut sizes = Vec::new();
    for start in 0..adjacency.len() {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        visited[start] = true;
        let mut size = 0;
        while let Some(node) = stack.pop() {
            size += 1;
            for &neighbor in &adjacency[node] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        sizes.push(size);
    }
    sizes
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
    /// For each module, all modules it imports (any scope) with the line of the first import.
    imports: Vec<BTreeMap<usize, usize>>,
    /// For each module, top-level eager imports only, used for runtime import cycle detection.
    eager_imports: Vec<BTreeMap<usize, usize>>,
    /// For each module, the modules that import it across any scope.
    importers: Vec<Vec<usize>>,
}

impl<'m> Graph<'m> {
    fn new(modules: &'m [Module]) -> Self {
        let index: HashMap<&Path, usize> = (modules.iter().enumerate())
            .map(|(nth, module)| (module.file.as_path(), nth))
            .collect();
        let mut imports = Vec::with_capacity(modules.len());
        let mut eager_imports = Vec::with_capacity(modules.len());
        let mut importers = vec![Vec::new(); modules.len()];

        for (nth, module) in modules.iter().enumerate() {
            let mut all_edges = BTreeMap::new();
            let mut eager_edges = BTreeMap::new();
            for import in &module.facts.imports {
                if let Some(&imported) = index.get(import.module.as_path())
                    && imported != nth
                {
                    all_edges.entry(imported).or_insert(import.line);
                    if import.scope == ImportScope::TopLevel {
                        eager_edges.entry(imported).or_insert(import.line);
                    }
                }
            }
            for &imported in all_edges.keys() {
                importers[imported].push(nth);
            }
            imports.push(all_edges);
            eager_imports.push(eager_edges);
        }
        Self {
            modules,
            imports,
            eager_imports,
            importers,
        }
    }

    fn place(&self, module: usize, line: usize) -> Place {
        Place::line(&self.modules[module].shown, line)
    }

    fn production_count(&self) -> usize {
        self.modules
            .iter()
            .filter(|module| module.facts.zone == Zone::Production)
            .count()
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
            let module = &self.modules[importer];
            if module.facts.zone == Zone::Test
                || module.facts.zone.is_generated_or_vendor()
                || is_init_py(&module.file)
            {
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

    /// Each group of modules that import one another eagerly at module level, found as the
    /// strongly connected parts of the eager import graph.
    fn cycles(&self) -> Vec<Finding> {
        let mut state = Tarjan {
            imports: &self.eager_imports,
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
            .filter(|part| {
                part.len() > 1
                    && !part.iter().all(|&module| {
                        let zone = self.modules[module].facts.zone;
                        zone == Zone::Test || zone.is_generated_or_vendor()
                    })
            })
            .map(|mut part| {
                part.sort_unstable();
                let places = (part.iter())
                    .map(|&module| {
                        let line = (self.eager_imports[module].iter())
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
                        "These {} modules import one another in a circle at load time, so none of them can change alone.",
                        part.len()
                    ),
                    weight: part.len(),
                }
            })
            .collect()
    }

    /// Production modules with zero surveyed importers that are not entry points or public surfaces.
    fn orphaned(&self, entry_points: &EntryPoints, project: &Project<'_>) -> Vec<Finding> {
        if self.production_count() < 2 {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            if facts.zone != Zone::Production
                || !self.importers[nth].is_empty()
                || is_init_py(&module.file)
                || facts.loc < ORPHANED_MIN_LOC
                || facts.has_dunder_all
                || facts.has_main_guard
                || facts
                    .module_name
                    .as_deref()
                    .is_some_and(|name| entry_points.contains(name))
                || project.is_dynamically_referenced(nth, module)
            {
                continue;
            }
            findings.push(Finding {
                kind: Kind::OrphanedModule,
                key: module.key().to_owned(),
                places: vec![Place::file(&module.shown)],
                evidence: format!(
                    "No surveyed Python file imports {} ({} lines).",
                    module.shown, facts.loc
                ),
                weight: facts.loc,
            });
        }
        findings
    }

    /// Pure re-export forwarding modules that add indirection without owning logic.
    fn facades(&self) -> Vec<Finding> {
        let mut dir_counts: HashMap<&Path, usize> = HashMap::new();
        for module in self.modules {
            if let Some(dir) = module.file.parent() {
                *dir_counts.entry(dir).or_default() += 1;
            }
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            let importers = self.importers[nth].len();
            if facts.zone != Zone::Production
                || !facts.is_facade
                || importers > FACADE_MAX_IMPORTERS
            {
                continue;
            }
            if is_init_py(&module.file)
                && module
                    .file
                    .parent()
                    .and_then(|dir| dir_counts.get(dir))
                    .is_some_and(|&count| count > 1)
            {
                continue;
            }
            findings.push(Finding {
                kind: Kind::FacadeModule,
                key: module.key().to_owned(),
                places: vec![Place::file(&module.shown)],
                evidence: format!(
                    "{} only re-exports names from other modules and has {} {}.",
                    module.shown,
                    importers,
                    if importers == 1 {
                        "importer"
                    } else {
                        "importers"
                    }
                ),
                weight: facts.loc + (FACADE_MAX_IMPORTERS + 1).saturating_sub(importers),
            });
        }
        findings
    }

    /// Production helper modules imported by only one non-test file.
    fn single_use(&self, entry_points: &EntryPoints) -> Vec<Finding> {
        if self.production_count() < 2 {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            if facts.zone != Zone::Production
                || is_init_py(&module.file)
                || facts.is_facade
                || facts.has_dunder_all
                || facts.has_main_guard
                || facts.loc < SINGLE_USE_MIN_LOC
                || facts.loc > SINGLE_USE_MAX_LOC
                || facts
                    .module_name
                    .as_deref()
                    .is_some_and(|name| entry_points.contains(name))
            {
                continue;
            }
            let [sole_idx] = self.importers[nth].as_slice() else {
                continue;
            };
            let sole = &self.modules[*sole_idx];
            if matches!(sole.facts.zone, Zone::Test | Zone::Script)
                || is_init_py(&sole.file)
                || sole.facts.loc + facts.loc > SINGLE_USE_MAX_LOC
                || self.imports[*sole_idx].len() >= 5
            {
                continue;
            }
            findings.push(Finding {
                kind: Kind::SingleUseModule,
                key: module.key().to_owned(),
                places: vec![Place::file(&module.shown), Place::file(&sole.shown)],
                evidence: format!(
                    "Only {} imports {} ({} lines), so it may belong in its sole caller.",
                    sole.shown, module.shown, facts.loc
                ),
                weight: facts.loc,
            });
        }
        findings
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

    /// Whether another surveyed module references `module` dynamically by dotted name or via
    /// `importlib` with the module's stem.
    fn is_dynamically_referenced(&self, idx: usize, module: &Module) -> bool {
        let dotted = module.facts.module_name.as_deref();
        let stem = module.file.file_stem().and_then(|part| part.to_str());
        for (other_idx, other) in self.modules.iter().enumerate() {
            if other_idx == idx {
                continue;
            }
            if let Some(name) = dotted
                && other.facts.quoted_modules.contains(name)
            {
                return true;
            }
            let dynamic_loader = other.facts.mentions.contains_key("import_module")
                || other.facts.mentions.contains_key("__import__")
                || other.facts.mentions.contains_key("importlib");
            if dynamic_loader
                && let Some(stem) = stem
                && other.facts.quoted.contains(stem)
            {
                return true;
            }
        }
        false
    }

    /// Each top-level function or class unreachable from any cross-file reference, decorator,
    /// quoted string, `main`, dunder, entry-point callable, or module top-level code. Catches
    /// self-recursive and mutually recursive dead helpers as well as single-mention definitions.
    fn unreferenced(&self, entry_points: &EntryPoints) -> Vec<Finding> {
        let mut findings = Vec::new();
        for module in self.modules {
            let skipped = module.facts.zone == Zone::Test
                || module.facts.zone.is_generated_or_vendor()
                || is_init_py(&module.file);
            if skipped {
                continue;
            }
            let defs = &module.facts.definitions;
            let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
            for (idx, definition) in defs.iter().enumerate() {
                by_name
                    .entry(definition.name.as_str())
                    .or_default()
                    .push(idx);
            }
            let mut live = vec![false; defs.len()];
            let mut queue = VecDeque::new();
            for (idx, definition) in defs.iter().enumerate() {
                let name = definition.name.as_str();
                let total_mentions = self.mentions.get(name).copied().unwrap_or(0);
                let local_mentions = module.facts.mentions.get(name).copied().unwrap_or(0);
                let mentioned_elsewhere = total_mentions > local_mentions;
                let declared_entry = module
                    .facts
                    .module_name
                    .as_deref()
                    .is_some_and(|dotted| entry_points.is_callable(dotted, name));
                let root = definition.decorated
                    || name == "main"
                    || name.starts_with("__")
                    || declared_entry
                    || self.quoted.contains(name)
                    || module.facts.top_level_mentions.contains(name)
                    || mentioned_elsewhere;
                if root {
                    live[idx] = true;
                    queue.push_back(idx);
                }
            }
            while let Some(idx) = queue.pop_front() {
                for reference in &defs[idx].references {
                    if let Some(targets) = by_name.get(reference.as_str()) {
                        for &target in targets {
                            if !live[target] {
                                live[target] = true;
                                queue.push_back(target);
                            }
                        }
                    }
                }
            }
            for (idx, definition) in defs.iter().enumerate() {
                if !live[idx] {
                    let name = definition.name.as_str();
                    findings.push(Finding {
                        kind: Kind::Unreferenced,
                        key: format!("{}\n{name}", module.key()),
                        places: vec![Place::line(&module.shown, definition.line)],
                        evidence: format!(
                            "No live code path in the surveyed Python files reaches {name}."
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
