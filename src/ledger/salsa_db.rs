//! Incremental queries for the ledger surveys, on `salsa` 0.28.6.
//!
//! File text, the per-file facts of `xpd optimize` and `xpd untangle`, and the changed files
//! of a commit are tracked queries over captured inputs. A caller captures the text of a file
//! and the diff of a commit before it asks, so every query stays a pure function of its
//! inputs. The blob id in a file key content-addresses the bytes of a commit, and the hash of
//! the captured text does the same for a dirty working tree, so a key can never stand for two
//! texts. Only compact summaries are memoized. No syntax tree is kept.
//!
//! The rules and the library index change seldom, so their input carries
//! `salsa::Durability::HIGH` and a change of code files never revises it. A snapshot of code
//! files carries `salsa::Durability::LOW`, so a new snapshot leaves the rules and the index
//! alone. Each tracked query caps how many of its values stay in memory, and evicted values
//! leave only their dependency record behind.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use ast_grep_language::SupportLang;
use salsa::Durability;
use sha2::{Digest as _, Sha256};

use super::twins;
use crate::syntax;
use crate::untangle::copies::{self, UNTANGLE_FACTS_LRU, UntangleFileSummary};

/// How many file texts stay in memory at once.
pub(crate) const FILE_TEXT_LRU: usize = 4096;
/// How many per-file facts of `xpd optimize` stay in memory at once.
pub(crate) const OPTIMIZE_FACTS_LRU: usize = 2048;
/// How many commit diffs stay in memory at once. The history window reads 2000 commits.
pub(crate) const COMMIT_DIFF_LRU: usize = 2048;

/// One file of a snapshot, as the caller captured it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FileEntry {
    /// The path below the top of the repository.
    pub(crate) rel_path: String,
    /// The id of the blob at a commit, or the hash of the captured text of the working tree.
    pub(crate) blob_oid: String,
    /// The language of the file.
    pub(crate) lang: SupportLang,
}

/// The files of one commit or working tree, and the fingerprint of the rules that read them.
///
/// Created through [`new_snapshot_input`], which gives every field `Durability::LOW`.
#[salsa::input]
pub(crate) struct SnapshotInput {
    /// The full id of the commit, or the empty text for the working tree.
    #[returns(deref)]
    pub(crate) commit_oid: String,
    /// Each file of the snapshot in path order, without repeats.
    pub(crate) files: Vec<FileEntry>,
    /// The hash of the project rules and style that the survey reads.
    #[returns(deref)]
    pub(crate) rules_fingerprint: String,
}

/// The rules and the library index that the surveys read, which change seldom.
///
/// Created through [`new_rules_input`], which gives every field `Durability::HIGH`, so that a change of
/// code files never revises the rule catalog or the index.
#[salsa::input]
pub(crate) struct RulesInput {
    /// The hash of the rule catalog and the project rules.
    #[returns(deref)]
    pub(crate) fingerprint: String,
    /// The hash of the bundled library index that holds the defaults of library calls.
    #[returns(deref)]
    pub(crate) index_fingerprint: String,
}

/// The text of one file, captured by the caller before the survey runs.
#[salsa::input]
pub(crate) struct SourceText {
    /// The path below the top of the repository.
    #[returns(deref)]
    pub(crate) rel_path: String,
    /// The id of the blob at a commit, or the hash of the text of the working tree.
    #[returns(deref)]
    pub(crate) blob_oid: String,
    /// The language of the file.
    #[returns(copy)]
    pub(crate) lang: SupportLang,
    /// The text itself, as read from the object database or from disk.
    #[returns(deref)]
    pub(crate) text: String,
}

/// The changed files of one commit, captured by the caller from a tree diff.
#[salsa::input]
pub(crate) struct CommitDiff {
    /// The full id of the commit.
    #[returns(deref)]
    pub(crate) commit_oid: String,
    /// The paths the commit changed below the top of the repository, in path order.
    pub(crate) changed_files: Vec<String>,
}

/// A file by the path and the content that its key names.
#[salsa::interned]
pub(crate) struct FileKey<'db> {
    /// The path below the top of the repository.
    #[returns(deref)]
    pub(crate) rel_path: String,
    /// The id of the blob at a commit, or the hash of the text of the working tree.
    #[returns(deref)]
    pub(crate) blob_oid: String,
    /// The language of the file.
    #[returns(copy)]
    pub(crate) lang: SupportLang,
}

