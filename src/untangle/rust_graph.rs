//! Rust `mod` and `use` dependency graph extraction for `xpd untangle` and `--map`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use ast_grep_language::SupportLang;

use crate::ledger::gix_source::Listing;
use crate::ledger::{FindingPlace, Tree};
use crate::syntax::{self, Node};

use super::{Finding, UntangleKind};

/// One surveyed Rust module node.
#[derive(Debug)]
pub(super) struct RustModule {
    pub(super) shown: String,
    pub(super) file: PathBuf,
    pub(super) module_path: String,
    pub(super) loc: usize,
    pub(super) is_root: bool,
    pub(super) declared_mods: Vec<DeclaredMod>,
    pub(super) used_paths: Vec<String>,
}

/// One external `mod name;` declaration, optionally carrying a `#[path = "..."]` override.
#[derive(Debug)]
pub(super) struct DeclaredMod {
    pub(super) scope_path: String,
    pub(super) name: String,
    pub(super) explicit_path: Option<String>,
}

/// Directed edge between two surveyed Rust modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RustEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) cycle: bool,
}

/// Extracted Rust module graph for `xpd untangle` surveys and `--map`.
#[derive(Debug)]
pub(super) struct RustGraph {
    pub(super) modules: Vec<RustModule>,
    pub(super) edges: Vec<RustEdge>,
    pub(super) cycle_sccs: Vec<Vec<usize>>,
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
            cycle_sccs: Vec::new(),
            cycle_nodes: BTreeSet::new(),
            orphan_nodes: BTreeSet::new(),
        };
    }

    let modules = parse_rust_modules(tree, listing, &rs_files);
    let by_module: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.module_path.as_str(), idx))
        .collect();
    let by_file: HashMap<PathBuf, usize> = modules
        .iter()
        .enumerate()
        .filter_map(|(idx, module)| Some((canonical_key(&module.file)?, idx)))
        .collect();

    let (raw_edges, incoming) = resolve_graph_edges(&modules, &by_module, &by_file);
    let externally_declared = collect_external_parent_declarations(&modules, &by_module, &by_file);

    let cycle_sccs = tarjan_cycle_sccs(modules.len(), &raw_edges);
    let cycle_nodes: BTreeSet<usize> = cycle_sccs
        .iter()
        .flat_map(|scc| scc.iter().copied())
        .collect();
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
        .filter(|(idx, module)| {
            !module.is_root && incoming[*idx] == 0 && !externally_declared.contains(idx)
        })
        .map(|(idx, _)| idx)
        .collect();

    RustGraph {
        modules,
        edges,
        cycle_sccs,
        cycle_nodes,
        orphan_nodes,
    }
}

/// Converts Rust cycles and orphaned modules in `graph` into `xpd untangle` ledger findings.
pub(super) fn rust_findings(graph: &RustGraph) -> Vec<Finding> {
    let mut findings = Vec::new();
    for scc in &graph.cycle_sccs {
        let mut places: Vec<FindingPlace> = scc
            .iter()
            .map(|&idx| FindingPlace::file(&graph.modules[idx].shown))
            .collect();
        places.sort();
        let names: Vec<&str> = places.iter().map(|place| place.path.as_str()).collect();
        findings.push(Finding {
            kind: UntangleKind::ImportCycle,
            key: format!("rust-cycle\n{}", names.join("\n")),
            evidence: format!(
                "Rust modules form a dependency cycle: {}.",
                names.join(", ")
            ),
            weight: places.len() * 100,
            places,
        });
    }
    for &idx in &graph.orphan_nodes {
        let module = &graph.modules[idx];
        findings.push(Finding {
            kind: UntangleKind::OrphanedModule,
            key: format!("rust-orphan\n{}", module.shown),
            places: vec![FindingPlace::file(&module.shown)],
            evidence: format!(
                "No surveyed Rust module or parent `mod` file declares or imports `{}` ({} lines).",
                module.module_path, module.loc
            ),
            weight: module.loc.max(1),
        });
    }
    findings
}

