//! Rust `mod` and `use` dependency graph extraction for `xpd untangle --map`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use ast_grep_language::SupportLang;

use crate::ledger::Tree;
use crate::ledger::gix_source::Listing;
use crate::syntax::{self, Node};

/// One surveyed Rust module node.
#[derive(Debug)]
pub(super) struct RustModule {
    pub(super) shown: String,
    pub(super) module_path: String,
    pub(super) loc: usize,
    pub(super) is_root: bool,
    pub(super) declared_mods: Vec<String>,
    pub(super) used_paths: Vec<String>,
}

/// Directed edge between two surveyed Rust modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RustEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) cycle: bool,
}

/// Extracted Rust module graph for `xpd untangle --map`.
#[derive(Debug)]
pub(super) struct RustGraph {
    pub(super) modules: Vec<RustModule>,
    pub(super) edges: Vec<RustEdge>,
    pub(super) cycle_nodes: BTreeSet<usize>,
    pub(super) orphan_nodes: BTreeSet<usize>,
}

/// Extracts a Rust `mod` and `use` graph from all `.rs` files in `listing`.
pub(super) fn build_rust_graph(tree: Tree<'_>, listing: &Listing) -> RustGraph {
    let rs_files: Vec<&PathBuf> = listing
        .files
        .iter()
        .filter(|(_, lang)| *lang == SupportLang::Rust)
        .map(|(path, _)| path)
        .collect();
    if rs_files.is_empty() {
        return RustGraph {
            modules: Vec::new(),
            edges: Vec::new(),
            cycle_nodes: BTreeSet::new(),
            orphan_nodes: BTreeSet::new(),
        };
    }

    let mut modules = Vec::with_capacity(rs_files.len());
    for path in &rs_files {
        let Some(text) = listing.text(path) else {
            continue;
        };
        let shown = tree.shown(path);
        let (module_path, is_root) = rust_module_path(&shown);
        let parsed = syntax::parse_as(&text, SupportLang::Rust);
        let mut declared_mods = Vec::new();
        let mut used_paths = Vec::new();
        collect_items(&parsed.root(), &mut declared_mods, &mut used_paths);
        modules.push(RustModule {
            shown,
            module_path,
            loc: text.lines().count(),
            is_root,
            declared_mods,
            used_paths,
        });
    }

    let by_module: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.module_path.as_str(), idx))
        .collect();

    let mut raw_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut incoming: Vec<usize> = vec![0; modules.len()];

    for (from_idx, module) in modules.iter().enumerate() {
        for child in &module.declared_mods {
            let target = if module.module_path == "crate" {
                format!("crate::{child}")
            } else {
                format!("{}::{child}", module.module_path)
            };
            if let Some(&to_idx) = by_module.get(target.as_str())
                && to_idx != from_idx
                && raw_edges.insert((from_idx, to_idx))
            {
                incoming[to_idx] += 1;
            }
        }
        for used in &module.used_paths {
            if let Some(to_idx) = resolve_use_target(&module.module_path, used, &by_module)
                && to_idx != from_idx
                && raw_edges.insert((from_idx, to_idx))
            {
                incoming[to_idx] += 1;
            }
        }
    }

    let cycle_nodes = tarjan_cycle_nodes(modules.len(), &raw_edges);
    let edges: Vec<RustEdge> = raw_edges
        .into_iter()
        .map(|(from, to)| RustEdge {
            from,
            to,
            cycle: cycle_nodes.contains(&from) && cycle_nodes.contains(&to),
        })
        .collect();
    let orphan_nodes: BTreeSet<usize> = modules
        .iter()
        .enumerate()
        .filter(|(idx, module)| !module.is_root && incoming[*idx] == 0)
        .map(|(idx, _)| idx)
        .collect();

    RustGraph {
        modules,
        edges,
        cycle_nodes,
        orphan_nodes,
    }
}

fn rust_module_path(shown: &str) -> (String, bool) {
    let normalized = shown.replace('\\', "/");
    let stripped = normalized
        .strip_prefix("src/")
        .or_else(|| normalized.split_once("/src/").map(|(_, rest)| rest))
        .unwrap_or(&normalized);
    if matches!(stripped, "lib.rs" | "main.rs" | "build.rs") {
        let name = Path::new(stripped)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("crate");
        let path = if name == "lib" || name == "main" {
            "crate".to_owned()
        } else {
            format!("crate::{name}")
        };
        return (path, true);
    }
    if normalized.starts_with("examples/") || normalized.starts_with("tests/") {
        let stem = Path::new(&normalized)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&normalized);
        return (format!("example::{stem}"), true);
    }
    let without_ext = stripped
        .strip_suffix("/mod.rs")
        .or_else(|| stripped.strip_suffix(".rs"))
        .unwrap_or(stripped);
    let segments: Vec<&str> = without_ext
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        ("crate".to_owned(), true)
    } else {
        (format!("crate::{}", segments.join("::")), false)
    }
}