/// A commit by its full id. Commits do not change, so one id always names one diff.
#[salsa::interned]
pub(crate) struct CommitId<'db> {
    /// The full id of the commit.
    #[returns(deref)]
    pub(crate) oid: String,
}

/// The rules that read a file, by the fingerprint of the catalog and the index.
#[salsa::interned]
pub(crate) struct RulesKey<'db> {
    /// The hash of the rule catalog, the project rules and the library index.
    #[returns(deref)]
    pub(crate) fingerprint: String,
}

/// The per-file facts of `xpd optimize` for one file: its lines and its near-copy stretches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptimizeFileSummary {
    /// The lines of the file.
    pub(crate) lines: usize,
    /// The stretches of the file that are long enough to be a near copy.
    pub(crate) shapes: twins::Shapes,
}

/// The per-file facts of `xpd untangle` for one file are the tracked query in
/// `untangle::copies`, beside the stretches it reads.

/// The text of a file. The caller captured the text in `source`, and the blob id or text hash
/// in the same input names that text, so this query reads no file and no object database.
#[salsa::tracked(lru = 4096, returns(ref))]
pub(crate) fn file_text(db: &dyn salsa::Database, source: SourceText) -> Arc<str> {
    Arc::from(source.text(db))
}

/// The facts of one file for `xpd optimize`, under the rules that `rules` names. The rules key
/// takes part so that a change of rules or of the library index reads the file again, while a
/// change of another file does not.
#[salsa::tracked(lru = 2048, returns(ref))]
pub(crate) fn optimize_file_facts(
    db: &dyn salsa::Database,
    source: SourceText,
    rules: RulesKey<'_>,
) -> Arc<OptimizeFileSummary> {
    let _ = rules.fingerprint(db);
    let text = file_text(db, source);
    let root = syntax::parse_as(text.as_ref(), source.lang(db));
    Arc::new(OptimizeFileSummary {
        lines: text.lines().count(),
        shapes: twins::shapes(&root.root()),
    })
}

/// The paths that the commit `commit` changed. The caller captured the diff in `diff` from a
/// tree-to-tree diff, so this query walks no history and reads no object database. A commit
/// never changes, so its diff stays right for as long as it stays in memory.
#[salsa::tracked(lru = 2048, returns(ref))]
pub(crate) fn commit_changed_files(
    db: &dyn salsa::Database,
    commit: CommitId<'_>,
    diff: CommitDiff,
) -> Arc<[String]> {
    debug_assert_eq!(
        commit.oid(db),
        diff.commit_oid(db),
        "a commit key and its diff name two commits"
    );
    Arc::from(diff.changed_files(db).clone())
}

/// A database with the memory caps of this module set on each tracked query. The caps that
/// the `lru` settings declare already hold at creation, and this sets them again through the
/// per-query setters so that a caller can lower them at runtime on the same database.
pub(crate) fn database() -> salsa::DatabaseImpl {
    let mut db = salsa::DatabaseImpl::new();
    file_text::set_lru_capacity(&mut db, FILE_TEXT_LRU);
    optimize_file_facts::set_lru_capacity(&mut db, OPTIMIZE_FACTS_LRU);
    copies::set_untangle_lru(&mut db, UNTANGLE_FACTS_LRU);
    commit_changed_files::set_lru_capacity(&mut db, COMMIT_DIFF_LRU);
    db
}

/// The snapshot of `files` at `commit_oid` under rules of `rules_fingerprint`, with
/// `Durability::LOW` on every field, so that it never revises the rules or the index.
#[expect(dead_code, reason = "reserved for multi-snapshot workspace queries")]
pub(crate) fn new_snapshot_input(
    db: &dyn salsa::Database,
    commit_oid: String,
    files: Vec<FileEntry>,
    rules_fingerprint: String,
) -> SnapshotInput {
    SnapshotInput::builder(commit_oid, files, rules_fingerprint)
        .commit_oid_durability(Durability::LOW)
        .files_durability(Durability::LOW)
        .rules_fingerprint_durability(Durability::LOW)
        .new(db)
}