fn parse_rust_modules(tree: Tree<'_>, listing: &Listing, rs_files: &[&PathBuf]) -> Vec<RustModule> {
    let mut modules = Vec::with_capacity(rs_files.len());
    for path in rs_files {
        let Some(text) = listing.text(path) else {
            continue;
        };
        let shown = tree.shown(path);
        let (module_path, is_root) = rust_module_path(&shown);
        let parsed = syntax::parse_as(&text, SupportLang::Rust);
        let mut declared_mods = Vec::new();
        let mut used_paths = Vec::new();
        collect_items(
            &parsed.root(),
            &module_path,
            &mut declared_mods,
            &mut used_paths,
        );
        let file = listing.absolute(path).unwrap_or_else(|| (*path).clone());
        modules.push(RustModule {
            shown,
            file,
            module_path,
            loc: text.lines().count(),
            is_root,
            declared_mods,
            used_paths,
        });
    }
    modules
}

fn resolve_graph_edges(
    modules: &[RustModule],
    by_module: &HashMap<&str, usize>,
    by_file: &HashMap<PathBuf, usize>,
) -> (BTreeSet<(usize, usize)>, Vec<usize>) {
    let mut raw_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut incoming: Vec<usize> = vec![0; modules.len()];

    for (from_idx, module) in modules.iter().enumerate() {
        for declared in &module.declared_mods {
            let target_idx =
                resolve_declared_mod_target(&module.file, declared, by_module, by_file);
            if let Some(to_idx) = target_idx
                && to_idx != from_idx
                && raw_edges.insert((from_idx, to_idx))
            {
                incoming[to_idx] += 1;
            }
        }
        for used in &module.used_paths {
            if let Some(to_idx) = resolve_use_target(&module.module_path, used, by_module)
                && to_idx != from_idx
                && raw_edges.insert((from_idx, to_idx))
            {
                incoming[to_idx] += 1;
            }
        }
    }
    (raw_edges, incoming)
}

fn resolve_declared_mod_target(
    owner_file: &Path,
    declared: &DeclaredMod,
    by_module: &HashMap<&str, usize>,
    by_file: &HashMap<PathBuf, usize>,
) -> Option<usize> {
    if let Some(explicit) = &declared.explicit_path {
        let base_dir = owner_file.parent().unwrap_or_else(|| Path::new("."));
        let candidate = base_dir.join(explicit);
        if let Some(key) = canonical_key(&candidate)
            && let Some(&idx) = by_file.get(&key)
        {
            return Some(idx);
        }
    }
    let target = if declared.scope_path == "crate" {
        format!("crate::{}", declared.name)
    } else {
        format!("{}::{}", declared.scope_path, declared.name)
    };
    by_module.get(target.as_str()).copied()
}

/// Checks immediate parent `mod` files outside the surveyed path (such as `src/untangle.rs` when
/// surveying `src/untangle/`) so submodules declared by their parent are not falsely marked orphan.
fn collect_external_parent_declarations(
    modules: &[RustModule],
    by_module: &HashMap<&str, usize>,
    by_file: &HashMap<PathBuf, usize>,
) -> BTreeSet<usize> {
    let mut external_files: BTreeMap<PathBuf, String> = BTreeMap::new();
    for module in modules {
        if module.is_root {
            continue;
        }
        let Some((parent_mod, _)) = module.module_path.rsplit_once("::") else {
            continue;
        };
        if by_module.contains_key(parent_mod) {
            continue;
        }
        for candidate in candidate_parent_files(&module.file) {
            if candidate.is_file()
                && let Some(key) = canonical_key(&candidate)
                && !by_file.contains_key(&key)
            {
                external_files
                    .entry(key)
                    .or_insert_with(|| parent_mod.to_owned());
            }
        }
    }

    let mut declared_indices = BTreeSet::new();
    for (parent_file, parent_mod) in external_files {
        let Ok(text) = fs::read_to_string(&parent_file) else {
            continue;
        };
        let parsed = syntax::parse_as(&text, SupportLang::Rust);
        let mut mods = Vec::new();
        let mut uses = Vec::new();
        collect_items(&parsed.root(), &parent_mod, &mut mods, &mut uses);
        for declared in &mods {
            if let Some(idx) =
                resolve_declared_mod_target(&parent_file, declared, by_module, by_file)
            {
                declared_indices.insert(idx);
            }
        }
        for used in &uses {
            if let Some(idx) = resolve_use_target(&parent_mod, used, by_module) {
                declared_indices.insert(idx);
            }
        }
    }
    declared_indices
}

