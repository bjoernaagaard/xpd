//! Renders a Mermaid or JSON module coupling and health map for `xpd untangle --map`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use serde::Serialize;

use crate::output::Format;

use super::python::{Edge, Module};
use super::zones::Zone;
use super::{Finding, Kind};

/// Maximum modules drawn in full before focusing on modules with findings and their neighbors.
const FULL_MAP_LIMIT: usize = 60;

/// Primary visual role of a module node in the coupling map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Role {
    Cycle,
    Mixed,
    Orphan,
    Facade,
    SingleUse,
    Normal,
}

impl Role {
    fn class_name(self) -> Option<&'static str> {
        match self {
            Self::Cycle => Some("cycle"),
            Self::Mixed => Some("mixed"),
            Self::Orphan => Some("orphan"),
            Self::Facade => Some("facade"),
            Self::SingleUse => Some("single"),
            Self::Normal => None,
        }
    }

    fn badge(self) -> Option<&'static str> {
        match self {
            Self::Cycle => Some("cycle"),
            Self::Mixed => Some("mixed"),
            Self::Orphan => Some("orphan"),
            Self::Facade => Some("facade"),
            Self::SingleUse => Some("single-use"),
            Self::Normal => None,
        }
    }
}

/// Serializable node of the coupling map.
#[derive(Debug, Serialize)]
struct MapNode {
    id: String,
    path: String,
    package: String,
    loc: usize,
    role: Role,
}

/// Serializable directed import edge of the coupling map.
#[derive(Debug, Serialize)]
struct MapEdge {
    from: String,
    to: String,
    eager: bool,
    cycle: bool,
    volatile: bool,
}

/// Serializable hidden co-change link between two unlinked modules.
#[derive(Debug, Serialize)]
struct HiddenLink {
    first: String,
    second: String,
}

/// Full structured coupling map output for `--format json`.
#[derive(Debug, Serialize)]
struct CouplingMap {
    focused: bool,
    total_modules: usize,
    nodes: Vec<MapNode>,
    edges: Vec<MapEdge>,
    hidden_links: Vec<HiddenLink>,
}

/// Prints the coupling map of `modules` in Mermaid (`Format::Text`) or JSON (`Format::Json`).
pub(super) fn print(
    modules: &[Module],
    edges: &[Edge],
    findings: &[Finding],
    format: Format,
) -> anyhow::Result<()> {
    let map = build(modules, edges, findings);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&map)?),
        Format::Text => print!("{}", render_mermaid(&map)),
    }
    Ok(())
}

fn build(modules: &[Module], edges: &[Edge], findings: &[Finding]) -> CouplingMap {
    let roles = node_roles(modules, findings);
    let cycle_pairs = cycle_edge_pairs(modules, findings);
    let volatile_pairs = volatile_edge_pairs(modules, findings);
    let hidden_pairs = hidden_link_pairs(modules, findings);
    let (kept, focused, total_modules) =
        select_nodes(modules, edges, &roles, &volatile_pairs, &hidden_pairs);

    let id_of: HashMap<usize, String> = kept
        .iter()
        .enumerate()
        .map(|(slot, &idx)| (idx, format!("m{slot}")))
        .collect();

    let nodes: Vec<MapNode> = kept
        .iter()
        .map(|&idx| {
            let module = &modules[idx];
            MapNode {
                id: id_of[&idx].clone(),
                path: module.shown.clone(),
                package: package_dir(&module.shown),
                loc: module.facts.loc(),
                role: roles[idx],
            }
        })
        .collect();

    let map_edges: Vec<MapEdge> = edges
        .iter()
        .filter_map(|edge| {
            let from = id_of.get(&edge.from)?.clone();
            let to = id_of.get(&edge.to)?.clone();
            Some(MapEdge {
                from,
                to,
                eager: edge.eager,
                cycle: edge.eager && cycle_pairs.contains(&(edge.from, edge.to)),
                volatile: volatile_pairs.contains(&(edge.from, edge.to)),
            })
        })
        .collect();

    let hidden_links: Vec<HiddenLink> = hidden_pairs
        .into_iter()
        .filter_map(|(a, b)| {
            Some(HiddenLink {
                first: id_of.get(&a)?.clone(),
                second: id_of.get(&b)?.clone(),
            })
        })
        .collect();

    CouplingMap {
        focused,
        total_modules,
        nodes,
        edges: map_edges,
        hidden_links,
    }
}

