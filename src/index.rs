//! Source-only library discovery and keyword search.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};

use crate::{catalog, pack, scan};

type Syntax<'a> = Node<'a, StrDoc<SupportLang>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Symbol,
    Rule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) parameters: Vec<String>,
    pub(crate) summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) kind: Kind,
    pub(crate) package: String,
    pub(crate) name: String,
    pub(crate) callable: Option<String>,
    pub(crate) parameters: Vec<String>,
    pub(crate) summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) keywords: Vec<String>,
    pub(crate) members: Vec<Member>,
    pub(crate) rules: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Index {
    pub(crate) schema_version: u32,
    pub(crate) package: String,
    pub(crate) version: String,
    pub(crate) entries: Vec<Entry>,
}

const EMBEDDED: &[&str] = include!(concat!(env!("OUT_DIR"), "/indexes.rs"));

fn public(name: &str) -> bool {
    name.split('.').all(|s| !s.starts_with('_'))
}

fn parameters(node: &Syntax<'_>) -> Vec<String> {
    node.field("parameters")
        .into_iter()
        .flat_map(|p| p.children().collect::<Vec<_>>())
        .filter_map(|p| {
            if p.kind() == "identifier" {
                Some(p.text().into_owned())
            } else {
                p.dfs()
                    .find(|n| n.kind() == "identifier")
                    .map(|n| n.text().into_owned())
            }
        })
        .filter(|p| p != "self" && p != "cls")
        .collect()
}

