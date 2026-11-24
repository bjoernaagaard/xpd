//! `xpd doctor --apply`: writes the plan to `pyproject.toml`, refreshes the lock files the
//! project's kind needs, and records the profile revision last.
//!
//! The write runs under a journal, the folder `.xpd-apply` at the project root. Making the
//! folder is the writer lock, and it holds the prior content of every file the write touches.
//! A failed lock refresh or a check that finds drift after the write puts the prior content back.
//! A journal that a dead process left behind is restored by the next apply before it plans.
//! Apply never syncs an environment, installs hooks, runs project scripts or publishes.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use super::catalog::Catalog;
use super::plan::{self, Change, Lock, PYPROJECT, Plan, Request};
use super::profile::Profile;

/// The journal folder at the project root.
const JOURNAL: &str = ".xpd-apply";
/// The file in the journal that names the process writing.
const OWNER: &str = "owner";
/// The file in the journal that holds the prior content of each file.
const MANIFEST: &str = "manifest.json";

/// What one apply did.
#[derive(Debug, Serialize)]
pub(crate) struct Applied {
    pub(crate) revision: i64,
    pub(crate) kind: Option<String>,
    pub(crate) changes: Vec<Change>,
    pub(crate) locks: Vec<Lock>,
    /// Files restored from the journal of an apply that did not finish.
    pub(crate) recovered: Vec<PathBuf>,
}

impl Applied {
    pub(crate) fn to_text(&self) -> String {
        let mut out = String::new();
        for file in &self.recovered {
            let _ = writeln!(
                out,
                "Restored {} from an apply that did not finish.",
                file.display()
            );
        }
        if self.changes.is_empty() && self.locks.is_empty() {
            let _ = writeln!(
                out,
                "The project matches profile revision {}. Nothing to apply.",
                self.revision
            );
            return out;
        }
        if !self.changes.is_empty() {
            let _ = writeln!(
                out,
                "Applied profile revision {}, kind {}, to {PYPROJECT}.",
                self.revision,
                self.kind.as_deref().unwrap_or_default()
            );
        }
        for change in &self.changes {
            let _ = writeln!(
                out,
                "  {} {}: {} -> {}",
                change.path, change.item, change.old, change.new
            );
        }
        for lock in &self.locks {
            let _ = writeln!(
                out,
                "Ran {}, which wrote {}.",
                lock.command.join(" "),
                lock.file.display()
            );
        }
        out
    }
}

/// Applies the profile to the project at `root`.
///
/// # Errors
///
/// Returns an error, after putting back the prior content of every file, if the plan has a
/// conflict or an exception the profile does not accept, the kind is not named, another apply
/// runs, a file changed while planning, a lock refresh fails, or drift remains after the write.
pub(crate) fn run(
    root: &Path,
    catalog: &Catalog,
    profile: &Profile,
    files: &[PathBuf],
    kind: Option<&str>,
) -> Result<Applied> {
    let recovered = Journal::recover(root)?;
    let mut full = plan::compute(root, catalog, profile, files, Request { kind, stamp: true })?;
    full.check_tools(root, super::validate::Fetch::Online);
    if !full.unverified.is_empty() {
        bail!(
            "doctor wrote nothing, because it could not check the settings it would write:\n  {}",
            full.unverified.join("\n  ")
        );
    }
    if full.kind_from == Some("suggestion") {
        bail!(
            "{PYPROJECT} records no kind. Name it: xpd doctor --apply --kind {}, which doctor suggests because {}",
            full.kind.as_deref().unwrap_or_default(),
            full.kind_evidence.as_deref().unwrap_or_default()
        );
    }
    let blockers = full.blockers();
    if !blockers.is_empty() {
        bail!(
            "doctor wrote nothing, because the plan has conflicts:\n  {}",
            blockers.join("\n  ")
        );
    }
    full.check_locks(root)?;
    if !full.drifts() {
        return Ok(Applied {
            revision: full.revision,
            kind: full.kind,
            changes: Vec::new(),
            locks: Vec::new(),
            recovered,
        });
    }
    let settings = plan::compute(
        root,
        catalog,
        profile,
        files,
        Request { kind, stamp: false },
    )?;
    let locks: Vec<Lock> = full
        .locks
        .iter()
        .filter(|lock| lock.state != "present")
        .cloned()
        .collect();
    if !locks.is_empty() {
        uv_runs()?;
    }
    let mut touched = vec![PathBuf::from(PYPROJECT)];
    touched.extend(settings.owned.iter().map(|owned| owned.path.clone()));
    touched.extend(locks.iter().map(|lock| lock.file.clone()));
    let journal = Journal::begin(root, &touched)?;
    let result = write(
        root, catalog, profile, files, kind, &full, &settings, &locks,
    );
    match result {
        Ok(()) => {
            journal.finish()?;
            Ok(Applied {
                revision: full.revision,
                kind: full.kind,
                changes: full.changes,
                locks,
                recovered,
            })
        }
        Err(error) => {
            journal
                .restore()
                .context("the prior content could not be put back")?;
            Err(error.context("doctor put back the prior content of every file"))
        }
    }
}

