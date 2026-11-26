//! The ledger that `xpd untangle` and `xpd optimize` print: the findings of a codebase by group
//! and kind, each with an id that it keeps at another commit, and the count of each kind, which
//! measures a round of work.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::hash::{Hash, Hasher as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
#[cfg(feature = "ledger-incremental")]
use std::sync::Arc;

use anyhow::{Context, Result};
use ast_grep_core::Language;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::Serialize;
use siphasher::sip::SipHasher13;

use crate::git;

#[cfg(feature = "ledger-durable")]
mod durable;
pub(crate) mod gix_source;
#[cfg(feature = "ledger-durable")]
mod orchestration;
mod run;
#[cfg(feature = "ledger-incremental")]
pub(crate) mod salsa_db;
pub(crate) mod twins;
#[cfg(feature = "ledger-distributed")]
pub(crate) mod worker;

use crate::output::Format;

/// How many places of a finding the text shows. The JSON holds every place.
const PLACES_SHOWN: usize = 3;

/// What a finding of one command is. The kinds of a command are in groups, and the ledger
/// lists the groups in the order of `GROUPS`.
pub(crate) trait Kind:
    Copy + Ord + Hash + std::fmt::Debug + Serialize + ValueEnum + Send + Sync + 'static
{
    type Group: Copy + Ord + std::fmt::Debug + Serialize + Send + Sync;

    const GROUPS: &'static [Self::Group];

    /// The name of the command that lists findings of this kind, such as `untangle`.
    const COMMAND: &'static str;

    fn group(self) -> Self::Group;

    /// The line that the text prints above the kinds of `group`.
    fn heading(group: Self::Group) -> &'static str;

    fn name(self) -> String {
        self.to_possible_value()
            .expect("no kind is skipped")
            .get_name()
            .to_owned()
    }
}

/// One place of a finding. Lines count from 1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct Place {
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) end_line: Option<usize>,
}

impl Place {
    pub(crate) fn file(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            line: None,
            end_line: None,
        }
    }

    pub(crate) fn line(path: &str, line: usize) -> Self {
        Self {
            line: Some(line),
            ..Self::file(path)
        }
    }

    pub(crate) fn lines(path: &str, (line, end_line): (usize, usize)) -> Self {
        Self {
            line: Some(line),
            end_line: Some(end_line),
            ..Self::file(path)
        }
    }

    fn text(&self) -> String {
        match (self.line, self.end_line) {
            (Some(line), Some(end)) => format!("{}:{line}-{end}", self.path),
            (Some(line), None) => format!("{}:{line}", self.path),
            _ => self.path.clone(),
        }
    }
}

/// One entry of the ledger.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Finding<K> {
    pub(crate) kind: K,
    /// What the finding is about, without a line or a count, so that the same finding has the
    /// same key at another commit.
    #[serde(skip)]
    pub(crate) key: String,
    pub(crate) places: Vec<Place>,
    /// What was measured, as one sentence.
    pub(crate) evidence: String,
    /// How strong the evidence is among findings of the same kind. The strongest come first.
    #[serde(skip)]
    pub(crate) weight: usize,
}

/// A finding as the ledger lists it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Entry<K> {
    /// The name of the finding, which it keeps at another commit as long as its files keep
    /// their paths. Of two findings that are alike in one file, the first in the file has the
    /// first id.
    pub(crate) id: String,
    /// The id that xpd 0.55.0 and older gave this finding, from [`legacy_id`]. A run that one
    /// of them recorded cites these ids, and its import rewrites each one to `id`.
    #[serde(skip)]
    #[cfg_attr(
        not(feature = "ledger-durable"),
        expect(dead_code, reason = "only the import of a JSON journal reads it")
    )]
    pub(crate) legacy_id: String,
    /// With `--since`: whether the commit had no finding of this id.
    #[serde(rename = "new", skip_serializing_if = "Option::is_none")]
    fresh: Option<bool>,
    #[serde(flatten)]
    pub(crate) finding: Finding<K>,
}

impl<K> Entry<K> {
    /// The finding on one line: its id, `new` when the commit of `--since` did not have it,
    /// its first places and its evidence.
    fn text(&self) -> String {
        let finding = &self.finding;
        let mut places: Vec<_> =
            (finding.places.iter().take(PLACES_SHOWN).map(Place::text)).collect();
        if finding.places.len() > PLACES_SHOWN {
            places.push(format!("{} more", finding.places.len() - PLACES_SHOWN));
        }
        let fresh = if self.fresh == Some(true) { " new" } else { "" };
        format!(
            "{}{fresh}  {}  {}",
            self.id,
            places.join(", "),
            finding.evidence
        )
    }
}