/// The rules and index input with `Durability::HIGH` on every field, so that a change of code
/// files never revises the rule catalog or the library index.
pub(crate) fn new_rules_input(
    db: &dyn salsa::Database,
    fingerprint: String,
    index_fingerprint: String,
) -> RulesInput {
    RulesInput::builder(fingerprint, index_fingerprint)
        .fingerprint_durability(Durability::HIGH)
        .index_fingerprint_durability(Durability::HIGH)
        .new(db)
}

/// The key of the file that `entry` names in a snapshot.
#[expect(dead_code, reason = "reserved for multi-snapshot workspace queries")]
pub(crate) fn file_key<'db>(db: &'db dyn salsa::Database, entry: &FileEntry) -> FileKey<'db> {
    FileKey::new(
        db,
        entry.rel_path.clone(),
        entry.blob_oid.clone(),
        entry.lang,
    )
}

/// The key of the rules that `input` names.
pub(crate) fn rules_key(db: &dyn salsa::Database, input: RulesInput) -> RulesKey<'_> {
    RulesKey::new(
        db,
        format!("{}\n{}", input.fingerprint(db), input.index_fingerprint(db)),
    )
}

struct SurveyCache {
    db: salsa::DatabaseImpl,
    sources: HashMap<(String, String, SupportLang), SourceText>,
    rules: HashMap<String, RulesInput>,
    commits: HashMap<String, CommitDiff>,
}

impl SurveyCache {
    fn new() -> Self {
        Self {
            db: database(),
            sources: HashMap::new(),
            rules: HashMap::new(),
            commits: HashMap::new(),
        }
    }

    fn source(&mut self, rel_path: &str, text: &str, lang: SupportLang) -> SourceText {
        let hash = content_hash(text);
        let key = (rel_path.to_owned(), hash.clone(), lang);
        if let Some(&existing) = self.sources.get(&key) {
            return existing;
        }
        let created = SourceText::new(&self.db, rel_path.to_owned(), hash, lang, text.to_owned());
        self.sources.insert(key, created);
        created
    }

    fn rules(&mut self, fingerprint: &str) -> RulesInput {
        if let Some(&existing) = self.rules.get(fingerprint) {
            return existing;
        }
        let created = new_rules_input(&self.db, fingerprint.to_owned(), "bundled".to_owned());
        self.rules.insert(fingerprint.to_owned(), created);
        created
    }
}

thread_local! {
    static SURVEY_CACHE: RefCell<SurveyCache> = RefCell::new(SurveyCache::new());
}

fn content_hash(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Returns memoized `xpd untangle` per-file stretches and shapes via a thread-local Salsa
/// database, keyed by path, content hash, and language without cross-thread lock contention.
pub(crate) fn cached_untangle_facts(
    rel_path: &str,
    text: &str,
    lang: SupportLang,
) -> Arc<UntangleFileSummary> {
    SURVEY_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let source = cache.source(rel_path, text, lang);
        Arc::clone(copies::untangle_file_facts(&cache.db, source))
    })
}

/// Returns memoized `xpd optimize` per-file line counts and shape stretches via Salsa.
pub(crate) fn cached_optimize_facts(
    rel_path: &str,
    text: &str,
    lang: SupportLang,
    rules_fingerprint: &str,
) -> Arc<OptimizeFileSummary> {
    SURVEY_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let source = cache.source(rel_path, text, lang);
        let rules_input = cache.rules(rules_fingerprint);
        let key = rules_key(&cache.db, rules_input);
        Arc::clone(optimize_file_facts(&cache.db, source, key))
    })
}

/// Returns memoized changed files for a commit pair key via Salsa's `commit_changed_files`.
pub(crate) fn cached_commit_diff(
    commit_key: &str,
    load: impl FnOnce() -> anyhow::Result<Vec<String>>,
) -> anyhow::Result<Arc<[String]>> {
    SURVEY_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let diff = if let Some(&existing) = cache.commits.get(commit_key) {
            existing
        } else {
            let changed = load()?;
            let created = CommitDiff::new(&cache.db, commit_key.to_owned(), changed);
            cache.commits.insert(commit_key.to_owned(), created);
            created
        };
        let id = CommitId::new(&cache.db, commit_key.to_owned());
        Ok(Arc::clone(commit_changed_files(&cache.db, id, diff)))
    })
}
