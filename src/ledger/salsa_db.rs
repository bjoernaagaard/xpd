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

use std::sync::Arc;

use ast_grep_language::SupportLang;
use salsa::Durability;

use super::twins;
use crate::syntax;

/// How many file texts stay in memory at once.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) const FILE_TEXT_LRU: usize = 4096;
/// How many per-file facts of `xpd optimize` stay in memory at once.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) const OPTIMIZE_FACTS_LRU: usize = 2048;
/// How many commit diffs stay in memory at once. The history window reads 2000 commits.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
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
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) fn database() -> salsa::DatabaseImpl {
    let mut db = salsa::DatabaseImpl::new();
    file_text::set_lru_capacity(&mut db, FILE_TEXT_LRU);
    optimize_file_facts::set_lru_capacity(&mut db, OPTIMIZE_FACTS_LRU);
    commit_changed_files::set_lru_capacity(&mut db, COMMIT_DIFF_LRU);
    db
}

/// The snapshot of `files` at `commit_oid` under rules of `rules_fingerprint`, with
/// `Durability::LOW` on every field, so that it never revises the rules or the index.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
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
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
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
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) fn file_key<'db>(db: &'db dyn salsa::Database, entry: &FileEntry) -> FileKey<'db> {
    FileKey::new(
        db,
        entry.rel_path.clone(),
        entry.blob_oid.clone(),
        entry.lang,
    )
}

/// The key of the rules that `input` names.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) fn rules_key(db: &dyn salsa::Database, input: RulesInput) -> RulesKey<'_> {
    RulesKey::new(
        db,
        format!("{}\n{}", input.fingerprint(db), input.index_fingerprint(db)),
    )
}
