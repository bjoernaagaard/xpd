//! Source-only library discovery and keyword search.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::SupportLang;
use serde::{Deserialize, Serialize};

use crate::{pack, scan};

type Syntax<'a> = Node<'a, StrDoc<SupportLang>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Symbol,
    Rule,
    Recipe,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipe: Option<crate::recipe::Recipe>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Index {
    pub(crate) schema_version: u32,
    pub(crate) package: String,
    pub(crate) version: String,
    pub(crate) entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipes: Option<crate::recipe::Stats>,
}

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

fn documentation(node: &Syntax<'_>) -> String {
    let body = node.field("body").unwrap_or_else(|| node.clone());
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
        .collect::<Vec<_>>()
        .join("\n")
}

fn introduction(node: &Syntax<'_>) -> String {
    documentation(node)
        .lines()
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
            let init = std::fs::read_to_string(path.join("__init__.py")).unwrap_or_default();
            let tree = scan::parse(&init);
            let description = introduction(&tree.root()).to_lowercase();
            if [
                "should be accessed through",
                "internal use only",
                "not part of the public api",
                "vendored",
            ]
            .iter()
            .any(|marker| description.contains(marker))
            {
                continue;
            }
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
    let mut classes = BTreeMap::new();
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
        for def in definitions(&top)
            .into_iter()
            .filter(|d| d.kind() == "class_definition")
        {
            let Some(name) = def.field("name") else {
                continue;
            };
            let bases = def
                .field("superclasses")
                .map(|b| {
                    b.children()
                        .filter(|n| matches!(n.kind().as_ref(), "identifier" | "attribute"))
                        .map(|n| format!("{module}.{}", n.text()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let owns_init = def.field("body").is_some_and(|b| {
                definitions(&b)
                    .iter()
                    .any(|m| m.field("name").is_some_and(|n| n.text() == "__init__"))
            });
            classes.insert(format!("{module}.{}", name.text()), (bases, owns_init));
        }
        read_imports(&top, &module, usize::from(init), &mut bindings, &mut stars);
        read_aliases(&top, &module, &mut bindings);
    }
    resolve_exports(&mut bindings, &stars);
    let inherited = inherit(&mut entries, &classes, &bindings);
    eprintln!("{package}: {inherited} classes gained constructor parameters");
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
        recipes: None,
    })
}

fn inherit(
    entries: &mut BTreeMap<String, Entry>,
    classes: &BTreeMap<String, (Vec<String>, bool)>,
    bindings: &BTreeMap<String, String>,
) -> usize {
    let snapshot = entries.clone();
    let resolve = |name: &str| {
        let mut parts: Vec<_> = name.split('.').collect();
        let mut suffix = Vec::new();
        while !parts.is_empty() {
            if let Some(target) = bindings.get(&parts.join(".")) {
                suffix.reverse();
                return if suffix.is_empty() {
                    target.clone()
                } else {
                    format!("{target}.{}", suffix.join("."))
                };
            }
            if let Some(part) = parts.pop() {
                suffix.push(part);
            }
        }
        name.to_owned()
    };
    let mut gained = 0;
    for (name, (bases, owns_init)) in classes {
        let Some(entry) = entries.get_mut(name) else {
            continue;
        };
        let mut queue: std::collections::VecDeque<_> = bases.iter().map(|b| resolve(b)).collect();
        let mut seen = BTreeSet::new();
        let mut constructor = *owns_init;
        while let Some(base) = queue.pop_front() {
            if !seen.insert(base.clone()) {
                continue;
            }
            let Some(parent) = snapshot.get(&base) else {
                continue;
            };
            let Some((ancestors, parent_init)) = classes.get(&base) else {
                continue;
            };
            if !constructor && *parent_init {
                entry.parameters.clone_from(&parent.parameters);
                constructor = true;
                gained += usize::from(!entry.parameters.is_empty());
            }
            for member in &parent.members {
                if !entry.members.iter().any(|m| m.name == member.name) {
                    entry.members.push(member.clone());
                }
            }
            queue.extend(ancestors.iter().map(|b| resolve(b)));
        }
    }
    gained
}

#[derive(Debug)]
pub(crate) struct Library {
    local: Option<Vec<Entry>>,
}

pub(crate) fn load(extra: Option<&Path>) -> Result<Library> {
    let local = if let Some(path) = extra {
        let index: Index = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        ensure!(index.schema_version == 1, "unsupported index schema");
        let mut entries = RECORDS
            .iter()
            .filter(|e| e.package != index.package || e.kind == Kind::Rule)
            .map(Stored::entry)
            .collect::<std::result::Result<Vec<Entry>, _>>()?;
        entries.extend(index.entries);
        Some(entries)
    } else {
        None
    };
    Ok(Library { local })
}

pub(crate) fn api(library: &Library, name: &str) -> Result<Option<Entry>> {
    if let Some(entries) = &library.local {
        return Ok(entries.iter().find(|e| e.name == name).cloned());
    }
    RECORDS
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.entry().context("embedded entry"))
        .transpose()
}

