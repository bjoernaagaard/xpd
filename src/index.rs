//! Source-only library discovery, and search by keywords and by meaning.
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::embed::{DIMENSIONS, Model};
use crate::{pack, venv};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Symbol,
    Rule,
    Recipe,
}

impl Kind {
    /// The kind's group in `Matches`, and the number of results it shows by default.
    fn group(self) -> (usize, usize) {
        match self {
            Kind::Symbol => (0, 5),
            Kind::Rule => (1, 3),
            Kind::Recipe => (2, 3),
        }
    }
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
    /// Loaded on the first search, so `api` never loads it. `None` inside when it is absent.
    model: OnceCell<Option<Model>>,
}

impl Library {
    /// The same library, searched by keywords only.
    pub(crate) fn keywords_only(self) -> Self {
        Self {
            model: OnceCell::from(None),
            ..self
        }
    }

    /// The embedding model, which is used for the embedded records only.
    fn model(&self) -> Option<&Model> {
        if self.local.is_some() {
            return None;
        }
        self.model.get_or_init(Model::open).as_ref()
    }

    pub(crate) fn has_model(&self) -> bool {
        self.model().is_some()
    }
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
    Ok(Library {
        local,
        model: OnceCell::new(),
    })
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
/// `DIMENSIONS` signed bytes for each of `RECORDS`, all zero for a record without a vector.
const VECTORS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vectors.bin"));
/// The start of `indexes/vectors.bin`, which build.rs reads.
const VECTORS_HEADER: &[u8] = b"xpd vectors 256\n";
/// Reciprocal rank fusion adds `weight / (FUSION_K + rank)` from each ranking.
const FUSION_K: f64 = 60.0;

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
            let (_, default) = entry.kind.group();
            let group = match entry.kind {
                Kind::Symbol => &mut found.symbols,
                Kind::Rule => &mut found.rules,
                Kind::Recipe => &mut found.recipes,
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
    let passes =
        |e: &Stored| package.is_none_or(|p| e.package == p) && kind.is_none_or(|k| e.kind == k);
    let meaning = library.model().and_then(|model| {
        model
            .query(query)
            .inspect_err(|e| {
                eprintln!("xpd: cannot embed the query: {e:#}. Search uses keywords only.");
            })
            .ok()
    });
    let mut ranked = keyword_ranking(query, passes);
    if let Some(meaning) = meaning {
        ranked = fuse(&ranked, &meaning_ranking(&meaning, passes));
    }
    // Select within each kind before decoding the embedded entries.
    let mut counts = [0; 3];
    Matches::collect(
        ranked
            .into_iter()
            .filter(|(i, _)| {
                let (slot, default) = RECORDS[*i].kind.group();
                counts[slot] += 1;
                counts[slot] <= limit.unwrap_or(default)
            })
            .map(|(i, _)| RECORDS[i].entry().expect("build validated index entry")),
        limit,
    )
}

/// Sorts `(record, score)` pairs by score, best first, and by name between equal scores.
fn sort_ranking(ranked: &mut [(usize, f64)]) {
    ranked.sort_by(|(a, sa), (b, sb)| {
        sb.total_cmp(sa)
            .then_with(|| RECORDS[*a].name.cmp(RECORDS[*b].name))
    });
}

/// The records that share a keyword with the query, best first.
fn keyword_ranking(query: &str, passes: impl Fn(&Stored) -> bool) -> Vec<(usize, f64)> {
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
    let mut ranked: Vec<_> = scores
        .into_iter()
        .enumerate()
        .filter(|(i, score)| *score > 0.0 && passes(&RECORDS[*i]))
        .map(|(i, score)| {
            let e = &RECORDS[i];
            (
                i,
                rank_score(e.name, e.summary, e.kind, query, words.len(), score),
            )
        })
        .collect();
    sort_ranking(&mut ranked);
    ranked
}

/// Every record with a vector, by its cosine similarity to the query's vector, best first.
fn meaning_ranking(meaning: &[f32], passes: impl Fn(&Stored) -> bool) -> Vec<(usize, f64)> {
    let mut ranked: Vec<_> = VECTORS
        .as_chunks::<DIMENSIONS>()
        .0
        .iter()
        .enumerate()
        .filter(|(i, _)| passes(&RECORDS[*i]))
        .filter_map(|(i, vector)| {
            let (dot, norm) =
                vector
                    .iter()
                    .zip(meaning)
                    .fold((0.0, 0.0), |(dot, norm), (&v, &m)| {
                        let v = f32::from(v.cast_signed());
                        (dot + v * m, norm + v * v)
                    });
            (norm > 0.0).then(|| (i, f64::from(dot / norm.sqrt())))
        })
        .collect();
    sort_ranking(&mut ranked);
    ranked
}

/// Reciprocal rank fusion of the two rankings, within each kind. The keyword ranking weighs a
/// little more, and its first three get a bonus, so an exact match keeps its place. The values
/// pass every checked question in indexes/questions.json, and 1.0 or 1.3 for the weight do not.
fn fuse(keywords: &[(usize, f64)], meaning: &[(usize, f64)]) -> Vec<(usize, f64)> {
    let mut fused = BTreeMap::<usize, f64>::new();
    for (ranking, weight, bonus) in [
        (keywords, 1.15, [0.05, 0.02, 0.02]),
        (meaning, 1.0, [0.0; 3]),
    ] {
        let mut counts = [0_u32; 3];
        for &(i, _) in ranking {
            let (slot, _) = RECORDS[i].kind.group();
            counts[slot] += 1;
            let rank = counts[slot];
            let top = usize::try_from(rank - 1)
                .ok()
                .and_then(|place| bonus.get(place))
                .copied()
                .unwrap_or_default();
            *fused.entry(i).or_default() += weight / (FUSION_K + f64::from(rank)) + top;
        }
    }
    let mut ranked: Vec<_> = fused.into_iter().collect();
    sort_ranking(&mut ranked);
    ranked
}

/// The names of the embedded records that have no vector.
pub(crate) fn records_without_vector() -> Vec<&'static str> {
    RECORDS
        .iter()
        .zip(VECTORS.as_chunks::<DIMENSIONS>().0)
        .filter(|(_, vector)| vector.iter().all(|&v| v == 0))
        .map(|(e, _)| e.name)
        .collect()
}

