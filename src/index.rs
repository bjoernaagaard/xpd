//! Source-only library discovery, and search by keywords and by meaning.
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::embed::{DIMENSIONS, Model};
use crate::outline::{self, Callable, ParameterText};
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
    /// The parameter list as the source writes it, as in `Entry`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) parameters_written: Vec<String>,
    pub(crate) summary: String,
    /// The member is a property, which is read and not called.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) property: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) kind: Kind,
    pub(crate) package: String,
    pub(crate) name: String,
    /// `class` or `function` for a symbol, and `method` or `property` for a member of a class
    /// that `api` was asked for.
    pub(crate) callable: Option<String>,
    pub(crate) parameters: Vec<String>,
    /// The parameter list as the source writes it: each parameter with its type annotation and
    /// its default, and a `*` or `/` that stands alone between parameters. Empty for a function
    /// that only overloads declare.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) parameters_written: Vec<String>,
    pub(crate) summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) keywords: Vec<String>,
    pub(crate) members: Vec<Member>,
    pub(crate) rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipe: Option<crate::recipe::Recipe>,
}

/// The version of an index file. Version 2 records which members of a class are properties, so
/// a member without `property` is a method. Version 1 left that out for every member.
const INDEX_SCHEMA: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Index {
    pub(crate) schema_version: u32,
    pub(crate) package: String,
    pub(crate) version: String,
    pub(crate) entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipes: Option<crate::recipe::Stats>,
}

/// A function or method whose parameters an entry takes. `receiver` says that Python binds its
/// first parameter to the object or the class, so a call does not pass it.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Definition {
    /// A definition in the source, by the file and the line where it starts.
    Source {
        name: String,
        file: PathBuf,
        line: usize,
        receiver: bool,
    },
    /// A function that Griffe builds, such as the `__init__` of a dataclass, which has no
    /// definition to read. Griffe writes its parameters from the fields of the class.
    Built {
        parameters: Vec<ParameterText>,
        receiver: bool,
    },
}

/// A member as py/index.py writes it, with the definitions whose parameters it takes.
#[derive(Debug, Deserialize)]
struct ResolvedMember {
    name: String,
    definitions: Vec<Definition>,
    summary: String,
    property: bool,
}