fn introduction(node: &Syntax<'_>) -> String {
    let Some(body) = node.field("body") else {
        return String::new();
    };
    let Some(first) = body
        .children()
        .find(|n| n.is_named() && n.kind() != "comment")
    else {
        return String::new();
    };
    if first.kind() != "expression_statement" {
        return String::new();
    }
    let Some(string) = first.children().find(|n| n.kind() == "string") else {
        return String::new();
    };
    let text = string.text();
    let Some(start) = text.find(['\'', '"']) else {
        return String::new();
    };
    let quote = &text[start..=start];
    let width = if text[start..].starts_with(&quote.repeat(3)) {
        3
    } else {
        1
    };
    if text.len() < start + width * 2 {
        return String::new();
    }
    text[start + width..text.len() - width]
        .trim()
        .lines()
        .map(str::trim)
        .take_while(|s| {
            ![
                "Parameters",
                "Returns",
                "Attributes",
                "Examples",
                "Notes",
                "References",
                "Args:",
                "Returns:",
            ]
            .contains(s)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summary(node: &Syntax<'_>) -> String {
    introduction(node)
        .lines()
        .take_while(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn definitions<'a>(node: &Syntax<'a>) -> Vec<Syntax<'a>> {
    node.children()
        .filter_map(|n| {
            if n.kind() == "decorated_definition" {
                n.field("definition")
            } else if matches!(
                n.kind().as_ref(),
                "class_definition" | "function_definition"
            ) {
                Some(n)
            } else {
                None
            }
        })
        .collect()
}

fn files(root: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for item in std::fs::read_dir(root)? {
        let item = item?;
        if item.file_type()?.is_symlink() {
            continue;
        }
        let path = item.path();
        let name = item.file_name();
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            "tests"
                | "test"
                | "testing"
                | "validators"
                | "graph_objs"
                | "vendor"
                | "vendored"
                | "externals"
                | "_vendor"
                | "__pycache__"
        ) || name.starts_with('.')
        {
            continue;
        }
        if path.is_dir() {
            files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "py") {
            out.push(path);
        }
    }
    Ok(())
}

/// Reads definitions and import edges, then resolves re-exports to a fixed point.
pub(crate) fn generate(source: &Path, package: &str, version: &str) -> Result<Index> {
    ensure!(
        !package.is_empty()
            && package
                .split('.')
                .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')),
        "invalid package name"
    );
    let root = if source.join(package).is_dir() {
        source.join(package)
    } else {
        source.to_path_buf()
    };
    let mut paths = Vec::new();
    files(&root, &mut paths)?;
    paths.sort();
    let mut entries = BTreeMap::new();
    let mut bindings = BTreeMap::<String, String>::new();
    let mut stars = Vec::new();
    for path in paths {
        let relative = path.strip_prefix(&root)?.with_extension("");
        let parts: Vec<_> = relative
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let init = parts.last().is_some_and(|p| p == "__init__");
        let tail = parts[..parts.len() - usize::from(init)].join(".");
        let module = if tail.is_empty() {
            package.to_owned()
        } else {
            format!("{package}.{tail}")
        };
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        let tree = scan::parse(&text);
        let top = tree.root();
        read_definitions(&top, &module, package, &mut entries, &mut bindings);
        read_imports(&top, &module, usize::from(init), &mut bindings, &mut stars);
        read_aliases(&top, &module, &mut bindings);
    }
    resolve_exports(&mut bindings, &stars);
    let mut names = BTreeMap::<String, String>::new();
    for (alias, target) in bindings {
        if !public(&alias) || !entries.contains_key(&target) {
            continue;
        }
        let best = names.entry(target).or_insert_with(|| alias.clone());
        if (alias.split('.').count(), alias.len(), &alias)
            < (best.split('.').count(), best.len(), best)
        {
            *best = alias;
        }
    }
    let mut result = Vec::new();
    for (target, name) in names {
        if let Some(mut entry) = entries.remove(&target) {
            entry.name = name;
            result.push(entry);
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    attach_rules(&mut result, &[package])?;
    ensure!(
        !result.is_empty(),
        "no public symbols found in {}",
        root.display()
    );
    Ok(Index {
        schema_version: 1,
        package: package.into(),
        version: version.into(),
        entries: result,
    })
}

pub(crate) fn load(extra: Option<&Path>) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for text in EMBEDDED {
        let index: Index = serde_json::from_str(text).context("invalid embedded index")?;
        entries.extend(index.entries);
    }
    if let Some(path) = extra {
        let index: Index = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        ensure!(index.schema_version == 1, "unsupported index schema");
        entries.retain(|e| e.package != index.package);
        entries.extend(index.entries);
    }
    let packs = pack::load(None)?;
    attach_rules(
        &mut entries,
        &packs.iter().map(|p| p.module).collect::<Vec<_>>(),
    )?;
    entries.extend(catalog::entries(&packs).iter().map(|r| {
        let r = r.to_json();
        Entry {
            kind: Kind::Rule,
            package: r["package"].as_str().unwrap_or_default().into(),
            name: format!(
                "{}/{}",
                r["package"].as_str().unwrap_or_default(),
                r["rule"].as_str().unwrap_or_default()
            ),
            callable: None,
            parameters: Vec::new(),
            summary: format!(
                "{}\n\n{}",
                r["message"].as_str().unwrap_or_default(),
                r["doc"].as_str().unwrap_or_default()
            ),
            keywords: Vec::new(),
            members: Vec::new(),
            rules: Vec::new(),
        }
    }));
    Ok(entries)
}

fn tokens(text: &str) -> Vec<String> {
    let mut split = String::new();
    let mut previous = false;
    for ch in text.chars() {
        if ch.is_uppercase() && previous {
            split.push(' ');
        }
        split.extend(ch.to_lowercase());
        previous = ch.is_lowercase();
    }
    split
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1 && !s.chars().all(|c| c.is_ascii_digit()))
        .filter(|s| {
            ![
                "the", "and", "for", "with", "from", "into", "that", "this", "how", "can", "use",
                "data", "using", "going", "to", "of", "in", "on", "is", "an", "my", "it", "all",
            ]
            .contains(s)
        })
        .map(|s| {
            let s = s.strip_suffix("ing").filter(|_| s.len() > 6).unwrap_or(s);
            let s = s.strip_suffix("ed").filter(|_| s.len() > 5).unwrap_or(s);
            let s = s
                .strip_suffix('s')
                .filter(|_| s.len() > 4 && !s.ends_with("ss"))
                .unwrap_or(s);
            match s {
                "estimator" | "predictor" => "model",
                "encoding" | "encoder" | "encode" | "cod" | "coding" => "encod",
                "categorical" | "categorie" | "category" => "categor",
                "average" => "mean",
                "transformer" | "transformation" | "transform" => "transform",
                "builder" | "build" | "construct" | "constructs" => "build",
                "separately" | "separate" => "separate",
                "thresholder" | "threshold" => "threshold",
                "anova" => "variance",
                "chart" => "plot",
                "regression" | "regressor" => "regress",
                "classification" | "classifier" => "classify",
                "prediction" => "predict",
                _ => s,
            }
            .to_owned()
        })
        .collect()
}

pub(crate) fn find<'a>(
    entries: &'a [Entry],
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
) -> Vec<&'a Entry> {
    let phrase_text = query.to_lowercase();
    let query: BTreeSet<_> = tokens(query).into_iter().collect();
    let docs: Vec<_> = entries
        .iter()
        .map(|e| {
            let leaf = e.name.rsplit('.').next().unwrap_or(&e.name);
            let mut counts = BTreeMap::<String, f64>::new();
            let fields = [
                (leaf.to_owned(), 3.0),
                (e.name.clone(), 0.3),
                (e.summary.clone(), 1.0),
                (e.parameters.join(" "), 0.3),
                (e.keywords.join(" "), 0.2),
            ];
            for (text, weight) in fields {
                for word in tokens(&text) {
                    *counts.entry(word).or_default() += weight;
                }
            }
            let length = tokens(&e.summary).len() + tokens(leaf).len() * 3;
            (counts, u32::try_from(length).unwrap_or(u32::MAX))
        })
        .collect();
    let n = f64::from(u32::try_from(docs.len()).unwrap_or(u32::MAX));
    let avg = docs.iter().map(|(_, len)| f64::from(*len)).sum::<f64>() / n.max(1.0);
    let weights: BTreeMap<_, _> = query
        .iter()
        .map(|q| {
            let df = f64::from(
                u32::try_from(docs.iter().filter(|(d, _)| d.contains_key(q)).count())
                    .unwrap_or(u32::MAX),
            );
            (q, ((n - df + 0.5) / (df + 0.5) + 1.0).ln())
        })
        .collect();
    let mut ranked: Vec<_> = entries
        .iter()
        .zip(&docs)
        .filter(|(e, _)| package.is_none_or(|p| e.package == p) && kind.is_none_or(|k| e.kind == k))
        .map(|(entry, (counts, len))| {
            let score = weights
                .iter()
                .map(|(q, idf)| {
                    let tf = counts.get(*q).copied().unwrap_or_default();
                    idf * tf * 2.2 / (tf + 1.2 * (0.25 + 0.75 * f64::from(*len) / avg.max(1.0)))
                })
                .sum::<f64>();
            let matched = query.iter().filter(|q| counts.contains_key(*q)).count();
            let matched = f64::from(u32::try_from(matched).unwrap_or(u32::MAX));
            let names: BTreeSet<_> = tokens(entry.name.rsplit('.').next().unwrap_or(&entry.name))
                .into_iter()
                .collect();
            let name_score = weights
                .iter()
                .filter(|(q, _)| names.contains(**q))
                .map(|(_, idf)| idf * 2.0)
                .sum::<f64>();
            let phrase = if query.len() > 1 && entry.summary.to_lowercase().contains(&phrase_text) {
                12.0
            } else {
                0.0
            };
            let raw_leaf = entry
                .name
                .rsplit('.')
                .next()
                .unwrap_or(&entry.name)
                .to_lowercase();
            let leaf = entry.name.rsplit('.').next().unwrap_or(&entry.name);
            let acronym =
                leaf.len() > 2 && leaf.chars().all(|c| c.is_uppercase() || c.is_ascii_digit());
            let exact_name = if acronym
                && phrase_text
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|s| s == raw_leaf)
            {
                30.0
            } else {
                0.0
            };
            let depth = u32::try_from(entry.name.split('.').count().saturating_sub(2).max(1))
                .unwrap_or(u32::MAX);
            let depth_penalty = if entry.kind == Kind::Symbol {
                f64::from(depth).sqrt()
            } else {
                1.0
            };
            (
                entry,
                (score + name_score + 3.0 * matched + phrase + exact_name) / depth_penalty,
            )
        })
        .filter(|(_, s)| *s > 0.0)
        .collect();
    ranked.sort_by(|(a, sa), (b, sb)| sb.total_cmp(sa).then_with(|| a.name.cmp(&b.name)));
    ranked.into_iter().take(5).map(|(e, _)| e).collect()
}

