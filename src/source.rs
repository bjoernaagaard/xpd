use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ast_grep_core::Language;
use ast_grep_language::SupportLang;

/// The language of the file at `path`, by its name. A file that names no language is read as
/// Python, so that a script without an extension is linted.
pub(crate) fn language_of(path: &Path) -> SupportLang {
    SupportLang::from_path(path).unwrap_or(SupportLang::Python)
}

/// One file, as a lint run names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct File<'a> {
    /// The path that the `files` and `ignores` globs of a rule match, which selects the rules
    /// that run.
    pub(crate) path: &'a Path,
    /// Where the file is on disk, from which a rule follows its imports. `None` for code that
    /// is in no file, such as a recipe.
    pub(crate) disk: Option<&'a Path>,
}

impl<'a> File<'a> {
    /// A file that is on disk at the path its rules match.
    pub(crate) fn on_disk(path: &'a Path) -> Self {
        Self {
            path,
            disk: Some(path),
        }
    }
}

/// Whether `path` has the `.py` extension.
pub(crate) fn is_python(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("py"))
}

/// Whether `path` is a test file, by the names pytest collects.
pub(crate) fn is_test(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name == "conftest.py"
        || name.starts_with("test_")
        || name.ends_with("_test.py")
        || path.components().any(|part| part.as_os_str() == "tests")
}

/// Whether the name of `path` tells its language.
pub(crate) fn names_language(path: &Path) -> bool {
    SupportLang::from_path(path).is_some()
}

pub(crate) fn cannot_read(path: &Path) -> String {
    format!("cannot read {}", path.display())
}

/// Reads `path` as UTF-8, returning `Ok(None)` when the file does not exist.
pub(crate) fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| cannot_read(path)),
    }
}

enum ReadError {
    Notebook,
    Io(std::io::Error),
}

impl File<'_> {
    fn read(self) -> std::result::Result<String, ReadError> {
        if self
            .path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ipynb"))
        {
            return Err(ReadError::Notebook);
        }
        fs::read_to_string(self.disk.unwrap_or(self.path)).map_err(ReadError::Io)
    }
}

/// Lint commands name unreadable files on stderr and continue with the rest.
pub(crate) fn read_source(file: File<'_>) -> Option<String> {
    let path = file.path;
    match file.read() {
        Ok(source) => return Some(source),
        Err(ReadError::Notebook) => eprintln!(
            "xpd: {} is a Jupyter notebook, and xpd reads .py files. Write it as a .py file with cell markers, as in `jupytext --to py:percent {}`, and check that file",
            path.display(),
            path.display()
        ),
        Err(ReadError::Io(error)) => eprintln!("xpd: {}: {error}", cannot_read(path)),
    }
    None
}

/// The Python files under `paths`, each once. Folders respect `.gitignore`.
pub(crate) fn python_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    for path in paths {
        let meta = fs::metadata(path).with_context(|| cannot_read(path))?;
        if meta.is_file() && !is_python(path) {
            bail!("{} is not a Python file", path.display());
        }
    }
    files_under(Path::new(""), paths, is_python)
}

/// The files under `paths` that `wanted` keeps, each once. A path that is a file is always
/// taken.
pub(crate) fn files_under(
    root: &Path,
    paths: &[PathBuf],
    wanted: impl Fn(&Path) -> bool,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |path: &Path, disk: &Path| -> Result<()> {
        let real = fs::canonicalize(disk).with_context(|| cannot_read(path))?;
        if seen.insert(real) {
            files.push(path.to_path_buf());
        }
        Ok(())
    };
    for p in paths {
        let disk = root.join(p);
        let meta = fs::metadata(&disk).with_context(|| cannot_read(p))?;
        if meta.is_file() {
            add(p, &disk)?;
            continue;
        }
        for entry in ignore::WalkBuilder::new(&disk).build() {
            let entry = entry.with_context(|| cannot_read(p))?;
            if entry.file_type().is_some_and(|t| t.is_file()) && wanted(entry.path()) {
                let tail = entry.path().strip_prefix(&disk).unwrap_or(entry.path());
                add(&p.join(tail), entry.path())?;
            }
        }
    }
    Ok(files)
}