/// The writes of one apply, inside its journal.
#[expect(
    clippy::too_many_arguments,
    reason = "the write needs both plans and what made them"
)]
fn write(
    root: &Path,
    catalog: &Catalog,
    profile: &Profile,
    files: &[PathBuf],
    kind: Option<&str>,
    full: &Plan,
    settings: &Plan,
    locks: &[Lock],
) -> Result<()> {
    let file = root.join(PYPROJECT);
    let now = read(&file)?;
    ensure!(
        now == full.before,
        "{PYPROJECT} changed while doctor planned. Run xpd doctor --apply again"
    );
    for owned in &settings.owned {
        let path = root.join(&owned.path);
        ensure!(
            read(&path)? == owned.before,
            "{} changed while doctor planned. Run xpd doctor --apply again",
            owned.path.display()
        );
    }
    if Some(&settings.after) != full.before.as_ref() {
        replace(&file, &settings.after)?;
    }
    for owned in &settings.owned {
        let path = root.join(&owned.path);
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)
                .with_context(|| format!("cannot make {}", folder.display()))?;
        }
        replace(&path, &owned.after)?;
    }
    for lock in locks {
        let (program, args) = lock.command.split_first().context("a lock has a command")?;
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("cannot run {}", lock.command.join(" ")))?;
        if !output.status.success() {
            bail!(
                "{} failed: {}",
                lock.command.join(" "),
                crate::output::hosts_only(String::from_utf8_lossy(&output.stderr).trim())
            );
        }
    }
    let mut check = plan::compute(
        root,
        catalog,
        profile,
        files,
        Request { kind, stamp: false },
    )?;
    check.check_locks(root)?;
    let left = check.drift();
    ensure!(
        left.is_empty(),
        "drift remains after the write:\n  {}",
        left.join("\n  ")
    );
    let stamped = plan::compute(root, catalog, profile, files, Request { kind, stamp: true })?;
    if Some(&stamped.after) != stamped.before.as_ref() {
        replace(&file, &stamped.after)?;
    }
    Ok(())
}

/// Checks that `uv` runs, before any file changes.
fn uv_runs() -> Result<()> {
    let output = Command::new("uv")
        .arg("--version")
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => bail!(
            "uv --version failed, so doctor cannot write the lock files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => bail!("doctor needs uv on PATH to write the lock files: {error}"),
    }
}

/// The text of `file`, `None` when it does not exist.
fn read(file: &Path) -> Result<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("cannot read {}", file.display())),
    }
}