/// Reads wheel metadata without executing the distribution.
pub(crate) fn source_version(source: &Path, package: &str) -> Result<String> {
    for item in std::fs::read_dir(source)? {
        let path = item?.path();
        if path.extension().is_none_or(|e| e != "dist-info") {
            continue;
        }
        let top = std::fs::read_to_string(path.join("top_level.txt")).unwrap_or_default();
        let record = std::fs::read_to_string(path.join("RECORD")).unwrap_or_default();
        if top.lines().any(|line| line == package)
            || record
                .lines()
                .any(|line| line.starts_with(&format!("{package}/")))
        {
            let metadata = std::fs::read_to_string(path.join("METADATA"))?;
            return metadata
                .lines()
                .find_map(|line| line.strip_prefix("Version: "))
                .map(str::to_owned)
                .context("METADATA has no Version");
        }
    }
    anyhow::bail!("no distribution metadata for {package}; pass --version for a source checkout")
}

fn read_definitions(
    top: &Syntax<'_>,
    module: &str,
    package: &str,
    entries: &mut BTreeMap<String, Entry>,
    bindings: &mut BTreeMap<String, String>,
) {
    for def in definitions(top) {
        let Some(name) = def.field("name") else {
            continue;
        };
        let name = name.text();
        if name.starts_with('_') {
            continue;
        }
        let class = def.kind() == "class_definition";
        let methods = def
            .field("body")
            .map(|b| definitions(&b))
            .unwrap_or_default();
        let params = if class {
            methods
                .iter()
                .find(|m| m.field("name").is_some_and(|n| n.text() == "__init__"))
                .map(parameters)
                .unwrap_or_default()
        } else {
            parameters(&def)
        };
        let full = format!("{module}.{name}");
        entries.insert(
            full.clone(),
            Entry {
                kind: Kind::Symbol,
                package: package.to_owned(),
                name: full.clone(),
                callable: Some(if class { "class" } else { "function" }.into()),
                parameters: params,
                summary: {
                    let doc = summary(&def);
                    if doc.is_empty() && class {
                        methods
                            .iter()
                            .find(|m| m.field("name").is_some_and(|n| n.text() == "__init__"))
                            .map(summary)
                            .unwrap_or_default()
                    } else {
                        doc
                    }
                },
                keywords: tokens(&introduction(&def))
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                members: methods
                    .iter()
                    .filter_map(|m| {
                        let name = m.field("name")?.text().into_owned();
                        if !public(&name) {
                            return None;
                        }
                        Some(Member {
                            name,
                            parameters: parameters(m),
                            summary: summary(m),
                        })
                    })
                    .collect(),
                rules: Vec::new(),
            },
        );
        bindings.insert(full.clone(), full);
    }
}
fn read_imports(
    top: &Syntax<'_>,
    module: &str,
    package_depth: usize,
    bindings: &mut BTreeMap<String, String>,
    stars: &mut Vec<(String, String)>,
) {
    // Imports inside conditionals are also used by libraries for optional exports.
    for import in top.dfs().filter(|n| {
        n.kind() == "import_from_statement"
            && !n.ancestors().any(|p| {
                matches!(
                    p.kind().as_ref(),
                    "function_definition" | "class_definition"
                )
            })
    }) {
        let Some(from) = import.field("module_name") else {
            continue;
        };
        let from_text = from.text();
        let dots = from_text.chars().take_while(|c| *c == '.').count();
        let base = if dots == 0 {
            from_text.into_owned()
        } else {
            let mut parent: Vec<_> = module.split('.').collect();
            let remove = dots - package_depth;
            if remove >= parent.len() {
                continue;
            }
            parent.truncate(parent.len() - remove);
            let suffix = from_text.trim_start_matches('.');
            if suffix.is_empty() {
                parent.join(".")
            } else {
                format!("{}.{suffix}", parent.join("."))
            }
        };
        for child in import
            .children()
            .filter(|n| n.range().start > from.range().end)
        {
            match child.kind().as_ref() {
                "dotted_name" => {
                    bindings.insert(
                        format!("{module}.{}", child.text()),
                        format!("{base}.{}", child.text()),
                    );
                }
                "aliased_import" => {
                    if let (Some(name), Some(alias)) = (child.field("name"), child.field("alias")) {
                        bindings.insert(
                            format!("{module}.{}", alias.text()),
                            format!("{base}.{}", name.text()),
                        );
                    }
                }
                "wildcard_import" => stars.push((module.to_owned(), base.clone())),
                _ => {}
            }
        }
    }
}
fn read_aliases(top: &Syntax<'_>, module: &str, bindings: &mut BTreeMap<String, String>) {
    for assignment in top.dfs().filter(|n| {
        n.kind() == "assignment"
            && !n.ancestors().any(|p| {
                matches!(
                    p.kind().as_ref(),
                    "function_definition" | "class_definition"
                )
            })
    }) {
        let (Some(left), Some(right)) = (assignment.field("left"), assignment.field("right"))
        else {
            continue;
        };
        if left.kind() != "identifier"
            || !matches!(right.kind().as_ref(), "identifier" | "attribute")
        {
            continue;
        }
        let value = right.text();
        let (head, tail) = value.split_once('.').unwrap_or((&value, ""));
        let local = format!("{module}.{head}");
        let target = bindings.get(&local).cloned().unwrap_or(local);
        let target = if tail.is_empty() {
            target
        } else {
            format!("{target}.{tail}")
        };
        bindings.insert(format!("{module}.{}", left.text()), target);
    }
}

