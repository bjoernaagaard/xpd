//! The git history of a repository as evidence: how often each file changed, and which files
//! changed in the same commits.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

/// How many commits are read, newest first. Older commits say little about how the code
/// changes today.
const COMMITS_READ: usize = 2000;
/// A commit that changes more files than this is a sweep, such as a rename or a formatter run,
/// and says nothing about which files belong together.
const SWEEP_FILES: usize = 25;
/// The fewest commits two files share before the pair is reported.
const SHARED_COMMITS: usize = 5;

/// The files that each commit changed, as paths below the top of the repository with `/`.
#[derive(Debug, Default)]
pub(super) struct History {
    commits: Vec<Vec<String>>,
}

/// Two files that changed in the same commits.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CochangePair {
    pub(super) first: String,
    pub(super) second: String,
    /// The commits that changed both files.
    pub(super) together: usize,
    /// The commits that changed the file of the two that changed less often.
    pub(super) of: usize,
}

impl History {
    /// The non-merge history for `revision` in the repository at `top`. Empty when it cannot
    /// be read.
    pub(super) fn read(top: &Path, revision: &str) -> Self {
        Self::read_gix(top, revision)
    }

    pub(super) fn read_gix(top: &Path, revision: &str) -> Self {
        Self::try_read_gix(top, revision).unwrap_or_default()
    }

    fn try_read_gix(top: &Path, revision: &str) -> gix::Result<Self> {
        use gix::bstr::ByteSlice;
        use gix::revision::walk::{CommitTimeOrder, Sorting};

        let repo = gix::open(top)?;
        let tip = repo
            .rev_parse_single(revision)?
            .object()?
            .peel_to_commit()?;
        let walk = repo
            .rev_walk([tip.id])
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
            .all()?;
        let mut commits = Vec::new();
        let mut remaining = COMMITS_READ;
        for entry in walk {
            let entry = entry?;
            if entry.parent_ids.len() > 1 {
                continue;
            }

            let tree = repo.find_commit(entry.id)?.tree()?;
            let parent_tree = entry
                .parent_ids
                .first()
                .map(|id| repo.find_commit(*id)?.tree())
                .transpose()?;
            let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), &tree, None)?;
            let mut paths: Vec<_> = diff
                .iter()
                .filter(|change| !change.entry_mode().is_tree())
                .map(gix::object::tree::diff::ChangeDetached::location)
                .collect();
            paths.sort_unstable();
            paths.dedup();
            let files: Vec<_> = paths
                .into_iter()
                .map(|path| {
                    gix::quote::ansi_c::quote(path)
                        .to_str_lossy()
                        .trim()
                        .to_owned()
                })
                .filter(|path| !path.is_empty())
                .collect();
            if !files.is_empty() {
                commits.push(files);
            }
            remaining -= 1;
            if remaining == 0 {
                break;
            }
        }
        Ok(Self { commits })
    }

    pub(super) fn len(&self) -> usize {
        self.commits.len()
    }

    /// The commits that changed each file, as their places in the history, newest first.
    pub(super) fn changes(&self) -> HashMap<&str, Vec<usize>> {
        let mut changes: HashMap<&str, Vec<usize>> = HashMap::new();
        for (nth, commit) in self.commits.iter().enumerate() {
            for file in commit {
                changes.entry(file.as_str()).or_default().push(nth);
            }
        }
        changes
    }

    /// The pairs of `files` that changed together in at least five commits, and in at least
    /// half of the commits of the file that changed less often. Sweeps are left out.
    pub(super) fn pairs(&self, files: &BTreeSet<&str>) -> Vec<CochangePair> {
        let mut alone: BTreeMap<&str, usize> = BTreeMap::new();
        let mut together: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for commit in &self.commits {
            if commit.len() > SWEEP_FILES {
                continue;
            }
            let changed: BTreeSet<&str> = (commit.iter().map(String::as_str))
                .filter(|file| files.contains(file))
                .collect();
            for (nth, first) in changed.iter().enumerate() {
                *alone.entry(first).or_default() += 1;
                for second in changed.iter().skip(nth + 1) {
                    *together.entry((first, second)).or_default() += 1;
                }
            }
        }
        let mut pairs: Vec<CochangePair> = (together.into_iter())
            .filter_map(|((first, second), together)| {
                let of = alone[first].min(alone[second]);
                (together >= SHARED_COMMITS && together * 2 >= of).then(|| CochangePair {
                    first: first.to_owned(),
                    second: second.to_owned(),
                    together,
                    of,
                })
            })
            .collect();
        pairs.sort_by(|a, b| (b.together * a.of, b.together).cmp(&(a.together * b.of, a.together)));
        pairs
    }
}