/// The findings by group, kind and strength, each with its id.
pub(crate) fn named<K: Kind>(mut findings: Vec<Finding<K>>) -> Vec<Entry<K>> {
    findings.sort_by(|a, b| {
        (
            a.kind.group(),
            a.kind,
            std::cmp::Reverse(a.weight),
            &a.places,
        )
            .cmp(&(
                b.kind.group(),
                b.kind,
                std::cmp::Reverse(b.weight),
                &b.places,
            ))
    });
    let mut alike: HashMap<(K, String), usize> = HashMap::new();
    (findings.into_iter())
        .map(|finding| {
            let nth = alike
                .entry((finding.kind, finding.key.clone()))
                .or_default();
            *nth += 1;
            Entry {
                id: id(&finding.kind.name(), &finding.key, *nth),
                legacy_id: legacy_id(&finding.kind.name(), &finding.key, *nth),
                fresh: None,
                finding,
            }
        })
        .collect()
}

/// The key of the id hash. It is fixed, so that every build gives a finding the same id.
const ID_KEY: [u8; 16] = *b"xpd ledger id v1";

/// The id of the `nth` finding of `kind` with `key`: SipHash-1-3 under `ID_KEY`, cut to 32 bits.
/// Each part goes in as its length and its bytes, in little-endian order, so that the id is the
/// same on every target and does not depend on how `Hash` writes a value.
fn id(kind: &str, key: &str, nth: usize) -> String {
    let mut hasher = SipHasher13::new_with_key(&ID_KEY);
    for part in [kind.as_bytes(), key.as_bytes()] {
        hasher.write(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
        hasher.write(part);
    }
    hasher.write(&u64::try_from(nth).unwrap_or(u64::MAX).to_le_bytes());
    format!("{:08x}", hasher.finish() & 0xffff_ffff)
}

/// The id that xpd 0.55.0 and older gave the `nth` finding of `kind` with `key`: the standard
/// library's `DefaultHasher` over the three, cut to 32 bits. The standard library may change
/// that hash in another Rust release, so only the import of an old journal uses it.
fn legacy_id(kind: &str, key: &str, nth: usize) -> String {
    let mut hasher = std::hash::DefaultHasher::new();
    (kind, key, nth).hash(&mut hasher);
    format!("{:08x}", hasher.finish() & 0xffff_ffff)
}

/// A read-only view of the files of a repository at a commit, or of its working tree.
/// `GixSource` implements it over the git object database, and `FsSource` over the files on
/// disk. Paths are relative to the top of the repository, and object ids are full hex ids in
/// the object format of the repository.
#[cfg(feature = "ledger-incremental")]
pub(crate) trait ContentSource: std::fmt::Debug + Send + Sync {
    /// The full id of the commit that `rev`, such as `HEAD`, `main` or a hash, names.
    #[expect(dead_code, reason = "the surveys that call it come in the next slice")]
    fn resolve_commit(&self, rev: &str) -> Result<String>;

    /// The code files under `paths` at `commit`, or in the working tree when it is `None`,
    /// each as its path, the id of its blob and its language.
    fn list_code_files(
        &self,
        paths: &[PathBuf],
        commit: Option<&str>,
    ) -> Result<Vec<(String, String, SupportLang)>>;

    /// The text of the blob `blob_oid`, which is the file at `rel_path`.
    fn read_blob(&self, rel_path: &str, blob_oid: &str) -> Result<Arc<str>>;

    /// The paths that each of the last `max_commits` commits without a merge changed, from
    /// `commit` back, newest first.
    #[expect(dead_code, reason = "the surveys that call it come in the next slice")]
    fn commit_history(&self, commit: &str, max_commits: usize) -> Result<Vec<Vec<String>>>;
}

/// How many findings of one kind there are.
#[derive(Debug, Serialize)]
struct KindCount<K> {
    kind: K,
    count: usize,
    /// The count at the commit of `--since`.
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(bound(serialize = "K: Kind"))]
struct GroupCount<K: Kind> {
    group: K::Group,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    kinds: Vec<KindCount<K>>,
}

#[derive(Debug, Serialize)]
#[serde(bound(serialize = "K: Kind"))]
pub(crate) struct Summary<K: Kind> {
    pub(crate) count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    was: Option<usize>,
    /// With `--since`: the findings of today that the commit did not have.
    #[serde(rename = "new", skip_serializing_if = "Option::is_none")]
    fresh: Option<usize>,
    /// With `--since`: the findings of the commit that are gone today.
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed: Option<usize>,
    groups: Vec<GroupCount<K>>,
}

/// The count of each kind and group, with the counts of `before` beside them.
fn summary<K: Kind>(findings: &[Entry<K>], before: Option<&[Entry<K>]>) -> Summary<K> {
    let count = |findings: &[Entry<K>], kind: K| {
        findings
            .iter()
            .filter(|entry| entry.finding.kind == kind)
            .count()
    };
    let groups: Vec<GroupCount<K>> = (K::GROUPS.iter())
        .map(|&group| {
            let kinds: Vec<KindCount<K>> = (K::value_variants().iter())
                .filter(|kind| kind.group() == group)
                .map(|&kind| KindCount {
                    kind,
                    count: count(findings, kind),
                    was: before.map(|before| count(before, kind)),
                })
                .collect();
            GroupCount {
                group,
                count: kinds.iter().map(|kind| kind.count).sum(),
                was: before.map(|_| kinds.iter().filter_map(|kind| kind.was).sum()),
                kinds,
            }
        })
        .collect();
    Summary {
        count: findings.len(),
        was: before.map(<[Entry<K>]>::len),
        fresh: None,
        fixed: None,
        groups,
    }
}

/// The findings a run lists: the kinds that were asked for, and the most of each kind.
#[derive(Debug, Clone, Copy)]
struct Listed<'k, K> {
    /// Every kind when empty.
    kinds: &'k [K],
    /// 0 lists every finding.
    limit: usize,
}