/// Writes the vector of every embedded record to `path`, as build.rs reads it. A record keeps
/// the vector that this build holds for it, and only the records without one are embedded, so
/// a new rule costs one text. Each vector is scaled so its largest value is 127. Search
/// compares directions, so the scale drops out.
pub(crate) fn write_vectors(path: &Path) -> Result<()> {
    let stored = VECTORS.as_chunks::<DIMENSIONS>().0;
    let missing: Vec<_> = (RECORDS.iter().zip(stored))
        .filter(|(_, vector)| vector.iter().all(|&v| v == 0))
        .map(|(e, _)| (e.name, e.summary))
        .collect();
    let mut embedded = if missing.is_empty() {
        Vec::new()
    } else {
        let model = Model::open().context("the embedding model is needed to write vectors")?;
        model.documents(&missing)?
    }
    .into_iter();
    let mut data = VECTORS_HEADER.to_vec();
    for (record, vector) in RECORDS.iter().zip(stored) {
        data.extend(u16::try_from(record.name.len())?.to_le_bytes());
        data.extend(record.name.as_bytes());
        if vector.iter().any(|&v| v != 0) {
            data.extend(vector);
            continue;
        }
        let vector = embedded
            .next()
            .context("every missing record was embedded")?;
        let largest = vector.iter().fold(f32::EPSILON, |m, v| m.max(v.abs()));
        data.extend(vector.iter().map(|v| {
            // The value is rounded and within -127..=127, so the cast loses nothing.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "rounded value within -127..=127"
            )]
            let byte = (v / largest * 127.0).round() as i8;
            byte.cast_unsigned()
        }));
    }
    eprintln!(
        "xpd: embedded {} records and kept {}",
        missing.len(),
        RECORDS.len() - missing.len()
    );
    std::fs::write(path, data).with_context(|| format!("cannot write {}", path.display()))
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