use crate::search::tokens;

#[derive(Debug)]
struct Stored {
    name: &'static str,
    package: &'static str,
    kind: Kind,
    summary: &'static str,
    start: u32,
    len: u16,
}
const DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/entries.bin"));
const STRINGS: &[&str] = include!(concat!(env!("OUT_DIR"), "/strings.rs"));

impl Stored {
    fn entry(&self) -> Result<Entry, serde_json::Error> {
        let start = usize::try_from(self.start).expect("entry offset fits usize");
        let end = start + usize::from(self.len);
        serde_json::from_value(decode(&mut &DATA[start..end]))
    }
}

fn integer(data: &mut &[u8]) -> usize {
    let (bytes, rest) = data.split_at(2);
    *data = rest;
    usize::from(u16::from_le_bytes(
        bytes.try_into().expect("build writes u16 IDs"),
    ))
}

fn decode(data: &mut &[u8]) -> serde_json::Value {
    use serde_json::Value;
    let tag = data[0];
    *data = &data[1..];
    match tag {
        0 => Value::Null,
        1 => Value::Bool(false),
        2 => Value::Bool(true),
        3 => Value::String(STRINGS[integer(data)].to_owned()),
        4 => Value::Array((0..integer(data)).map(|_| decode(data)).collect()),
        5 => Value::Object(
            (0..integer(data))
                .map(|_| {
                    let key = STRINGS[integer(data)].to_owned();
                    (key, decode(data))
                })
                .collect(),
        ),
        _ => unreachable!("build writes validated entry tags"),
    }
}

const POSTINGS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/postings.bin"));
const TABLE: &[(&str, u32, u32)] = include!(concat!(env!("OUT_DIR"), "/search.rs"));
const RECORDS: &[Stored] = include!(concat!(env!("OUT_DIR"), "/records.rs"));

#[derive(Debug, Default, Serialize)]
pub(crate) struct Matches {
    pub(crate) symbols: Vec<Entry>,
    pub(crate) rules: Vec<Entry>,
    pub(crate) recipes: Vec<Entry>,
}

impl Matches {
    fn collect(entries: impl IntoIterator<Item = Entry>, limit: Option<usize>) -> Self {
        let mut found = Self::default();
        for entry in entries {
            let (group, default) = match entry.kind {
                Kind::Symbol => (&mut found.symbols, 5),
                Kind::Rule => (&mut found.rules, 3),
                Kind::Recipe => (&mut found.recipes, 3),
            };
            if group.len() < limit.unwrap_or(default) {
                group.push(entry);
            }
        }
        found
    }
}

pub(crate) fn find(
    library: &Library,
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
    limit: Option<usize>,
) -> Matches {
    if let Some(entries) = &library.local {
        return Matches::collect(
            find_dynamic(entries, query, package, kind)
                .into_iter()
                .cloned(),
            limit,
        );
    }
    let words: BTreeSet<_> = search_terms(query).into_iter().collect();
    let mut scores = vec![0.0; RECORDS.len()];
    for word in &words {
        if let Ok(i) = TABLE.binary_search_by_key(&word.as_str(), |(w, _, _)| *w) {
            let start = usize::try_from(TABLE[i].1).expect("posting offset fits usize");
            let end = usize::try_from(TABLE[i].2).expect("posting offset fits usize");
            for posting in POSTINGS[start..end].as_chunks::<4>().0 {
                let entry = usize::from(u16::from_le_bytes(
                    posting[..2]
                        .try_into()
                        .expect("build writes two byte entry IDs"),
                ));
                let score = u16::from_le_bytes(
                    posting[2..]
                        .try_into()
                        .expect("build writes two byte scores"),
                );
                scores[entry] += f64::from(score) / 256.0;
            }
        }
    }
    let mut ranked: Vec<_> = RECORDS
        .iter()
        .zip(scores)
        .filter(|(e, score)| {
            *score > 0.0
                && package.is_none_or(|p| e.package == p)
                && kind.is_none_or(|k| e.kind == k)
        })
        .map(|(e, score)| {
            (
                e,
                rank_score(e.name, e.summary, e.kind, query, words.len(), score),
            )
        })
        .collect();
    ranked.sort_by(|(a, sa), (b, sb)| sb.total_cmp(sa).then_with(|| a.name.cmp(b.name)));
    // Select within each kind before decoding the embedded entries.
    let mut counts = [0; 3];
    Matches::collect(
        ranked
            .into_iter()
            .filter(|(e, _)| {
                let (slot, default) = match e.kind {
                    Kind::Symbol => (0, 5),
                    Kind::Rule => (1, 3),
                    Kind::Recipe => (2, 3),
                };
                counts[slot] += 1;
                counts[slot] <= limit.unwrap_or(default)
            })
            .map(|(e, _)| e.entry().expect("build validated index entry")),
        limit,
    )
}

