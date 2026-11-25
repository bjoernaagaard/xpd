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

/// The folder that the absolute imports of a module in `package` start from: the folder that
/// holds the outermost package. A file in no package is a script, and Python starts from the
/// script's own folder.
fn import_root(package: &Path) -> &Path {
    let mut top = package;
    while top.join(INIT).is_file() {
        let Some(above) = top.parent() else {
            break;
        };
        top = above;
    }
    top
}

/// The folder and the dotted name that the `from` import `import` starts from. `None` for a
/// relative import that climbs above the file system.
fn from_module<'p>(
    import: &Node<'_>,
    package: &'p Path,
    top: &'p Path,
) -> Option<(&'p Path, Vec<String>)> {
    let module = import.field("module_name")?;
    if module.kind() != "relative_import" {
        return Some((top, parts(&module)));
    }
    let dots = module.child(0).map_or(1, |prefix| prefix.text().len());
    let base = package.ancestors().nth(dots.saturating_sub(1))?;
    let named = module
        .children()
        .find(|child| child.kind() == "dotted_name");
    Some((base, named.as_ref().map(parts).unwrap_or_default()))
}

/// The files that the module at `path` loads itself: the `__init__.py` of each package it is in,
/// and each module it imports unconditionally at module level that is in the same project.
fn loaded_by(path: &Path, root: &Root) -> Vec<PathBuf> {
    let package = path.parent().unwrap_or(Path::new(""));
    let top = import_root(package);
    let mut loaded: Vec<PathBuf> = (package.ancestors())
        .take_while(|folder| *folder != top)
        .map(|folder| folder.join(INIT))
        .collect();
    for import in root.root().children() {
        match &*import.kind() {
            "import_statement" => {
                for name in import.field_children("name") {
                    loaded.extend(module_files(top, &parts(&name)));
                }
            }
            "import_from_statement" => {
                let Some((base, mut module)) = from_module(&import, package, top) else {
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

/// One name that a module of the project gives to the file that imports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Import {
    /// The file of the imported module.
    pub(crate) module: PathBuf,
    /// The name that `from module import name` takes from it. `None` when the module itself is
    /// imported.
    pub(crate) name: Option<String>,
    /// The name the importing file uses for the module or the name.
    pub(crate) bound: String,
    /// The line of the import statement, counted from 1.
    pub(crate) line: usize,
}

/// Every import of a module of the same project in the file at `path`, wherever the statement
/// stands, also in a function or under `if TYPE_CHECKING:`. `from package import module` gives
/// the module, and no entry for the package. `from . import name` names no module, so it starts
/// from the package itself.
pub(crate) fn imported(path: &Path, root: &Root) -> Vec<Import> {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let package = path.parent().unwrap_or(Path::new(""));
    let top = import_root(package);
    let mut imports = Vec::new();
    for import in root.root().dfs() {
        let line = import.start_pos().line() + 1;
        match &*import.kind() {
            "import_statement" => {
                for name in import.field_children("name") {
                    let dotted = parts(&name);
                    let Some(module) = module_file(top, &dotted) else {
                        continue;
                    };
                    imports.push(Import {
                        module,
                        name: None,
                        bound: bound_name(&name).unwrap_or_else(|| dotted.join(".")),
                        line,
                    });
                }
            }
            "import_from_statement" => {
                let Some((base, mut module)) = from_module(&import, package, top) else {
                    continue;
                };
                let package_file = if module.is_empty() {
                    Some(base.join(INIT)).filter(|init| init.is_file())
                } else {
                    module_file(base, &module)
                };
                let Some(package_file) = package_file else {
                    continue;
                };
                for name in import.field_children("name") {
                    let taken = parts(&name);
                    module.extend(taken.iter().cloned());
                    let found = module_file(base, &module);
                    module.truncate(module.len() - taken.len());
                    let bound = bound_name(&name).unwrap_or_else(|| taken.join("."));
                    imports.push(match found {
                        Some(submodule) => Import {
                            module: submodule,
                            name: None,
                            bound,
                            line,
                        },
                        None => Import {
                            module: package_file.clone(),
                            name: Some(taken.join(".")),
                            bound,
                            line,
                        },
                    });
                }
            }
            _ => {}
        }
    }
    imports
}

/// The file of the module `parts` below the folder `base`: the `__init__.py` of a package, or
/// the file of a module. `None` for a module that is not below `base`.
fn module_file(base: &Path, parts: &[String]) -> Option<PathBuf> {
    let (last, packages) = parts.split_last()?;
    let folder = packages
        .iter()
        .fold(base.to_path_buf(), |at, part| at.join(part));
    [
        folder.join(last).join(INIT),
        folder.join(format!("{last}.py")),
    ]
    .into_iter()
    .find(|file| file.is_file())
}

/// The name after `as` in `a.b as c`.
fn bound_name(name: &Node<'_>) -> Option<String> {
    (name.kind() == "aliased_import")
        .then(|| name.field("alias"))
        .flatten()
        .map(|alias| alias.text().into_owned())
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