impl<K: Kind> Listed<'_, K> {
    fn selects(&self, kind: K) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&kind)
    }

    fn of(&self, findings: Vec<Entry<K>>) -> Vec<Entry<K>> {
        let mut listed: HashMap<K, usize> = HashMap::new();
        (findings.into_iter())
            .filter(|entry| self.selects(entry.finding.kind))
            .filter(|entry| {
                let count = listed.entry(entry.finding.kind).or_default();
                *count += 1;
                self.limit == 0 || *count <= self.limit
            })
            .collect()
    }
}

/// The arguments of a command of the ledger.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Select<K: Kind> {
    /// A stage of a refactor run, which xpd records in the git folder of the repository.
    #[command(subcommand)]
    action: Option<run::Action>,
    /// Files or directories (directories respect .gitignore).
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// List only the findings of these kinds, separated by commas.
    #[arg(long, value_enum, value_delimiter = ',')]
    kind: Vec<K>,
    /// The most findings to list of each kind, strongest first. 0 lists every finding.
    #[arg(long, default_value_t = 20)]
    limit: usize,
    /// Count the findings at this commit too, and print each count beside the count of today.
    #[arg(long, value_name = "COMMIT")]
    since: Option<String>,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum, global = true)]
    format: Option<Format>,
}

impl<K: Kind> Select<K> {
    /// Prints the ledger of the paths, or runs a stage of a refactor run. `survey` reads the files under the paths it gets, which
    /// are in the folders of the tree, and returns what it read and the findings. It also gets
    /// the top of the repository and the commit that the files are from, which is `HEAD` for
    /// the files of today. `header` writes the first lines of the text.
    pub(crate) fn print<R: Serialize>(
        self,
        default: Format,
        survey: impl Fn(&[PathBuf], Tree<'_>, Option<&Path>, &str) -> Result<(R, Vec<Entry<K>>)>,
        header: impl FnOnce(&Ledger<K, R>) -> String,
    ) -> Result<ExitCode> {
        if let Some(action) = self.action {
            return run::act(action, self.format.unwrap_or(default), survey);
        }
        let top = repository(&self.paths[0]);
        let files = Tree {
            top: top.as_deref(),
            shown_from: None,
        };
        let (read, findings) = survey(&self.paths, files, top.as_deref(), "HEAD")?;
        let before = match &self.since {
            Some(commit) => {
                let top = top.as_deref().with_context(|| {
                    format!(
                        "{} is in no git repository, which --since needs",
                        self.paths[0].display()
                    )
                })?;
                let then = at_commit(&self.paths, top, commit, |then, files, resolved| {
                    Ok(survey(then, files, Some(top), resolved)?.1)
                })?;
                Some((commit.clone(), then.unwrap_or_default()))
            }
            None => None,
        };
        let listed = Listed {
            kinds: &self.kind,
            limit: self.limit,
        };
        let ledger = Ledger::new(read, findings, before, listed);
        match self.format.unwrap_or(default) {
            Format::Json => println!("{}", serde_json::to_string(&ledger)?),
            Format::Text => print!("{}", ledger.text(&header(&ledger), listed)),
        }
        Ok(ExitCode::SUCCESS)
    }
}

/// What a command of the ledger prints. `R` says what the run read.
#[derive(Debug, Serialize)]
#[serde(bound(serialize = "K: Kind, R: Serialize"))]
pub(crate) struct Ledger<K: Kind, R> {
    pub(crate) read: R,
    /// The commit that `was` counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
    pub(crate) summary: Summary<K>,
    /// The strongest findings of each kind, up to the limit.
    findings: Vec<Entry<K>>,
    /// With `--since`: the findings of the commit that are gone today, up to the limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed: Option<Vec<Entry<K>>>,
}

impl<K: Kind, R> Ledger<K, R> {
    /// The ledger of `findings`. `since` holds the commit of `--since` and the findings the
    /// files had at it.
    fn new(
        read: R,
        mut findings: Vec<Entry<K>>,
        since: Option<(String, Vec<Entry<K>>)>,
        listed: Listed<'_, K>,
    ) -> Self {
        let (since, before) = since.unzip();
        let mut fixed = None;
        if let Some(before) = &before {
            let had: HashSet<&str> = before.iter().map(|entry| entry.id.as_str()).collect();
            for entry in &mut findings {
                entry.fresh = Some(!had.contains(entry.id.as_str()));
            }
            let has: HashSet<&str> = findings.iter().map(|entry| entry.id.as_str()).collect();
            fixed = Some(
                (before.iter())
                    .filter(|entry| !has.contains(entry.id.as_str()))
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }
        let mut summary = summary(&findings, before.as_deref());
        summary.fresh = (before.is_some()).then(|| {
            findings
                .iter()
                .filter(|entry| entry.fresh == Some(true))
                .count()
        });
        summary.fixed = fixed.as_ref().map(Vec::len);
        Self {
            read,
            since,
            summary,
            findings: listed.of(findings),
            fixed: fixed.map(|fixed| listed.of(fixed)),
        }
    }

    /// The count of today with the count, the fixed and the new findings of `--since` behind
    /// it, for the first line of the text.
    pub(crate) fn count(&self) -> String {
        let summary = &self.summary;
        match (summary.was, summary.fixed, summary.fresh) {
            (Some(was), Some(fixed), Some(fresh)) => {
                format!(
                    "{} findings (was {was}, {fixed} fixed, {fresh} new)",
                    summary.count
                )
            }
            _ => format!("{} findings", summary.count),
        }
    }

    /// The ledger as text, below the lines of `header`.
    fn text(&self, header: &str, listed: Listed<'_, K>) -> String {
        let mut text = header.to_owned();
        let was = |was: Option<usize>| was.map_or_else(String::new, |was| format!(" (was {was})"));
        for group in &self.summary.groups {
            let _ = writeln!(
                text,
                "\n{}: {}{}",
                K::heading(group.group),
                group.count,
                was(group.was)
            );
            for kind in &group.kinds {
                if kind.count == 0 && kind.was.unwrap_or(0) == 0 {
                    continue;
                }
                let _ = writeln!(
                    text,
                    "  {}: {}{}",
                    kind.kind.name(),
                    kind.count,
                    was(kind.was)
                );
                let shown: Vec<_> = (self.findings.iter())
                    .filter(|entry| entry.finding.kind == kind.kind)
                    .collect();
                for entry in &shown {
                    let _ = writeln!(text, "    {}", entry.text());
                }
                if listed.selects(kind.kind) && shown.len() < kind.count {
                    let _ = writeln!(
                        text,
                        "    and {} more with --kind {} --limit 0",
                        kind.count - shown.len(),
                        kind.kind.name()
                    );
                }
            }
        }
        if let (Some(fixed), Some(commit)) = (&self.fixed, &self.since) {
            let _ = writeln!(
                text,
                "\nFixed since {commit}: {}",
                self.summary.fixed.unwrap_or(0)
            );
            for entry in fixed {
                let _ = writeln!(text, "  {}  {}", entry.finding.kind.name(), entry.text());
            }
        }
        text
    }
}

/// The top folder of the git repository that holds `path`. `None` outside a repository.
fn repository(path: &Path) -> Option<PathBuf> {
    let folder = if path.is_dir() {
        path
    } else {
        path.parent()
            .filter(|folder| !folder.as_os_str().is_empty())?
    };
    let top = git::output(folder, &["rev-parse", "--show-toplevel"]).ok()?;
    fs::canonicalize(top.trim_end_matches(['\r', '\n'])).ok()
}

/// The folders that the files of a survey are in.
#[derive(Clone, Copy)]
pub(crate) struct Tree<'p> {
    /// The folder that the paths of the history start from. `None` outside a repository.
    pub(crate) top: Option<&'p Path>,
    /// The folder that the printed paths start from, for the copy of a commit, whose files are
    /// in a temporary folder. `None` prints each path as it was given.
    pub(crate) shown_from: Option<&'p Path>,
}

impl Tree<'_> {
    /// The path of the file at `path` below the top of the repository, as git writes it.
    pub(crate) fn tracked(&self, absolute: &Path) -> Option<String> {
        self.top
            .and_then(|top| absolute.strip_prefix(top).ok())
            .map(|below| below.to_string_lossy().replace('\\', "/"))
    }

