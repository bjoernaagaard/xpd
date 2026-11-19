//! Finds the optional Ruff and ty executables and runs them. Doctor never installs a tool: an
//! executable that is not in the project's venv or on `PATH` leaves its advice unverified.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::catalog::Tool;
use crate::venv::Venv;

/// An executable of Ruff or ty and the release it reports.
#[derive(Debug, Clone)]
pub(crate) struct Installed {
    pub(crate) path: PathBuf,
    pub(crate) version: String,
}

/// One optional checker, found or not.
#[derive(Debug, Clone)]
pub(crate) struct Checker {
    pub(crate) tool: Tool,
    pub(crate) installed: Option<Installed>,
}

impl Checker {
    /// The checker in the venv of `root`, or else on `PATH`.
    pub(crate) fn find(tool: Tool, root: &Path) -> Self {
        let installed = executable(tool.name(), root).and_then(|path| {
            let output = Command::new(&path).arg("--version").output().ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            let version = text.split_whitespace().nth(1)?.to_owned();
            output
                .status
                .success()
                .then_some(Installed { path, version })
        });
        Self { tool, installed }
    }

    /// Whether the installed release is the one the decisions were reviewed for.
    pub(crate) fn is_pinned(&self) -> bool {
        self.installed
            .as_ref()
            .is_some_and(|installed| installed.version == self.tool.pinned())
    }

    /// The standard output of the checker run with `args` in `dir`, or the error it printed.
    pub(crate) fn run<S: AsRef<OsStr>>(
        &self,
        args: &[S],
        dir: &Path,
    ) -> Option<Result<String, String>> {
        let installed = self.installed.as_ref()?;
        let output = Command::new(&installed.path)
            .args(args)
            .current_dir(dir)
            .output();
        Some(match output {
            Ok(output) if output.status.success() => {
                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            }
            Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
            Err(error) => Err(error.to_string()),
        })
    }

    /// The status of advice that the installed checker confirmed.
    pub(crate) fn verified(&self) -> String {
        match &self.installed {
            Some(installed) => format!(
                "verified with {} {} (decisions reviewed for {})",
                self.tool.name(),
                installed.version,
                self.tool.pinned()
            ),
            None => self.unverified(),
        }
    }

    /// The status of advice that rests on the pinned release's decisions and defaults alone.
    pub(crate) fn unverified(&self) -> String {
        format!(
            "unverified locally (decisions for {} {})",
            self.tool.name(),
            self.tool.pinned()
        )
    }
}

/// The executable `name` in the project's venv, or else the first on `PATH`.
fn executable(name: &str, root: &Path) -> Option<PathBuf> {
    let names = [name.to_owned(), format!("{name}.exe")];
    let venv = Venv::find_from(root).map(|venv| {
        let root = venv.root().to_path_buf();
        vec![root.join("bin"), root.join("Scripts")]
    });
    let path = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    venv.into_iter()
        .flatten()
        .chain(path)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}
