//! Per-file Python fact extraction and cross-module untangle coordination.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::imports::{self, Import};
use crate::syntax::{Node, Root};

use super::module_graph::ModuleImportGraph;
use super::shared_state::{
    mixed_responsibility_findings, mutable_container_mutations, one_value_branch_findings,
    shared_state_and_private_link,
};
use super::symbol_reach::ProjectSymbolIndex;
use super::zones::{EntryPoints, Zone, classify_file_zone};
use super::{Finding, UntangleKind};

/// One surveyed Python file of the project with its extracted syntax facts.
#[derive(Debug)]
pub(super) struct SurveyedModule {
    /// The path as the ledger prints it.
    pub(super) shown: String,
    /// The absolute path, which an import resolves to.
    pub(super) file: PathBuf,
    /// The path below the top of the repository, which the history names.
    pub(super) tracked: Option<String>,
    pub(super) facts: ModuleFacts,
}

impl SurveyedModule {
    /// The path that names the module in the id of a finding: the one git tracks, so that the
    /// id is the same at another commit.
    pub(super) fn key(&self) -> &str {
        self.tracked.as_deref().unwrap_or(&self.shown)
    }
}

/// Syntax and structural facts extracted from one Python file in isolation.
#[derive(Debug)]
pub(super) struct ModuleFacts {
    pub(super) zone: Zone,
    pub(super) module_name: Option<String>,
    pub(super) loc: usize,
    pub(super) has_dunder_all: bool,
    pub(super) has_main_guard: bool,
    pub(super) is_facade: bool,
    pub(super) imports: Vec<Import>,
    /// How often each identifier is written in the file.
    pub(super) mentions: HashMap<String, usize>,
    /// How often each identifier is written in an import statement.
    pub(super) import_mentions: HashMap<String, usize>,
    /// Identifiers referenced by module top-level code outside top-level definitions.
    pub(super) top_level_mentions: HashSet<String>,
    /// The strings that hold one identifier, as `__all__` and `getattr` name a definition.
    pub(super) quoted: HashSet<String>,
    /// Dotted module paths written as strings, such as `"pkg.plugins.foo"`.
    pub(super) quoted_modules: HashSet<String>,
    pub(super) definitions: Vec<TopLevelDefinition>,
    /// The name of every `def`, also of a method and of a nested function.
    pub(super) function_names: Vec<String>,
    pub(super) calls: Vec<ParsedCall>,
    pub(super) branches: Vec<StringValueBranch>,
    /// The findings that need no other file, each with its line and its evidence.
    pub(super) local: Vec<LocalUntangleFinding>,
}

impl ModuleFacts {
    /// Intent zone classified for this module's path.
    pub(super) fn zone(&self) -> Zone {
        self.zone
    }

    /// Total line count of this module.
    pub(super) fn loc(&self) -> usize {
        self.loc
    }
}

/// A finding of one file: its kind, its line, its evidence and how strong the evidence is.
pub(super) type LocalUntangleFinding = (UntangleKind, usize, String, usize);

/// Whether a top-level definition is a function or a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TopLevelDefKind {
    Function,
    Class,
}

/// A function or class at the top of a Python module.
#[derive(Debug)]
pub(super) struct TopLevelDefinition {
    pub(super) kind: TopLevelDefKind,
    pub(super) name: String,
    pub(super) line: usize,
    pub(super) decorated: bool,
    /// `None` for a class, and for a function with `*args` or `**kwargs`, whose calls cannot be
    /// matched to its parameters.
    pub(super) parameters: Option<Vec<FunctionParameter>>,
    /// Identifiers referenced inside this definition or its decorators.
    pub(super) references: HashSet<String>,
    /// Top-level import root packages referenced inside a function definition.
    pub(super) import_roots: BTreeSet<String>,
}

/// One formal parameter of a top-level Python function.
#[derive(Debug)]
pub(super) struct FunctionParameter {
    pub(super) name: String,
    pub(super) default: bool,
    /// The parameter can be passed by position.
    pub(super) positional: bool,
}

/// An `if` that runs for one string value.
#[derive(Debug)]
pub(super) struct StringValueBranch {
    pub(super) line: usize,
    /// The comparison, as the source writes it.
    pub(super) condition: String,
    /// The string it compares with, with its quotes.
    pub(super) value: String,
    /// The other side is a plain name, such as a parameter.
    pub(super) named: bool,
}

