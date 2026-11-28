//! Shared mutable state, module cohesion clustering, and single-value branch detectors.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ledger::FindingPlace;
use crate::ledger::python_ast::local_nodes;
use crate::names::{is_bound, is_parameter_name};
use crate::syntax::Node;

use super::python_facts::{
    LocalUntangleFinding, StringValueBranch, SurveyedModule, TopLevelDefKind, TopLevelDefinition,
    is_private_python_name, node_line,
};
use super::zones::Zone;
use super::{Finding, UntangleKind};

/// How much of a condition is quoted in the evidence.
const QUOTED_CHARS: usize = 60;
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

/// `text` on one line, cut to the length the evidence quotes.
fn quoted(text: &str) -> String {
    crate::output::quote_one_line(text, QUOTED_CHARS)
}

/// Detects module-level mutable containers mutated inside functions without `global`.
pub(super) fn mutable_container_mutations(
    module: &Node<'_>,
    zone: Zone,
) -> Vec<LocalUntangleFinding> {
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
                let at = node_line(&node);
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
                UntangleKind::SharedState,
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
pub(super) fn shared_state_and_private_link(
    node: &Node<'_>,
    modules: &HashSet<&str>,
    zone: Zone,
) -> Option<(UntangleKind, String, usize)> {
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
                UntangleKind::SharedState,
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
                UntangleKind::SharedState,
                format!("This writes {name} of the module {module} from outside it."),
                2,
            ))
        }
        "attribute" => {
            let (module, name) =
                of_module(node).filter(|(_, name)| is_private_python_name(name))?;
            Some((
                UntangleKind::PrivateReach,
                format!("This reads the private name {name} of the module {module}."),
                1,
            ))
        }
        _ => None,
    }
}

/// Large production modules whose top-level functions split into disconnected clusters.
pub(super) fn mixed_responsibility_findings(modules: &[SurveyedModule]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for module in modules {
        if module.facts.zone != Zone::Production || module.facts.loc < COHESION_MIN_LOC {
            continue;
        }
        let functions: Vec<&TopLevelDefinition> = module
            .facts
            .definitions
            .iter()
            .filter(|def| def.kind == TopLevelDefKind::Function)
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
            kind: UntangleKind::MixedResponsibilities,
            key: module.key().to_owned(),
            places: vec![FindingPlace::file(&module.shown)],
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

/// The branches for one value, one finding for each value with every place that checks it.
pub(super) fn one_value_branch_findings(modules: &[SurveyedModule]) -> Vec<Finding> {
    let mut by_value: BTreeMap<String, Vec<(&SurveyedModule, &StringValueBranch)>> =
        BTreeMap::new();
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
                kind: UntangleKind::OneValueBranch,
                key: value,
                places: (found.iter())
                    .map(|(module, branch)| FindingPlace::line(&module.shown, branch.line))
                    .collect(),
                evidence,
                weight: found.len() * 2 + usize::from(found.iter().any(|(_, branch)| branch.named)),
            }
        })
        .collect()
}
