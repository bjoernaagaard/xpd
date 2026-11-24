//! Folders and files that only the user can read, for what xpd writes that can hold settings of
//! a project: the copy of the Ruff settings that `xpd doctor` checks, the journal of
//! `xpd doctor --apply`, and the session files of `xpd hook`.

use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, RandomState};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// How often [`Scratch::new`] tries another name when the one it drew exists.
const ATTEMPTS: usize = 16;

/// Options that create a new file only the user can read and write, mode 0600 on Unix.
pub(crate) fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

/// Writes `text` to the new file `path`, mode 0600, and fails when it exists.
pub(crate) fn write_new(path: &Path, text: &str) -> io::Result<()> {
    use io::Write;
    let mut file: File = options().create_new(true).open(path)?;
    file.write_all(text.as_bytes())
}

/// Makes the folder `path`, mode 0700, and fails when it exists.
pub(crate) fn create_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

/// A new folder in the system's temporary folder with a name no other process can predict,
/// mode 0700. Dropping it removes it with everything in it, on every path out of the code that
/// holds it.
pub(crate) struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Makes the folder `<prefix>-<random>` in the temporary folder.
    pub(crate) fn new(prefix: &str) -> io::Result<Self> {
        let base = std::env::temp_dir();
        let mut last = None;
        for attempt in 0..ATTEMPTS {
            let seed = (std::process::id(), attempt, SystemTime::now());
            let name = format!(
                "{prefix}-{:016x}{:016x}",
                RandomState::new().hash_one(seed),
                RandomState::new().hash_one(seed)
            );
            let path = base.join(name);
            match create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(last.unwrap_or_else(|| io::Error::other("no free name for a scratch folder")))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
