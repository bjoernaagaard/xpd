//! The files that a survey of the ledger reads: the files of today on disk, or the files of a
//! commit, which `GixSource` reads from the git object database without writing them anywhere.
//!
//! The files of a commit are listed the way `ledger::files` lists them in a copy of the commit
//! that has no git folder: regular files of code below each path, without hidden entries and
//! without what `.ignore` files leave out, sorted and without repeats. A path that names a file
//! is kept as it is, as the walk keeps a file it is given.

#[cfg(feature = "ledger-incremental")]
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use ast_grep_language::SupportLang;

use crate::ledger::{self, Tree};

/// The code files that one survey reads, and where their text comes from.
#[derive(Debug)]
pub(crate) struct Listing {
    /// Each file in path order, with its language. The path of a file of a commit is the path
    /// it has in the copy of the commit that the tree of the survey names.
    pub(crate) files: Vec<(PathBuf, SupportLang)>,
    origin: Origin,
}

#[derive(Debug)]
enum Origin {
    Disk,
    /// The files of a commit, each with its path below the top of the repository and the id of
    /// its blob.
    #[cfg(feature = "ledger-incremental")]
    Commit {
        source: GixSource,
        blobs: HashMap<PathBuf, (String, String)>,
    },
}

impl Listing {
    /// The files under `paths`, which are in the folders of `tree`. `top` is the top of the
    /// repository and `commit` the commit the files are from, which is `HEAD` for the files of
    /// today. The files of another commit are read from the object database of `top`.
    #[cfg_attr(
        not(feature = "ledger-incremental"),
        expect(
            clippy::unnecessary_wraps,
            reason = "reading a commit, which only the ledger-incremental build does, can fail"
        )
    )]
    pub(crate) fn of(
        paths: &[PathBuf],
        tree: Tree<'_>,
        top: Option<&Path>,
        commit: &str,
    ) -> Result<Self> {
        #[cfg(feature = "ledger-incremental")]
        if commit != "HEAD"
            && let (Some(top), Some(copy)) = (top, tree.top)
        {
            return Self::at_commit(paths, copy, top, commit);
        }
        #[cfg(not(feature = "ledger-incremental"))]
        let _ = (tree, top, commit);
        Ok(Self {
            files: ledger::files(paths),
            origin: Origin::Disk,
        })
    }

    /// The files of `commit` under `paths`, which are in `copy`, the folder that stands for the
    /// top of the repository at `top`.
    #[cfg(feature = "ledger-incremental")]
    fn at_commit(paths: &[PathBuf], copy: &Path, top: &Path, commit: &str) -> Result<Self> {
        use crate::ledger::ContentSource as _;

        let source = GixSource::open(top);
        let roots: Vec<PathBuf> = (paths.iter())
            .filter_map(|path| path.strip_prefix(copy).ok().map(Path::to_path_buf))
            .collect();
        let mut files = Vec::new();
        let mut blobs = HashMap::new();
        for (below, blob, language) in source.list_code_files(&roots, Some(commit))? {
            let path = copy.join(&below);
            files.push((path.clone(), language));
            blobs.insert(path, (below, blob));
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files.dedup_by(|a, b| a.0 == b.0);
        Ok(Self {
            files,
            origin: Origin::Commit { source, blobs },
        })
    }

    /// The text of the file at `path`. `None` for a file that cannot be read or is not UTF-8.
    pub(crate) fn text(&self, path: &Path) -> Option<Arc<str>> {
        match &self.origin {
            Origin::Disk => fs::read_to_string(path).ok().map(Arc::from),
            #[cfg(feature = "ledger-incremental")]
            Origin::Commit { source, blobs } => {
                use crate::ledger::ContentSource as _;

                let (below, blob) = blobs.get(path)?;
                source.read_blob(below, blob).ok()
            }
        }
    }

    /// The absolute path of the file at `path`, which the tree of the survey names below its
    /// top. A file of a commit keeps the path it has in the copy of the commit.
    pub(crate) fn absolute(&self, path: &Path) -> Option<PathBuf> {
        match &self.origin {
            Origin::Disk => fs::canonicalize(path).ok(),
            #[cfg(feature = "ledger-incremental")]
            Origin::Commit { blobs, .. } => blobs.contains_key(path).then(|| path.to_path_buf()),
        }
    }
}

#[cfg(feature = "ledger-incremental")]
pub(crate) use commit::GixSource;