fn resolve_exports(bindings: &mut BTreeMap<String, String>, stars: &[(String, String)]) {
    for _ in 0..64 {
        let mut changes = Vec::new();
        for (alias, target) in bindings.iter() {
            if let Some(next) = bindings.get(target)
                && next != target
            {
                changes.push((alias.clone(), next.clone()));
            }
        }
        for (module, base) in stars {
            let prefix = format!("{base}.");
            for (name, target) in bindings.iter() {
                if let Some(leaf) = name.strip_prefix(&prefix)
                    && !leaf.contains('.')
                    && public(leaf)
                {
                    let alias = format!("{module}.{leaf}");
                    if !bindings.contains_key(&alias) {
                        changes.push((alias, target.clone()));
                    }
                }
            }
        }
        if changes.is_empty() {
            break;
        }
        bindings.extend(changes);
    }
}

fn attach_rules(entries: &mut [Entry], packs: &[&str]) -> Result<()> {
    for pack in packs {
        let links = pack::symbol_rules(pack)?;
        for entry in entries.iter_mut().filter(|e| e.package == *pack) {
            entry.rules = links
                .get(entry.name.rsplit('.').next().unwrap_or(&entry.name))
                .cloned()
                .unwrap_or_default();
        }
    }
    Ok(())
}