/// Replaces `file` with `text` in one rename, so a reader sees the old or the new file. The new
/// file is written with mode 0600, so no other user reads it while it is written, and then takes
/// the permissions of the old one.
fn replace(file: &Path, text: &str) -> Result<()> {
    let name = file
        .file_name()
        .context("a file has a name")?
        .to_string_lossy();
    let temporary = file.with_file_name(format!(".{name}.xpd-{}", std::process::id()));
    let _ = fs::remove_file(&temporary);
    crate::private::write_new(&temporary, text)
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    if let Ok(metadata) = fs::metadata(file) {
        fs::set_permissions(&temporary, metadata.permissions())
            .with_context(|| format!("cannot set the permissions of {}", temporary.display()))?;
    }
    fs::rename(&temporary, file).with_context(|| format!("cannot replace {}", file.display()))
}

/// The prior content of each file an apply touches. `None` for a file that did not exist.
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    files: BTreeMap<PathBuf, Option<String>>,
}

/// The journal of one apply, at the root of the project it writes.
#[derive(Debug)]
struct Journal {
    root: PathBuf,
    folder: PathBuf,
}

impl Journal {
    /// Makes the journal, which fails while another apply holds it, and saves the prior content
    /// of `files`, each relative to `root`.
    fn begin(root: &Path, files: &[PathBuf]) -> Result<Self> {
        let folder = root.join(JOURNAL);
        match crate::private::create_dir(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => bail!(
                "{} exists, so another xpd doctor --apply is writing this project",
                folder.display()
            ),
            Err(error) => {
                return Err(error).with_context(|| format!("cannot make {}", folder.display()));
            }
        }
        let journal = Self {
            root: root.to_path_buf(),
            folder,
        };
        crate::private::write_new(&journal.folder.join(OWNER), &std::process::id().to_string())?;
        let mut manifest = Manifest {
            files: BTreeMap::new(),
        };
        for file in files {
            manifest.files.insert(file.clone(), read(&root.join(file))?);
        }
        replace(
            &journal.folder.join(MANIFEST),
            &serde_json::to_string(&manifest)?,
        )?;
        Ok(journal)
    }

    /// Restores the files of a journal whose process is gone. Fails while that process runs.
    fn recover(root: &Path) -> Result<Vec<PathBuf>> {
        let folder = root.join(JOURNAL);
        if !folder.is_dir() {
            return Ok(Vec::new());
        }
        let owner = fs::read_to_string(folder.join(OWNER))
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok());
        if let Some(owner) = owner
            && alive(owner)
        {
            bail!(
                "process {owner} is writing this project with xpd doctor --apply. Wait for it to end"
            );
        }
        let journal = Self {
            root: root.to_path_buf(),
            folder,
        };
        let restored = journal.restore()?;
        Ok(restored)
    }

    /// Puts back the prior content of every file, removes the journal and returns the files.
    fn restore(&self) -> Result<Vec<PathBuf>> {
        let manifest = match fs::read_to_string(self.folder.join(MANIFEST)) {
            Ok(text) => serde_json::from_str::<Manifest>(&text)
                .with_context(|| format!("{} does not parse", self.folder.display()))?,
            Err(error) if error.kind() == ErrorKind::NotFound => Manifest {
                files: BTreeMap::new(),
            },
            Err(error) => return Err(error.into()),
        };
        for (file, content) in &manifest.files {
            let path = self.root.join(file);
            match content {
                Some(text) => replace(&path, text)?,
                None => match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                },
            }
        }
        fs::remove_dir_all(&self.folder)
            .with_context(|| format!("cannot remove {}", self.folder.display()))?;
        Ok(manifest.files.into_keys().collect())
    }

    fn finish(self) -> Result<()> {
        fs::remove_dir_all(&self.folder)
            .with_context(|| format!("cannot remove {}", self.folder.display()))
    }
}

/// Whether the process `pid` runs.
fn alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    if cfg!(windows) {
        let filter = format!("PID eq {pid}");
        Command::new("tasklist")
            .args(["/FI", &filter, "/NH"])
            .stdin(Stdio::null())
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
    } else {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}