fn candidate_parent_files(child_file: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(dir) = child_file.parent() else {
        return candidates;
    };
    candidates.push(dir.join("mod.rs"));
    candidates.push(dir.join("lib.rs"));
    candidates.push(dir.join("main.rs"));
    if let Some(dir_name) = dir.file_name().and_then(|s| s.to_str())
        && let Some(grandparent) = dir.parent()
    {
        candidates.push(grandparent.join(format!("{dir_name}.rs")));
        candidates.push(grandparent.join("lib.rs"));
        candidates.push(grandparent.join("main.rs"));
    }
    candidates
}

fn canonical_key(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .ok()
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
    if let Some(bin_rest) = stripped.strip_prefix("bin/") {
        let stem = Path::new(bin_rest)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        return (format!("bin::{stem}"), true);
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

fn collect_items(
    node: &Node<'_>,
    scope_path: &str,
    mods: &mut Vec<DeclaredMod>,
    uses: &mut Vec<String>,
) {
    let children: Vec<_> = node.children().collect();
    let mut pending_path: Option<String> = None;
    for child in children {
        match child.kind().as_ref() {
            "attribute_item" => {
                if let Some(path) = extract_path_attribute(&child) {
                    pending_path = Some(path);
                }
            }
            "mod_item" => {
                let explicit_path = pending_path.take();
                let Some(name_node) = child.field("name") else {
                    continue;
                };
                let name = name_node.text().into_owned();
                if let Some(body) = child.field("body") {
                    let nested_scope = if scope_path == "crate" {
                        format!("crate::{name}")
                    } else {
                        format!("{scope_path}::{name}")
                    };
                    collect_items(&body, &nested_scope, mods, uses);
                } else {
                    mods.push(DeclaredMod {
                        scope_path: scope_path.to_owned(),
                        name,
                        explicit_path,
                    });
                }
            }
            "use_declaration" => {
                pending_path = None;
                if let Some(arg) = child.field("argument") {
                    expand_use_tree(&arg, "", uses);
                }
            }
            _ => {
                if child.is_named() {
                    pending_path = None;
                    collect_items(&child, scope_path, mods, uses);
                }
            }
        }
    }
}

fn extract_path_attribute(attr: &Node<'_>) -> Option<String> {
    let text = attr.text();
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("#[")?.strip_suffix(']')?.trim();
    let rest = inner.strip_prefix("path")?.trim();
    let value = rest.strip_prefix('=')?.trim();
    let unquoted = value.strip_prefix('"')?.strip_suffix('"')?;
    (!unquoted.is_empty()).then(|| unquoted.to_owned())
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

fn tarjan_cycle_sccs(node_count: usize, edges: &BTreeSet<(usize, usize)>) -> Vec<Vec<usize>> {
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
        sccs: Vec::new(),
    };
    for node in 0..node_count {
        if state.indices[node].is_none() {
            tarjan_visit(node, &adj, &mut state);
        }
    }
    state.sccs
}

struct TarjanState {
    index: usize,
    indices: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    sccs: Vec<Vec<usize>>,
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
            component.sort_unstable();
            state.sccs.push(component);
        }
    }
}