fn select_nodes(
    modules: &[Module],
    edges: &[Edge],
    roles: &[Role],
    volatile_pairs: &HashSet<(usize, usize)>,
    hidden_pairs: &BTreeSet<(usize, usize)>,
) -> (Vec<usize>, bool, usize) {
    let non_test: Vec<usize> = modules
        .iter()
        .enumerate()
        .filter(|(_, module)| {
            let zone = module.facts.zone();
            zone != Zone::Test && !zone.is_generated_or_vendor()
        })
        .map(|(idx, _)| idx)
        .collect();
    let candidates = if non_test.is_empty() {
        (0..modules.len()).collect()
    } else {
        non_test
    };
    let total = candidates.len();
    if total <= FULL_MAP_LIMIT {
        return (candidates, false, total);
    }
    let candidate_set: HashSet<usize> = candidates.iter().copied().collect();
    let mut active: BTreeSet<usize> = candidates
        .iter()
        .copied()
        .filter(|&idx| roles[idx] != Role::Normal)
        .collect();
    for &(a, b) in hidden_pairs.iter().chain(volatile_pairs.iter()) {
        if candidate_set.contains(&a) && candidate_set.contains(&b) {
            active.insert(a);
            active.insert(b);
        }
    }
    let seeds = active.clone();
    for edge in edges {
        if candidate_set.contains(&edge.from)
            && candidate_set.contains(&edge.to)
            && (seeds.contains(&edge.from) || seeds.contains(&edge.to))
        {
            active.insert(edge.from);
            active.insert(edge.to);
        }
    }
    let kept = if active.is_empty() {
        candidates.into_iter().take(FULL_MAP_LIMIT).collect()
    } else {
        active.into_iter().collect()
    };
    (kept, true, total)
}

fn package_dir(shown: &str) -> String {
    Path::new(shown)
        .parent()
        .and_then(Path::to_str)
        .filter(|dir| !dir.is_empty() && *dir != ".")
        .unwrap_or("")
        .to_owned()
}

fn short_name(shown: &str) -> String {
    Path::new(shown)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shown)
        .to_owned()
}

fn node_roles(modules: &[Module], findings: &[Finding]) -> Vec<Role> {
    let by_shown: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.shown.as_str(), idx))
        .collect();
    let mut roles = vec![Role::Normal; modules.len()];
    let mut set_role = |path: &str, role: Role| {
        if let Some(&idx) = by_shown.get(path)
            && role < roles[idx]
        {
            roles[idx] = role;
        }
    };
    for finding in findings {
        match finding.kind {
            Kind::ImportCycle => {
                for place in &finding.places {
                    set_role(&place.path, Role::Cycle);
                }
            }
            Kind::MixedResponsibilities => {
                for place in &finding.places {
                    set_role(&place.path, Role::Mixed);
                }
            }
            Kind::OrphanedModule => {
                for place in &finding.places {
                    set_role(&place.path, Role::Orphan);
                }
            }
            Kind::FacadeModule => {
                for place in &finding.places {
                    set_role(&place.path, Role::Facade);
                }
            }
            Kind::SingleUseModule => {
                if let Some(first) = finding.places.first() {
                    set_role(&first.path, Role::SingleUse);
                }
            }
            _ => {}
        }
    }
    roles
}

fn cycle_edge_pairs(modules: &[Module], findings: &[Finding]) -> HashSet<(usize, usize)> {
    let by_shown: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.shown.as_str(), idx))
        .collect();
    let mut pairs = HashSet::new();
    for finding in findings {
        if finding.kind != Kind::ImportCycle {
            continue;
        }
        let members: Vec<usize> = finding
            .places
            .iter()
            .filter_map(|place| by_shown.get(place.path.as_str()).copied())
            .collect();
        for &a in &members {
            for &b in &members {
                if a != b {
                    pairs.insert((a, b));
                }
            }
        }
    }
    pairs
}

fn volatile_edge_pairs(modules: &[Module], findings: &[Finding]) -> HashSet<(usize, usize)> {
    let by_key: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.key(), idx))
        .collect();
    let mut pairs = HashSet::new();
    for finding in findings {
        if finding.kind != Kind::VolatileImport {
            continue;
        }
        if let Some((importer, imported)) = finding.key.split_once('\n')
            && let (Some(&from), Some(&to)) = (by_key.get(importer), by_key.get(imported))
        {
            pairs.insert((from, to));
        }
    }
    pairs
}