    /// The path as the ledger prints it, without the `./` of a walk that starts in this folder.
    pub(crate) fn shown(&self, path: &Path) -> String {
        let below = (self.shown_from).and_then(|from| path.strip_prefix(from).ok());
        let path = below.unwrap_or(path);
        path.strip_prefix(".").unwrap_or(path).display().to_string()
    }
}

/// A folder that is removed when the value is dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What `survey` returns for the files under `paths` as they were at `commit`. `survey` gets
/// the paths in a copy of the commit, the folders of that copy and the full hash of the commit.
/// `None` when the commit holds none of the paths. The files of the commit are written to a
/// folder of their own through an index of their own, so the working tree and the index of the
/// repository stay as they are.
fn at_commit<T>(
    paths: &[PathBuf],
    top: &Path,
    commit: &str,
    survey: impl FnOnce(&[PathBuf], Tree<'_>, &str) -> Result<T>,
) -> Result<Option<T>> {
    let resolved = git::output(
        top,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )
    .with_context(|| format!("{commit} names no commit"))?;
    let resolved = resolved.trim();
    let scratch = Scratch(std::env::temp_dir().join(format!("xpd-ledger-{}", std::process::id())));
    let tree = scratch.0.join("tree");
    fs::create_dir_all(&tree).with_context(|| format!("cannot create {}", tree.display()))?;
    let index = scratch.0.join("index");
    let env: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", index.as_os_str())];
    git::output_with(top, &["read-tree", resolved], &env)?;
    let prefix = format!("--prefix={}/", tree.display());
    git::output_with(top, &["checkout-index", "--all", &prefix], &env)?;
    let then: Vec<PathBuf> = (paths.iter())
        .filter_map(|path| {
            let below = fs::canonicalize(path).ok()?;
            Some(tree.join(below.strip_prefix(top).ok()?))
        })
        .filter(|path| path.exists())
        .collect();
    if then.is_empty() {
        return Ok(None);
    }
    let here = std::env::current_dir().and_then(fs::canonicalize).ok();
    let shown_from = (here.as_deref())
        .and_then(|here| here.strip_prefix(top).ok())
        .map_or_else(|| tree.clone(), |below| tree.join(below));
    let files = Tree {
        top: Some(&tree),
        shown_from: Some(&shown_from),
    };
    survey(&then, files, resolved).map(Some)
}

/// A language whose files hold code. A data or markup file repeats its structure by nature, so
/// its copies and its changes say nothing about the design.
fn code(path: &Path) -> Option<SupportLang> {
    SupportLang::from_path(path).filter(|language| {
        !matches!(
            language,
            SupportLang::Json
                | SupportLang::Yaml
                | SupportLang::Html
                | SupportLang::Css
                | SupportLang::Markdown
        )
    })
}

/// The files of code under `paths`, in path order. A folder that cannot be read is named on
/// stderr, and the rest of the project is still read.
pub(crate) fn files(paths: &[PathBuf]) -> Vec<(PathBuf, SupportLang)> {
    let (first, rest) = paths.split_first().expect("clap gives a default path");
    let mut walk = ignore::WalkBuilder::new(first);
    for path in rest {
        walk.add(path);
    }
    let mut files = Vec::new();
    for entry in walk.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("xpd: {error}");
                continue;
            }
        };
        if entry.file_type().is_some_and(|kind| kind.is_file())
            && let Some(language) = code(entry.path())
        {
            files.push((entry.into_path(), language));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    files
}