/// An entry as py/index.py writes it: Griffe's public name and documentation, and the
/// definitions whose parameters it takes.
#[derive(Debug, Deserialize)]
struct Resolved {
    kind: Kind,
    package: String,
    name: String,
    callable: Option<String>,
    definitions: Vec<Definition>,
    summary: String,
    keywords: Vec<String>,
    members: Vec<ResolvedMember>,
    rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolvedIndex {
    package: String,
    version: String,
    entries: Vec<Resolved>,
}

/// The functions and methods of each source file that an index reads, outlined once.
#[derive(Debug, Default)]
struct Sources(HashMap<PathBuf, Vec<Callable>>);

/// The parameters of a symbol, by name and as the source writes them.
#[derive(Debug)]
struct Parameters {
    names: Vec<String>,
    written: Vec<String>,
}

impl Sources {
    /// The parameters of `name`: those of each of its definitions, each name once, without the
    /// receiver of a method. Only one definition gives a written list, because several
    /// overloads write a parameter in several ways.
    fn parameters(&mut self, name: &str, definitions: &[Definition]) -> Result<Parameters> {
        let mut parameters: Vec<ParameterText> = Vec::new();
        for definition in definitions {
            let (mut read, receiver) = match definition {
                Definition::Source {
                    name: function,
                    file,
                    line,
                    receiver,
                } => {
                    if !self.0.contains_key(file) {
                        let src = std::fs::read_to_string(file)
                            .with_context(|| format!("cannot read {}", file.display()))?;
                        self.0.insert(file.clone(), outline::python_callables(&src));
                    }
                    let callable = (self.0[file].iter())
                        .find(|c| c.line == *line && c.name == *function)
                        .with_context(|| {
                            format!("{name}: no {function} at {}:{line}", file.display())
                        })?;
                    (callable.parameters.as_slice(), *receiver)
                }
                Definition::Built {
                    parameters,
                    receiver,
                } => (parameters.as_slice(), *receiver),
            };
            // The receiver is the first parameter whatever its name is, and `*args` takes it
            // together with the arguments of the call.
            if receiver
                && let [first, rest @ ..] = read
                && first.name.is_some()
                && !first.written.starts_with('*')
            {
                read = rest;
                // `def method(self, /, value)` leaves a `/` with no parameter before it.
                if let [slash, rest @ ..] = read
                    && slash.written == "/"
                {
                    read = rest;
                }
            }
            for parameter in read {
                // A `*` or `/` that stands alone has no name, and a definition writes it once.
                let known = (parameter.name.as_ref())
                    .is_some_and(|name| parameters.iter().any(|p| p.name.as_ref() == Some(name)));
                if !known {
                    parameters.push(parameter.clone());
                }
            }
        }
        let names = parameters.iter().filter_map(|p| p.name.clone()).collect();
        let written = match definitions {
            [_] => parameters.into_iter().map(|p| p.written).collect(),
            _ => Vec::new(),
        };
        Ok(Parameters { names, written })
    }
}

/// Gets the public API from Griffe without importing the library, and the parameters of each
/// symbol from its source.
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
    let resolved: ResolvedIndex =
        serde_json::from_slice(&output.stdout).context("invalid Griffe index")?;
    let mut sources = Sources::default();
    let mut entries = Vec::with_capacity(resolved.entries.len());
    for entry in resolved.entries {
        let mut members = Vec::with_capacity(entry.members.len());
        for member in entry.members {
            let name = format!("{}.{}", entry.name, member.name);
            let parameters = sources.parameters(&name, &member.definitions)?;
            members.push(Member {
                parameters: parameters.names,
                parameters_written: parameters.written,
                name: member.name,
                summary: member.summary,
                property: member.property,
            });
        }
        let parameters = sources.parameters(&entry.name, &entry.definitions)?;
        entries.push(Entry {
            parameters: parameters.names,
            parameters_written: parameters.written,
            kind: entry.kind,
            package: entry.package,
            name: entry.name,
            callable: entry.callable,
            summary: entry.summary,
            keywords: entry.keywords,
            members,
            rules: entry.rules,
            recipe: None,
        });
    }
    let mut index = Index {
        schema_version: INDEX_SCHEMA,
        package: resolved.package,
        version: resolved.version,
        entries,
        recipes: None,
    };
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
    /// The same library, which searches by meaning only when the embedding model is already
    /// on this machine. The hook searches on every prompt, and it must not wait for a download.
    pub(crate) fn without_download(self, project: &Path) -> Self {
        Self {
            model: OnceCell::from(Model::cached(project)),
            ..self
        }
    }

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
        ensure!(
            index.schema_version == INDEX_SCHEMA,
            "{} has index schema {}, and this xpd reads schema {INDEX_SCHEMA}. Build the index again with xpd index.",
            path.display(),
            index.schema_version
        );
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

impl Entry {
    /// Whether this is one of xpd's own recipes, which state the house style for a kind of
    /// task.
    pub(crate) fn is_playbook(&self) -> bool {
        (self.recipe.as_ref()).is_some_and(crate::recipe::Recipe::is_playbook)
    }

    /// The method or property `name` of this class, as an entry of its own.
    fn member(self, name: &str) -> Option<Self> {
        let member = self.members.into_iter().find(|m| m.name == name)?;
        let callable = if member.property {
            "property"
        } else {
            "method"
        };
        Some(Self {
            kind: Kind::Symbol,
            package: self.package,
            name: format!("{}.{}", self.name, member.name),
            callable: Some(callable.to_owned()),
            parameters: member.parameters,
            parameters_written: member.parameters_written,
            summary: member.summary,
            keywords: Vec::new(),
            members: Vec::new(),
            rules: Vec::new(),
            recipe: None,
        })
    }
}

/// The entry named `name`, or else the member that `name` names as `Class.member`.
pub(crate) fn api(library: &Library, name: &str) -> Result<Option<Entry>> {
    let entry = |name: &str| -> Result<Option<Entry>> {
        if let Some(entries) = &library.local {
            return Ok(entries.iter().find(|e| e.name == name).cloned());
        }
        RECORDS
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.entry().context("embedded entry"))
            .transpose()
    };
    if let Some(found) = entry(name)? {
        return Ok(Some(found));
    }
    let Some((class, member)) = name.rsplit_once('.') else {
        return Ok(None);
    };
    Ok(entry(class)?.and_then(|class| class.member(member)))
}