fn hidden_link_pairs(modules: &[Module], findings: &[Finding]) -> BTreeSet<(usize, usize)> {
    let by_key: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(idx, module)| (module.key(), idx))
        .collect();
    let mut pairs = BTreeSet::new();
    for finding in findings {
        if finding.kind != Kind::ChangesTogether {
            continue;
        }
        if let Some((first, second)) = finding.key.split_once('\n')
            && let (Some(&a), Some(&b)) = (by_key.get(first), by_key.get(second))
        {
            let pair = if a < b { (a, b) } else { (b, a) };
            pairs.insert(pair);
        }
    }
    pairs
}

fn render_mermaid(map: &CouplingMap) -> String {
    let mut out = String::new();
    out.push_str("flowchart LR\n");
    if map.focused {
        let _ = writeln!(
            out,
            "  %% Focused view: {} of {} modules with findings and direct neighbors",
            map.nodes.len(),
            map.total_modules
        );
    }
    let mut by_pkg: BTreeMap<&str, Vec<&MapNode>> = BTreeMap::new();
    for node in &map.nodes {
        by_pkg.entry(node.package.as_str()).or_default().push(node);
    }
    for (group_idx, (pkg, group)) in by_pkg.iter().enumerate() {
        let use_subgraph = !pkg.is_empty() && group.len() > 1;
        if use_subgraph {
            let _ = writeln!(out, "  subgraph g{group_idx}[\"{pkg}\"]");
        }
        let indent = if use_subgraph { "    " } else { "  " };
        for node in group {
            let name = if use_subgraph {
                short_name(&node.path)
            } else {
                node.path.clone()
            };
            let label = match node.role.badge() {
                Some(badge) => format!("{name} ({} LOC, {badge})", node.loc),
                None => format!("{name} ({} LOC)", node.loc),
            };
            let class = node
                .role
                .class_name()
                .map_or_else(String::new, |cls| format!(":::{cls}"));
            let _ = writeln!(out, "{indent}{}[\"{label}\"]{class}", node.id);
        }
        if use_subgraph {
            out.push_str("  end\n");
        }
    }

    let mut link_index = 0usize;
    let mut link_styles: Vec<(usize, &'static str)> = Vec::new();
    for edge in &map.edges {
        if edge.cycle {
            let _ = writeln!(out, "  {} ==>|\"cycle\"| {}", edge.from, edge.to);
            link_styles.push((link_index, "stroke:#dc2626,stroke-width:2px"));
        } else if edge.volatile {
            let _ = writeln!(out, "  {} -->|\"volatile\"| {}", edge.from, edge.to);
            link_styles.push((link_index, "stroke:#d97706,stroke-width:2px"));
        } else if edge.eager {
            let _ = writeln!(out, "  {} --> {}", edge.from, edge.to);
        } else {
            let _ = writeln!(out, "  {} -.-> {}", edge.from, edge.to);
        }
        link_index += 1;
    }
    for link in &map.hidden_links {
        let _ = writeln!(
            out,
            "  {} <-. \"changes together\" .-> {}",
            link.first, link.second
        );
        link_styles.push((link_index, "stroke:#9333ea,stroke-dasharray: 5 3"));
        link_index += 1;
    }

    let used_roles: BTreeSet<Role> = map.nodes.iter().map(|node| node.role).collect();
    if used_roles.contains(&Role::Cycle) {
        out.push_str(
            "  classDef cycle fill:#fee2e2,stroke:#dc2626,stroke-width:2px,color:#7f1d1d\n",
        );
    }
    if used_roles.contains(&Role::Mixed) {
        out.push_str(
            "  classDef mixed fill:#ffedd5,stroke:#ea580c,stroke-width:2px,color:#7c2d12\n",
        );
    }
    if used_roles.contains(&Role::Orphan) {
        out.push_str(
            "  classDef orphan fill:#f3f4f6,stroke:#6b7280,stroke-dasharray: 5 3,color:#374151\n",
        );
    }
    if used_roles.contains(&Role::Facade) {
        out.push_str(
            "  classDef facade fill:#fef3c7,stroke:#d97706,stroke-dasharray: 4 2,color:#78350f\n",
        );
    }
    if used_roles.contains(&Role::SingleUse) {
        out.push_str("  classDef single fill:#e0f2fe,stroke:#0284c7,color:#0c4a6e\n");
    }
    for (idx, style) in link_styles {
        let _ = writeln!(out, "  linkStyle {idx} {style}");
    }
    out
}