fn find_dynamic<'a>(
    entries: &'a [Entry],
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
) -> Vec<&'a Entry> {
    let documents = entries
        .iter()
        .map(|e| {
            [
                e.name.rsplit('.').next().unwrap_or(&e.name).to_owned(),
                e.name.clone(),
                e.summary.clone(),
                serde_json::to_string(&e.parameters).unwrap_or_default(),
                serde_json::to_string(&e.keywords).unwrap_or_default(),
                e.members
                    .iter()
                    .map(|m| m.summary.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ]
        })
        .collect::<Vec<_>>();
    let table = crate::search::weights(&documents);
    let words: BTreeSet<_> = search_terms(query).into_iter().collect();
    let mut scores = vec![0.0; entries.len()];
    for word in &words {
        if let Some(postings) = table.get(word) {
            for &(i, score) in postings {
                scores[i] += score;
            }
        }
    }
    let mut ranked = entries
        .iter()
        .zip(scores)
        .filter(|(e, score)| {
            *score > 0.0
                && package.is_none_or(|p| e.package == p)
                && kind.is_none_or(|k| e.kind == k)
        })
        .map(|(e, score)| {
            (
                e,
                rank_score(&e.name, &e.summary, e.kind, query, words.len(), score),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(a, sa), (b, sb)| sb.total_cmp(sa).then_with(|| a.name.cmp(&b.name)));
    ranked.into_iter().map(|(e, _)| e).collect()
}

fn search_terms(query: &str) -> Vec<String> {
    tokens(query)
}

fn rank_score(
    name: &str,
    summary: &str,
    kind: Kind,
    query: &str,
    words: usize,
    mut score: f64,
) -> f64 {
    let phrase = query.to_lowercase();
    if words > 1 && summary.to_lowercase().contains(&phrase) {
        score += 12.0;
    }
    let leaf = name.rsplit('.').next().unwrap_or(name);
    if leaf.len() > 2
        && leaf.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())
        && phrase
            .split(|c: char| !c.is_alphanumeric())
            .any(|s| s == leaf.to_lowercase())
    {
        score += 30.0;
    }
    let depth = u32::try_from(name.split('.').count().saturating_sub(2)).unwrap_or(u32::MAX);
    if kind == Kind::Symbol {
        score /= 1.0 + 0.25 * f64::from(depth);
    }
    score
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
                keywords: vec![
                    documentation(&def)
                        .lines()
                        .take_while(|line| *line != "Examples")
                        .collect::<Vec<_>>()
                        .join(" "),
                ],
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
                recipe: None,
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
    for import in top.dfs().filter(|n| {
        n.kind() == "import_statement"
            && !n.ancestors().any(|p| {
                matches!(
                    p.kind().as_ref(),
                    "function_definition" | "class_definition"
                )
            })
    }) {
        for child in import.children() {
            if child.kind() == "aliased_import" {
                if let (Some(name), Some(alias)) = (child.field("name"), child.field("alias")) {
                    bindings.insert(
                        format!("{module}.{}", alias.text()),
                        name.text().into_owned(),
                    );
                }
            } else if child.kind() == "dotted_name" {
                let text = child.text();
                let head = text.split('.').next().unwrap_or(&text);
                bindings.insert(format!("{module}.{head}"), head.to_owned());
            }
        }
    }
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
