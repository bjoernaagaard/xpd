//! The modules of a project that a Python file loads: the `__init__.py` of each package it is
//! in, the modules of the project it imports, and what those load in turn.
//!
//! A statement in one of them can change what the file's own code does, as
//! `pio.templates.default = "simple_white"` in a package's `__init__.py` changes every figure
//! of the package. A rule that reports what a file leaves unset reads these modules first.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::syntax::{self, Node, Root};

/// The file that makes a folder a package.
const INIT: &str = "__init__.py";

/// Whether `found` holds for a module that the file at `path` loads. `root` is the parsed file,
/// which may be newer than the one on disk. The nearest modules are read first.
pub(crate) fn any_loaded(path: &Path, root: &Root, found: impl Fn(&Root) -> bool) -> bool {
    // Every path below is built from this one, so that one file has one path.
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut open = VecDeque::from(loaded_by(&path, root));
    let mut seen = HashSet::from([path]);
    while let Some(module) = open.pop_front() {
        if !seen.insert(module.clone()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&module) else {
            continue;
        };
        let tree = syntax::parse(&text);
        if found(&tree) {
            return true;
        }
        open.extend(loaded_by(&module, &tree));
    }
    false
}

/// The files that the module at `path` loads itself: the `__init__.py` of each package it is in,
/// and each module it imports unconditionally at module level that is in the same project.
fn loaded_by(path: &Path, root: &Root) -> Vec<PathBuf> {
    let package = path.parent().unwrap_or(Path::new(""));
    let mut loaded = Vec::new();
    // Absolute imports start from the folder that holds the outermost package. A file in no
    // package is a script, and Python starts from the script's own folder.
    let mut top = package;
    while top.join(INIT).is_file() {
        loaded.push(top.join(INIT));
        let Some(above) = top.parent() else {
            break;
        };
        top = above;
    }
    for import in root.root().children() {
        match &*import.kind() {
            "import_statement" => {
                for name in import.field_children("name") {
                    loaded.extend(module_files(top, &parts(&name)));
                }
            }
            "import_from_statement" => {
                let Some(module) = import.field("module_name") else {
                    continue;
                };
                let (base, mut module) = if module.kind() == "relative_import" {
                    let dots = module.child(0).map_or(1, |prefix| prefix.text().len());
                    let base = package.ancestors().nth(dots.saturating_sub(1));
                    let named = module
                        .children()
                        .find(|child| child.kind() == "dotted_name");
                    (base, named.as_ref().map(parts).unwrap_or_default())
                } else {
                    (Some(top), parts(&module))
                };
                let Some(base) = base else {
                    continue;
                };
                loaded.extend(module_files(base, &module));
                // A name that `from` imports can be a module of the package.
                for name in import.field_children("name") {
                    module.extend(parts(&name));
                    loaded.extend(module_files(base, &module));
                    module.pop();
                }
            }
            _ => {}
        }
    }
    loaded
}

/// The parts of a dotted module name. For `a.b as c` they are the parts of `a.b`.
fn parts(name: &Node<'_>) -> Vec<String> {
    let dotted = match name.field("name") {
        Some(dotted) if name.kind() == "aliased_import" => dotted,
        _ => name.clone(),
    };
    dotted
        .text()
        .split('.')
        .map(|part| part.trim().to_owned())
        .collect()
}

/// The files that importing the module `parts` from the folder `base` runs: the `__init__.py`
/// of each package on the way, and the module itself. Empty for a module that is not below
/// `base`, such as one of the standard library.
fn module_files(base: &Path, parts: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut folder = base.to_path_buf();
    for (nth, part) in parts.iter().enumerate() {
        let file = folder.join(format!("{part}.py"));
        folder.push(part);
        let init = folder.join(INIT);
        if init.is_file() {
            files.push(init);
        } else if nth + 1 == parts.len() && file.is_file() {
            files.push(file);
        } else if !folder.is_dir() {
            break;
        }
    }
    files
}