#[cfg(feature = "ledger-incremental")]
mod commit {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashMap};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use anyhow::{Context as _, Result, bail};
    use ast_grep_core::Language as _;
    use ast_grep_language::SupportLang;
    use gix::ObjectId;
    use gix::objs::tree::EntryKind;
    use gix::revision::walk::Sorting;
    use gix::traverse::commit::simple::CommitTimeOrder;
    use ignore::Match;
    use ignore::gitignore::{Gitignore, GitignoreBuilder};

    use crate::ledger::ContentSource;

    /// The name of the files whose patterns leave files out of the walk.
    const IGNORE: &str = ".ignore";

    thread_local! {
        /// The repository that this thread opened last, by the folder it was opened at. A
        /// `gix::Repository` without the `parallel` feature of gix cannot move to another
        /// thread, so each thread of a survey opens its own.
        static OPENED: RefCell<Option<(PathBuf, gix::Repository)>> = const { RefCell::new(None) };
    }

    /// The files of the commits of the repository at `top`, read from its object database.
    #[derive(Debug, Clone)]
    pub(crate) struct GixSource {
        top: PathBuf,
    }

    impl GixSource {
        /// The source of the repository whose top folder is `top`. It is opened on first use.
        pub(crate) fn open(top: &Path) -> Self {
            Self {
                top: top.to_path_buf(),
            }
        }

        /// Runs `read` on this thread's handle of the repository.
        fn with<T>(&self, read: impl FnOnce(&gix::Repository) -> Result<T>) -> Result<T> {
            OPENED.with(|opened| {
                let mut opened = opened.borrow_mut();
                if opened.as_ref().is_none_or(|(top, _)| *top != self.top) {
                    let repo = gix::open(&self.top).with_context(|| {
                        format!("cannot open the repository at {}", self.top.display())
                    })?;
                    *opened = Some((self.top.clone(), repo));
                }
                let (_, repo) = opened.as_ref().context("the repository was just opened")?;
                read(repo)
            })
        }
    }

    /// A file of a commit: the id of its blob and what kind of entry it is.
    type Entries = BTreeMap<String, (ObjectId, EntryKind)>;

    impl ContentSource for GixSource {
        fn resolve_commit(&self, rev: &str) -> Result<String> {
            self.with(|repo| {
                let commit = (repo.rev_parse_single(rev))
                    .with_context(|| format!("{rev} names no commit"))?
                    .object()?
                    .peel_to_commit()
                    .with_context(|| format!("{rev} names no commit"))?;
                Ok(commit.id.to_hex().to_string())
            })
        }

        fn list_code_files(
            &self,
            paths: &[PathBuf],
            commit: Option<&str>,
        ) -> Result<Vec<(String, String, SupportLang)>> {
            let Some(commit) = commit else {
                bail!("the files of the working tree are on disk, which GixSource does not read");
            };
            let entries: Entries = self.with(|repo| {
                let id = ObjectId::from_hex(commit.as_bytes())
                    .with_context(|| format!("{commit} is not the full id of a commit"))?;
                let tree = repo.find_commit(id)?.tree()?;
                Ok((tree.traverse().breadthfirst.files()?.into_iter())
                    .filter(|entry| !entry.mode.is_tree())
                    .map(|entry| (entry.filepath.to_string(), (entry.oid, entry.mode.kind())))
                    .collect())
            })?;
            let ignores = self.with(|repo| ignores(repo, &entries))?;
            let mut files = Vec::new();
            for root in paths {
                let root = below_top(root);
                if let Some(&(oid, kind)) = entries.get(&root) {
                    if regular(kind)
                        && let Some(language) = code(Path::new(&root))
                    {
                        files.push((root, oid.to_hex().to_string(), language));
                    }
                    continue;
                }
                let prefix = if root.is_empty() {
                    String::new()
                } else {
                    format!("{root}/")
                };
                let walked = (entries.range(prefix.clone()..))
                    .take_while(|(path, _)| path.starts_with(&prefix))
                    .filter(|(path, (_, kind))| regular(*kind) && walked(&ignores, &root, path));
                for (path, (oid, _)) in walked {
                    if let Some(language) = code(Path::new(path)) {
                        files.push((path.clone(), oid.to_hex().to_string(), language));
                    }
                }
            }
            files.sort_by(|a, b| Path::new(&a.0).cmp(Path::new(&b.0)));
            files.dedup_by(|a, b| a.0 == b.0);
            Ok(files)
        }

        fn read_blob(&self, rel_path: &str, blob_oid: &str) -> Result<Arc<str>> {
            self.with(|repo| {
                let id = ObjectId::from_hex(blob_oid.as_bytes())
                    .with_context(|| format!("{blob_oid} is not the full id of a blob"))?;
                let blob = repo.find_blob(id)?;
                let text = std::str::from_utf8(&blob.data)
                    .with_context(|| format!("{rel_path} is not UTF-8"))?;
                Ok(Arc::from(text))
            })
        }

        fn commit_history(&self, commit: &str, max_commits: usize) -> Result<Vec<Vec<String>>> {
            self.with(|repo| {
                let tip = repo
                    .rev_parse_single(commit)?
                    .object()?
                    .peel_to_commit()?
                    .id;
                let walk = (repo.rev_walk([tip]))
                    .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
                    .all()?;
                let mut history = Vec::new();
                for info in walk {
                    if history.len() == max_commits {
                        break;
                    }
                    let info = info?;
                    let mut parents = info.parent_ids();
                    let parent = parents.next();
                    if parents.next().is_some() {
                        continue;
                    }
                    let tree = info.object()?.tree()?;
                    let before = match parent {
                        Some(parent) => Some(parent.object()?.peel_to_commit()?.tree()?),
                        None => None,
                    };
                    let changes = repo.diff_tree_to_tree(before.as_ref(), &tree, None)?;
                    history.push(
                        (changes.iter())
                            .filter(|change| !change.entry_mode().is_tree())
                            .map(|change| change.location().to_string())
                            .collect(),
                    );
                }
                Ok(history)
            })
        }
    }

    /// The path of `root` below the top of the repository, as git writes it. The top itself
    /// is the empty path.
    fn below_top(root: &Path) -> String {
        let parts: Vec<String> = (root.components())
            .filter(|part| !matches!(part, std::path::Component::CurDir))
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        parts.join("/")
    }

    /// Whether an entry is a file that a checkout writes as a regular file.
    fn regular(kind: EntryKind) -> bool {
        matches!(kind, EntryKind::Blob | EntryKind::BlobExecutable)
    }

    /// A language whose files hold code, as `ledger::files` keeps them. A data or markup file
    /// repeats its structure by nature.
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

    /// The patterns of each `.ignore` file of the commit, by the folder that holds it, which is
    /// the empty path at the top.
    fn ignores(repo: &gix::Repository, entries: &Entries) -> Result<HashMap<String, Gitignore>> {
        let mut ignores = HashMap::new();
        for (path, &(oid, kind)) in entries {
            let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
            if name != IGNORE || !regular(kind) {
                continue;
            }
            let blob = repo.find_blob(oid)?;
            let mut builder = GitignoreBuilder::new(Path::new(folder));
            for (nth, line) in String::from_utf8_lossy(&blob.data).lines().enumerate() {
                let line = if nth == 0 {
                    line.trim_start_matches('\u{feff}')
                } else {
                    line
                };
                let _ = builder.add_line(None, line);
            }
            ignores.insert(folder.to_owned(), builder.build()?);
        }
        Ok(ignores)
    }

    /// Whether the walk from `root` reaches the file at `path`: no folder on the way and not
    /// the file is hidden or left out by a `.ignore` file of a folder above it. The deepest
    /// `.ignore` file whose patterns name an entry decides, and a pattern that keeps an entry
    /// also keeps it when it is hidden.
    fn walked(ignores: &HashMap<String, Gitignore>, root: &str, path: &str) -> bool {
        let start = if root.is_empty() { 0 } else { root.len() + 1 };
        let parts: Vec<&str> = path[start..].split('/').collect();
        let mut end = start;
        for (nth, part) in parts.iter().enumerate() {
            end += part.len();
            let entry = &path[..end];
            let is_dir = nth + 1 < parts.len();
            let mut folder = entry.rsplit_once('/').map_or("", |(folder, _)| folder);
            let decided = loop {
                let named = (ignores.get(folder)).map_or(Match::None, |ignore| {
                    ignore.matched(Path::new(entry), is_dir)
                });
                if !named.is_none() || folder.is_empty() {
                    break named;
                }
                folder = folder.rsplit_once('/').map_or("", |(above, _)| above);
            };
            if decided.is_ignore() || (decided.is_none() && part.starts_with('.')) {
                return false;
            }
            end += 1;
        }
        true
    }
}
