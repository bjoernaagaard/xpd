//! The git history of a repository as evidence: how often each file changed, and which files
//! changed in the same commits.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::git;

/// How many commits are read, newest first. Older commits say little about how the code
/// changes today.
const COMMITS_READ: &str = "2000";
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
pub(super) struct Pair {
    pub(super) first: String,
    pub(super) second: String,
    /// The commits that changed both files.
    pub(super) together: usize,
    /// The commits that changed the file of the two that changed less often.
    pub(super) of: usize,
}

impl History {
    /// The history that `git log` prints for `revision` in the repository at `top`. Empty when
    /// git cannot read it.
    pub(super) fn read(top: &Path, revision: &str) -> Self {
        let args = [
            "log",
            "--no-merges",
            "--name-only",
            "--format=%x00",
            "-n",
            COMMITS_READ,
            revision,
            "--",
        ];
        git::output(top, &args).map_or_else(|_| Self::default(), |log| Self::parse(&log))
    }

    /// The commits of a log printed with `--name-only --format=%x00`: a NUL starts each commit,
    /// and each line after it names one file.
    pub(super) fn parse(log: &str) -> Self {
        let commits = log
            .split('\0')
            .map(|commit| {
                (commit.lines().map(str::trim))
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|files| !files.is_empty())
            .collect();
        Self { commits }
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
    pub(super) fn pairs(&self, files: &BTreeSet<&str>) -> Vec<Pair> {
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
        let mut pairs: Vec<Pair> = (together.into_iter())
            .filter_map(|((first, second), together)| {
                let of = alone[first].min(alone[second]);
                (together >= SHARED_COMMITS && together * 2 >= of).then(|| Pair {
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