fn collect_items(root: &Node<'_>, mods: &mut Vec<String>, uses: &mut Vec<String>) {
    for node in root.dfs() {
        match node.kind().as_ref() {
            "mod_item" => {
                if node.field("body").is_none()
                    && let Some(name) = node.field("name")
                {
                    mods.push(name.text().into_owned());
                }
            }
            "use_declaration" => {
                if let Some(arg) = node.field("argument") {
                    expand_use_tree(&arg, "", uses);
                }
            }
            _ => {}
        }
    }
}

fn expand_use_tree(node: &Node<'_>, prefix: &str, out: &mut Vec<String>) {
    match node.kind().as_ref() {
        "scoped_identifier" | "identifier" | "crate" | "super" | "self" => {
            let text = node.text().replace(' ', "");
            let joined = join_rust_path(prefix, &text);
            if !joined.is_empty() {
                out.push(joined);
            }
        }
        "use_as_clause" => {
            if let Some(path) = node.field("path") {
                expand_use_tree(&path, prefix, out);
            }
        }
        "use_wildcard" => {
            let text = node.text().replace("::*", "").replace(' ', "");
            let joined = join_rust_path(prefix, &text);
            if !joined.is_empty() {
                out.push(joined);
            }
        }
        "scoped_use_list" => {
            let next_prefix = match node.field("path") {
                Some(path) => join_rust_path(prefix, &path.text().replace(' ', "")),
                None => prefix.to_owned(),
            };
            if let Some(list) = node.field("list") {
                for child in list.children() {
                    if child.is_named() {
                        expand_use_tree(&child, &next_prefix, out);
                    }
                }
            }
        }
        "use_list" => {
            for child in node.children() {
                if child.is_named() {
                    expand_use_tree(&child, prefix, out);
                }
            }
        }
        _ => {}
    }
}

fn join_rust_path(prefix: &str, suffix: &str) -> String {
    let trimmed = suffix.trim_matches(':');
    if trimmed == "self" {
        return prefix.to_owned();
    }
    if prefix.is_empty() {
        trimmed.to_owned()
    } else if trimmed.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}::{trimmed}")
    }
}

fn resolve_use_target(
    current_module: &str,
    raw_use: &str,
    by_module: &HashMap<&str, usize>,
) -> Option<usize> {
    let normalized = if raw_use.starts_with("crate::") || raw_use == "crate" {
        raw_use.to_owned()
    } else if let Some(rest) = raw_use.strip_prefix("self::") {
        format!("{current_module}::{rest}")
    } else if raw_use.starts_with("super::") || raw_use == "super" {
        let mut parts: Vec<&str> = current_module.split("::").collect();
        let mut remaining = raw_use;
        while let Some(after) = remaining.strip_prefix("super::") {
            if parts.len() > 1 {
                parts.pop();
            }
            remaining = after;
        }
        if remaining == "super" {
            if parts.len() > 1 {
                parts.pop();
            }
            parts.join("::")
        } else {
            format!("{}::{remaining}", parts.join("::"))
        }
    } else {
        return None;
    };

    let mut candidate = normalized.as_str();
    loop {
        if let Some(&idx) = by_module.get(candidate) {
            return Some(idx);
        }
        let (parent, _) = candidate.rsplit_once("::")?;
        if parent == "crate" {
            return None;
        }
        candidate = parent;
    }
}

fn tarjan_cycle_nodes(node_count: usize, edges: &BTreeSet<(usize, usize)>) -> BTreeSet<usize> {
    let mut adj: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(from, to) in edges {
        adj.entry(from).or_default().push(to);
    }
    let mut state = TarjanState {
        index: 0,
        indices: vec![None; node_count],
        lowlink: vec![0; node_count],
        on_stack: vec![false; node_count],
        stack: Vec::new(),
        cycles: BTreeSet::new(),
    };
    for node in 0..node_count {
        if state.indices[node].is_none() {
            tarjan_visit(node, &adj, &mut state);
        }
    }
    state.cycles
}

struct TarjanState {
    index: usize,
    indices: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    cycles: BTreeSet<usize>,
}

fn tarjan_visit(node: usize, adj: &BTreeMap<usize, Vec<usize>>, state: &mut TarjanState) {
    state.indices[node] = Some(state.index);
    state.lowlink[node] = state.index;
    state.index += 1;
    state.stack.push(node);
    state.on_stack[node] = true;

    if let Some(neighbors) = adj.get(&node) {
        for &next in neighbors {
            if state.indices[next].is_none() {
                tarjan_visit(next, adj, state);
                state.lowlink[node] = state.lowlink[node].min(state.lowlink[next]);
            } else if state.on_stack[next] {
                state.lowlink[node] =
                    state.lowlink[node].min(state.indices[next].expect("visited node has index"));
            }
        }
    }

    if state.lowlink[node] == state.indices[node].expect("node has index") {
        let mut component = Vec::new();
        while let Some(member) = state.stack.pop() {
            state.on_stack[member] = false;
            component.push(member);
            if member == node {
                break;
            }
        }
        if component.len() > 1 {
            for member in component {
                state.cycles.insert(member);
            }
        }
    }
}