/// One call site, indexed by the last part of the name it calls.
#[derive(Debug)]
pub(super) struct ParsedCall {
    pub(super) name: String,
    pub(super) line: usize,
    /// The text of each argument that is a literal, and `None` for any other argument.
    pub(super) positional: Vec<Option<String>>,
    pub(super) keywords: Vec<(String, Option<String>)>,
    /// The call unpacks `*args` or `**kwargs`, so its arguments cannot be matched.
    pub(super) splat: bool,
}

/// Whether `name` follows Python's single-underscore private naming convention.
pub(super) fn is_private_python_name(name: &str) -> bool {
    name.starts_with('_') && !name.starts_with("__")
}

/// Whether `path` is a package `__init__.py` module.
pub(super) fn is_init_py(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "__init__.py")
}

/// One-based starting line number of a syntax node.
pub(super) fn node_line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}

/// Extracts syntax, import, call, branch, and local finding facts for one Python module.
pub(super) fn extract_module_facts(
    path: &Path,
    root: &Root,
    text: &str,
    exists: impl Fn(&Path) -> bool,
) -> ModuleFacts {
    let zone = classify_file_zone(path);
    let module_root = root.root();
    let import_alias_roots = top_level_import_roots(&module_root);
    let top_summary = inspect_top_level(&module_root);
    let mut facts = ModuleFacts {
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
        if let Some(name) = import
            .name
            .as_deref()
            .filter(|name| is_private_python_name(name))
        {
            facts.local.push((
                UntangleKind::PrivateReach,
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
            "call" => facts.calls.extend(parse_call(&node)),
            "if_statement" if !skip_branches => facts.branches.extend(one_value_branch(&node)),
            _ => {}
        }
        let found = shared_state_and_private_link(&node, &modules, zone);
        local.extend(
            found.map(|(kind, evidence, weight)| (kind, node_line(&node), evidence, weight)),
        );
    }
    local.extend(mutable_container_mutations(&module_root, zone));
    facts.local.extend(local);
    facts
}

/// Computes all cross-module untangle findings across the surveyed Python modules.
pub(super) fn cross_module_findings(
    modules: &[SurveyedModule],
    changes: &HashMap<&str, Vec<usize>>,
    entry_points: &EntryPoints,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let graph = ModuleImportGraph::new(modules);
    let project = ProjectSymbolIndex::new(modules);
    findings.extend(graph.volatile(changes));
    findings.extend(graph.cycles());
    findings.extend(graph.orphaned(entry_points, &project));
    findings.extend(graph.facades());
    findings.extend(graph.single_use(entry_points));
    findings.extend(mixed_responsibility_findings(modules));
    findings.extend(one_value_branch_findings(modules));
    findings.extend(project.unreferenced(entry_points));
    findings.extend(project.parameters());
    findings
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
fn definitions(
    root: &Root,
    import_alias_roots: &HashMap<String, String>,
) -> Vec<TopLevelDefinition> {
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
                "function_definition" => TopLevelDefKind::Function,
                "class_definition" => TopLevelDefKind::Class,
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
                    if kind == TopLevelDefKind::Function
                        && let Some(root_pkg) = import_alias_roots.get(text.as_ref())
                    {
                        import_roots.insert(root_pkg.clone());
                    }
                    references.insert(text.into_owned());
                }
            }
            Some(TopLevelDefinition {
                kind,
                name,
                line: node_line(&definition),
                decorated,
                parameters: if kind == TopLevelDefKind::Class {
                    None
                } else {
                    parse_parameters(&definition)
                },
                references,
                import_roots,
            })
        })
        .collect()
}

/// The parameters of the function `definition`. `None` when it takes `*args` or `**kwargs`.
fn parse_parameters(definition: &Node<'_>) -> Option<Vec<FunctionParameter>> {
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
        parameters.push(FunctionParameter {
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

fn parse_call(node: &Node<'_>) -> Option<ParsedCall> {
    let function = node.field("function")?;
    let name = match function.kind().as_ref() {
        "identifier" => function.text().into_owned(),
        "attribute" => function.field("attribute")?.text().into_owned(),
        _ => return None,
    };
    let mut call = ParsedCall {
        name,
        line: node_line(node),
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
fn one_value_branch(statement: &Node<'_>) -> Option<StringValueBranch> {
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
        (equality && !main_guard).then(|| StringValueBranch {
            line: node_line(statement),
            condition: node.text().into_owned(),
            value: value.text().into_owned(),
            named: children.iter().any(|child| child.kind() == "identifier"),
        })
    })
}
