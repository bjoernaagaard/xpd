//! Source-only library discovery and keyword search.
use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{pack, venv};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) inputs: Option<Inputs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Inputs {
    pub(crate) griffe: String,
    pub(crate) python: String,
    pub(crate) archive: Option<String>,
    pub(crate) examples: Option<Examples>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Examples {
    pub(crate) archive: String,
    pub(crate) path: String,
    pub(crate) url: String,
    pub(crate) license: String,
    pub(crate) notice: Option<String>,
}

/// Gets the public API from Griffe without importing the library.
pub(crate) fn generate(source: &Path, package: &str, version: &str) -> Result<Index> {
    let output = std::process::Command::new(venv::project_python())
        .arg("-I")
        .arg("-c")
        .arg(include_str!("../py/index.py"))
        .arg(source)
        .arg(package)
        .arg(version)
        .output()
        .context("could not start Python for Griffe")?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let mut index: Index =
        serde_json::from_slice(&output.stdout).context("invalid Griffe index")?;
    ensure!(!index.entries.is_empty(), "Griffe found no public symbols");
    attach_rules(&mut index.entries, &[package])?;
    Ok(index)
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