use crate::search::tokens;

#[derive(Debug)]
struct Stored {
    name: &'static str,
    package: &'static str,
    kind: Kind,
    summary: &'static str,
    /// The hash of the name and the summary, which build.rs computes.
    text: u64,
    start: u32,
    len: u16,
}
const DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/entries.bin"));
const STRINGS: &[&str] = include!(concat!(env!("OUT_DIR"), "/strings.rs"));

impl Stored {
    /// Whether the record is one of xpd's own recipes. build.rs gives no other record the
    /// kind and the package of a playbook.
    fn is_playbook(&self) -> bool {
        self.kind == Kind::Recipe && self.package == "xpd"
    }

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
/// `DIMENSIONS` signed bytes for each of `RECORDS`, all zero for a record that has no vector
/// of its current text.
const VECTORS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vectors.bin"));
/// The start of `indexes/vectors.bin`, which build.rs reads.
const VECTORS_HEADER: &[u8] = b"xpd vectors 256 with text hashes\n";
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

/// Sorts `(record, score)` pairs by score, best first. Between equal scores a playbook comes
/// before a recipe taken from a library, and then the name decides.
fn sort_ranking(ranked: &mut [(usize, f64)]) {
    ranked.sort_by(|(a, sa), (b, sb)| {
        let (a, b) = (&RECORDS[*a], &RECORDS[*b]);
        sb.total_cmp(sa)
            .then_with(|| b.is_playbook().cmp(&a.is_playbook()))
            .then_with(|| a.name.cmp(b.name))
    });
}

/// The playbooks, which are the recipes of xpd's own package.
pub(crate) fn playbooks() -> Result<Vec<Entry>> {
    (RECORDS.iter())
        .filter(|record| record.is_playbook())
        .map(|record| record.entry().context("embedded playbook"))
        .collect()
}

/// The lowest keyword score at which a playbook is taken as the answer to a prompt. The score
/// is the one that the keyword search ranks by, so the bar is the same with and without the
/// embedding model. Of the prompts tried on 2026-11-19, the ten loose asks scored 28.2 and
/// above for their playbook. Of thirty prompts about other work, none had a playbook as its
/// first recipe, and the best playbook of each scored 18.0 and below.
const PLAYBOOK_BAR: f64 = 24.0;

/// The playbook that answers `prompt`: the first recipe that `find` gives for it, when that is
/// a playbook whose keyword score clears [`PLAYBOOK_BAR`]. A prompt about other work shares
/// few words with the asks of a playbook, so it gets `None`.
pub(crate) fn playbook_for(library: &Library, prompt: &str) -> Option<Entry> {
    let first = (find(library, prompt, None, Some(Kind::Recipe), Some(1)).recipes)
        .pop()
        .filter(Entry::is_playbook)?;
    let (_, score) = *keyword_ranking(prompt, |record| record.name == first.name).first()?;
    (score >= PLAYBOOK_BAR).then_some(first)
}

/// The records that share a keyword with the query, best first.
fn keyword_ranking(query: &str, passes: impl Fn(&Stored) -> bool) -> Vec<(usize, f64)> {
    let words: BTreeSet<_> = tokens(query).into_iter().collect();
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
/// the vector that this build holds for it. build.rs drops the vector of a record whose text
/// changed, so only new and changed records are embedded, and a new rule costs one text. Each vector is scaled so its largest value is 127. Search
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
        data.extend(record.text.to_le_bytes());
        if vector.iter().any(|&v| v != 0) {
            data.extend(vector);
            continue;
        }
        let vector = embedded
            .next()
            .context("every missing record was embedded")?;
        let largest = vector.iter().fold(f32::EPSILON, |m, v| m.max(v.abs()));
        data.extend(vector.iter().map(|v| {
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
    let words: BTreeSet<_> = tokens(query).into_iter().collect();
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
    ranked.sort_by(|(a, sa), (b, sb)| {
        sb.total_cmp(sa)
            .then_with(|| b.is_playbook().cmp(&a.is_playbook()))
            .then_with(|| a.name.cmp(&b.name))
    });
    ranked.into_iter().map(|(e, _)| e).collect()
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
