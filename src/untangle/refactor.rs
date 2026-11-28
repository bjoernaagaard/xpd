//! Format-preserving mechanical Python refactors validated with `libcst`:
//! `xpd untangle --move <from> <to>` and `xpd untangle --inline-facade <facade>`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use ast_grep_language::SupportLang;
use serde::Serialize;
use similar::TextDiff;

use crate::imports;
use crate::ledger::walk_code_files;
use crate::output::Format;
use crate::syntax::{self, Node};

/// Whether a mechanical refactor writes changes to disk or only prints a preview diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RefactorMode {
    DryRun,
    Apply,
}

#[derive(Debug, Serialize)]
struct FileChange {
    path: String,
    rewrites: usize,
    diff: String,
}

#[derive(Debug, Serialize)]
struct RefactorReport {
    action: String,
    applied: bool,
    moved: Option<(String, String)>,
    removed: Option<String>,
    files_changed: usize,
    total_rewrites: usize,
    changes: Vec<FileChange>,
}

#[derive(Debug, Clone)]
struct ByteEdit {
    start: usize,
    end: usize,
    replacement: String,
}

/// Moves a Python module from `from` to `to` and rewrites every absolute and relative import
/// referencing it across `project_root`, preserving formatting and validating every edited
/// file with `libcst`.
pub(super) fn move_module(
    project_root: &Path,
    from: &Path,
    to: &Path,
    mode: RefactorMode,
    format: Format,
) -> Result<()> {
    let root = fs::canonicalize(project_root).or_else(|_| std::path::absolute(project_root))?;
    let from_abs = fs::canonicalize(from)
        .with_context(|| format!("source module {} does not exist", from.display()))?;
    ensure!(
        from_abs.extension().and_then(|ext| ext.to_str()) == Some("py"),
        "source {} must be a .py module",
        from.display()
    );
    let to_abs = if to.is_absolute() {
        to.to_path_buf()
    } else {
        std::path::absolute(to)?
    };
    ensure!(
        to_abs.extension().and_then(|ext| ext.to_str()) == Some("py"),
        "destination {} must be a .py module",
        to.display()
    );
    ensure!(
        !to_abs.exists(),
        "destination {} already exists",
        to_abs.display()
    );

    let old_mod = imports::module_name(&from_abs, Path::is_file)
        .with_context(|| format!("cannot infer Python module name for {}", from_abs.display()))?;
    let new_mod = infer_destination_module(&root, &from_abs, &to_abs)?;

    let py_files = collect_python_files(&root);
    let mut planned: BTreeMap<PathBuf, (String, String, usize)> = BTreeMap::new();
    for file in &py_files {
        let original =
            fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
        let (updated, count) =
            rewrite_imports_for_move(file, &from_abs, &old_mod, &new_mod, &original);
        if count > 0 && updated != original {
            validate_python_syntax(file, &updated)?;
            planned.insert(file.clone(), (original, updated, count));
        }
    }

    if mode == RefactorMode::Apply {
        if let Some(parent) = to_abs.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create directory {}", parent.display()))?;
        }
        let moved_content = if let Some((_, updated, _)) = planned.remove(&from_abs) {
            updated
        } else {
            fs::read_to_string(&from_abs)
                .with_context(|| format!("cannot read {}", from_abs.display()))?
        };
        for (path, (_, updated, _)) in &planned {
            fs::write(path, updated).with_context(|| format!("cannot write {}", path.display()))?;
        }
        fs::write(&to_abs, moved_content)
            .with_context(|| format!("cannot write {}", to_abs.display()))?;
        fs::remove_file(&from_abs)
            .with_context(|| format!("cannot remove {}", from_abs.display()))?;
    }

    let mut changes = Vec::new();
    let mut total_rewrites = 0;
    for (path, (original, updated, count)) in &planned {
        let shown = display_rel(&root, path);
        let diff = unified_diff(&shown, original, updated);
        total_rewrites += *count;
        changes.push(FileChange {
            path: shown,
            rewrites: *count,
            diff,
        });
    }
    let report = RefactorReport {
        action: "move".to_owned(),
        applied: mode == RefactorMode::Apply,
        moved: Some((display_rel(&root, &from_abs), display_rel(&root, &to_abs))),
        removed: None,
        files_changed: changes.len(),
        total_rewrites,
        changes,
    };
    print_report(&report, format)
}

/// Rewrites every caller importing names from a pure re-export facade module `facade` so they
/// import directly from the defining target modules, validates every edited file with `libcst`,
/// and removes `facade` when `--apply` is set.
pub(super) fn inline_facade(
    project_root: &Path,
    facade: &Path,
    mode: RefactorMode,
    format: Format,
) -> Result<()> {
    let root = fs::canonicalize(project_root).or_else(|_| std::path::absolute(project_root))?;
    let facade_abs = fs::canonicalize(facade)
        .with_context(|| format!("facade module {} does not exist", facade.display()))?;
    ensure!(
        facade_abs.extension().and_then(|ext| ext.to_str()) == Some("py"),
        "facade {} must be a .py module",
        facade.display()
    );
    let facade_text = fs::read_to_string(&facade_abs)
        .with_context(|| format!("cannot read {}", facade_abs.display()))?;
    let facade_mod = imports::module_name(&facade_abs, Path::is_file).with_context(|| {
        format!(
            "cannot infer Python module name for {}",
            facade_abs.display()
        )
    })?;
    let reexports = extract_facade_reexports(&facade_abs, &facade_mod, &facade_text)?;
    ensure!(
        !reexports.is_empty(),
        "{} has no re-exported `from ... import ...` symbols to inline",
        facade.display()
    );

    let py_files = collect_python_files(&root);
    let mut planned: BTreeMap<PathBuf, (String, String, usize)> = BTreeMap::new();
    for file in &py_files {
        if *file == facade_abs {
            continue;
        }
        let original =
            fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
        let (updated, count) =
            rewrite_callers_for_facade(file, &facade_abs, &facade_mod, &reexports, &original)?;
        if count > 0 && updated != original {
            validate_python_syntax(file, &updated)?;
            planned.insert(file.clone(), (original, updated, count));
        }
    }

    if mode == RefactorMode::Apply {
        for (path, (_, updated, _)) in &planned {
            fs::write(path, updated).with_context(|| format!("cannot write {}", path.display()))?;
        }
        fs::remove_file(&facade_abs)
            .with_context(|| format!("cannot remove facade {}", facade_abs.display()))?;
    }

    let mut changes = Vec::new();
    let mut total_rewrites = 0;
    for (path, (original, updated, count)) in &planned {
        let shown = display_rel(&root, path);
        let diff = unified_diff(&shown, original, updated);
        total_rewrites += *count;
        changes.push(FileChange {
            path: shown,
            rewrites: *count,
            diff,
        });
    }
    let report = RefactorReport {
        action: "inline-facade".to_owned(),
        applied: mode == RefactorMode::Apply,
        moved: None,
        removed: Some(display_rel(&root, &facade_abs)),
        files_changed: changes.len(),
        total_rewrites,
        changes,
    };
    print_report(&report, format)
}

fn validate_python_syntax(path: &Path, source: &str) -> Result<()> {
    libcst_native::parse_module(source, None).map_err(|error| {
        anyhow::anyhow!(
            "rewritten {} failed libcst syntax validation: {error:?}",
            path.display()
        )
    })?;
    Ok(())
}

fn collect_python_files(root: &Path) -> Vec<PathBuf> {
    walk_code_files(&[root.to_path_buf()])
        .into_iter()
        .filter(|(_, lang)| *lang == SupportLang::Python)
        .filter_map(|(path, _)| fs::canonicalize(&path).ok().or(Some(path)))
        .collect()
}

fn infer_destination_module(root: &Path, from_abs: &Path, to_abs: &Path) -> Result<String> {
    let from_dir = from_abs.parent().unwrap_or(root);
    let mut package_top = from_dir;
    while package_top.join("__init__.py").is_file() {
        let Some(parent) = package_top.parent() else {
            break;
        };
        package_top = parent;
    }
    let relative = to_abs
        .strip_prefix(package_top)
        .or_else(|_| to_abs.strip_prefix(root))
        .with_context(|| {
            format!(
                "destination {} must lie under package root {}",
                to_abs.display(),
                package_top.display()
            )
        })?;
    let mut parts: Vec<String> = relative
        .components()
        .filter_map(|part| part.as_os_str().to_str().map(str::to_owned))
        .collect();
    let last = parts.pop().context("destination path has no file name")?;
    if last != "__init__.py" {
        let stem = Path::new(&last)
            .file_stem()
            .and_then(|s| s.to_str())
            .context("destination file has invalid stem")?;
        parts.push(stem.to_owned());
    }
    ensure!(
        !parts.is_empty(),
        "cannot derive module name from {}",
        to_abs.display()
    );
    Ok(parts.join("."))
}

fn rewrite_imports_for_move(
    file: &Path,
    from_abs: &Path,
    old_mod: &str,
    new_mod: &str,
    source: &str,
) -> (String, usize) {
    let tree = syntax::parse(source);
    let mut edits = Vec::new();
    let file_mod = imports::module_name(file, Path::is_file);
    let (old_parent, old_leaf) = old_mod
        .rsplit_once('.')
        .map_or(("", old_mod), |(parent, leaf)| (parent, leaf));
    let (new_parent, new_leaf) = new_mod
        .rsplit_once('.')
        .map_or(("", new_mod), |(parent, leaf)| (parent, leaf));

    let moved_dir_changed = old_parent != new_parent;
    let is_moved_file = file == from_abs;

    for node in tree.root().dfs() {
        match node.kind().as_ref() {
            "import_from_statement" => {
                let Some(module_node) = node.field("module_name") else {
                    continue;
                };
                let resolved = resolve_from_target(
                    file,
                    file_mod.as_deref(),
                    &module_node,
                    from_abs.parent().unwrap_or(Path::new("")),
                );
                if resolved.as_deref() == Some(old_mod) {
                    let range = module_node.range();
                    edits.push(ByteEdit {
                        start: range.start,
                        end: range.end,
                        replacement: new_mod.to_owned(),
                    });
                } else if !old_parent.is_empty() && resolved.as_deref() == Some(old_parent) {
                    let rel_mode = if is_moved_file && moved_dir_changed {
                        MovedRelativeMode::MakeAbsolute
                    } else {
                        MovedRelativeMode::KeepOriginal
                    };
                    rewrite_parent_from_import(
                        &node,
                        source,
                        &module_node,
                        (old_leaf, new_parent, new_leaf),
                        rel_mode,
                        &mut edits,
                    );
                } else if is_moved_file
                    && moved_dir_changed
                    && module_node.kind() == "relative_import"
                    && let Some(abs_target) = resolved
                {
                    let range = module_node.range();
                    edits.push(ByteEdit {
                        start: range.start,
                        end: range.end,
                        replacement: abs_target,
                    });
                }
            }
            "import_statement" => {
                for name_node in node.field_children("name") {
                    if name_node.kind() == "dotted_name" && name_node.text() == old_mod {
                        let range = name_node.range();
                        edits.push(ByteEdit {
                            start: range.start,
                            end: range.end,
                            replacement: new_mod.to_owned(),
                        });
                    } else if name_node.kind() == "aliased_import"
                        && let Some(target) = name_node.field("name")
                        && target.text() == old_mod
                    {
                        let range = target.range();
                        edits.push(ByteEdit {
                            start: range.start,
                            end: range.end,
                            replacement: new_mod.to_owned(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    let count = edits.len();
    (apply_byte_edits(source, edits), count)
}

#[derive(Clone, Copy)]
enum MovedRelativeMode {
    KeepOriginal,
    MakeAbsolute,
}

fn rewrite_parent_from_import(
    node: &Node<'_>,
    source: &str,
    module_node: &Node<'_>,
    moved: (&str, &str, &str),
    mode: MovedRelativeMode,
    edits: &mut Vec<ByteEdit>,
) {
    let (old_leaf, new_parent, new_leaf) = moved;
    let mut kept = Vec::new();
    let mut moved_line: Option<String> = None;
    for name_node in node.field_children("name") {
        let raw = name_node.text();
        let trimmed = raw.trim();
        let imported_name = trimmed.split(" as ").next().unwrap_or("").trim();
        if imported_name == old_leaf {
            let line = if new_parent.is_empty() {
                if let Some((_, alias)) = trimmed.split_once(" as ") {
                    format!("import {new_leaf} as {}", alias.trim())
                } else if new_leaf == old_leaf {
                    format!("import {new_leaf}")
                } else {
                    format!("import {new_leaf} as {old_leaf}")
                }
            } else if let Some((_, alias)) = trimmed.split_once(" as ") {
                format!("from {new_parent} import {new_leaf} as {}", alias.trim())
            } else if new_leaf == old_leaf {
                format!("from {new_parent} import {new_leaf}")
            } else {
                format!("from {new_parent} import {new_leaf} as {old_leaf}")
            };
            moved_line = Some(line);
        } else {
            kept.push(trimmed.to_owned());
        }
    }
    let Some(moved_stmt) = moved_line else {
        if matches!(mode, MovedRelativeMode::MakeAbsolute)
            && module_node.kind() == "relative_import"
        {
            let range = module_node.range();
            let resolved_parent = moved.1;
            if !resolved_parent.is_empty() {
                edits.push(ByteEdit {
                    start: range.start,
                    end: range.end,
                    replacement: resolved_parent.to_owned(),
                });
            }
        }
        return;
    };
    let stmt_range = node.range();
    let replacement = if kept.is_empty() {
        moved_stmt
    } else {
        let line_start = source[..stmt_range.start]
            .rfind('\n')
            .map_or(0, |pos| pos + 1);
        let indent = &source[line_start..stmt_range.start];
        format!(
            "from {} import {}\n{indent}{moved_stmt}",
            module_node.text(),
            kept.join(", ")
        )
    };
    edits.push(ByteEdit {
        start: stmt_range.start,
        end: stmt_range.end,
        replacement,
    });
}

fn resolve_from_target(
    file: &Path,
    file_mod: Option<&str>,
    module_node: &Node<'_>,
    _fallback_dir: &Path,
) -> Option<String> {
    if module_node.kind() != "relative_import" {
        return Some(module_node.text().into_owned());
    }
    let current = file_mod?;
    let is_init = file.file_name().and_then(|s| s.to_str()) == Some("__init__.py");
    let mut base_parts: Vec<&str> = current.split('.').collect();
    if !is_init {
        base_parts.pop();
    }
    let dots = module_node.child(0).map_or(1, |prefix| prefix.text().len());
    for _ in 1..dots {
        if base_parts.is_empty() {
            return None;
        }
        base_parts.pop();
    }
    let named = module_node
        .children()
        .find(|child| child.kind() == "dotted_name")
        .map(|node| node.text().into_owned());
    if let Some(tail) = named {
        if base_parts.is_empty() {
            Some(tail)
        } else {
            Some(format!("{}.{tail}", base_parts.join(".")))
        }
    } else if base_parts.is_empty() {
        None
    } else {
        Some(base_parts.join("."))
    }
}

/// Maps each name exported by a facade module to `(target_module, original_symbol_name)`.
fn extract_facade_reexports(
    facade_abs: &Path,
    facade_mod: &str,
    source: &str,
) -> Result<BTreeMap<String, (String, String)>> {
    let tree = syntax::parse(source);
    let mut map = BTreeMap::new();
    for stmt in tree.root().children() {
        match stmt.kind().as_ref() {
            "import_from_statement" => {
                let Some(module_node) = stmt.field("module_name") else {
                    continue;
                };
                let Some(target_mod) = resolve_from_target(
                    facade_abs,
                    Some(facade_mod),
                    &module_node,
                    facade_abs.parent().unwrap_or(Path::new("")),
                ) else {
                    continue;
                };
                for name_node in stmt.field_children("name") {
                    if name_node.kind() == "aliased_import" {
                        if let (Some(orig), Some(alias)) =
                            (name_node.field("name"), name_node.field("alias"))
                        {
                            map.insert(
                                alias.text().into_owned(),
                                (target_mod.clone(), orig.text().into_owned()),
                            );
                        }
                    } else if name_node.kind() == "dotted_name" {
                        let name = name_node.text().into_owned();
                        map.insert(name.clone(), (target_mod.clone(), name));
                    }
                }
            }
            "expression_statement" | "comment" | "future_import_statement" => {
                if stmt.kind() == "expression_statement" {
                    let is_allowed = stmt.child(0).is_some_and(|inner| {
                        inner.kind() == "string"
                            || (inner.kind() == "assignment"
                                && inner
                                    .field("left")
                                    .is_some_and(|lhs| lhs.text() == "__all__"))
                    });
                    ensure!(
                        is_allowed,
                        "{} defines logic beyond re-exports and __all__; refusing to inline",
                        facade_abs.display()
                    );
                }
            }
            "function_definition" | "class_definition" | "decorated_definition" => {
                bail!(
                    "{} defines functions or classes; it is not a pure facade module",
                    facade_abs.display()
                );
            }
            "pass_statement" => {}
            other => {
                bail!(
                    "{} contains `{other}` beyond re-exports and __all__; refusing to inline",
                    facade_abs.display()
                );
            }
        }
    }
    Ok(map)
}

fn rewrite_callers_for_facade(
    file: &Path,
    facade_abs: &Path,
    facade_mod: &str,
    reexports: &BTreeMap<String, (String, String)>,
    source: &str,
) -> Result<(String, usize)> {
    let tree = syntax::parse(source);
    let file_mod = imports::module_name(file, Path::is_file);
    let mut edits = Vec::new();

    for node in tree.root().dfs() {
        if node.kind() == "import_statement" {
            for name_node in node.field_children("name") {
                let target = if name_node.kind() == "aliased_import" {
                    name_node
                        .field("name")
                        .map(|n| n.text().into_owned())
                        .unwrap_or_default()
                } else {
                    name_node.text().into_owned()
                };
                if target == facade_mod {
                    bail!(
                        "{}: bare `import {facade_mod}` cannot be safely inlined automatically",
                        file.display()
                    );
                }
            }
            continue;
        }
        if node.kind() != "import_from_statement" {
            continue;
        }
        let Some(module_node) = node.field("module_name") else {
            continue;
        };
        let resolved = resolve_from_target(
            file,
            file_mod.as_deref(),
            &module_node,
            facade_abs.parent().unwrap_or(Path::new("")),
        );
        if resolved.as_deref() != Some(facade_mod) {
            continue;
        }
        let range = node.range();
        let line_start = source[..range.start].rfind('\n').map_or(0, |pos| pos + 1);
        let indent = &source[line_start..range.start];
        let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for name_node in node.field_children("name") {
            let (imported, bound) = if name_node.kind() == "aliased_import" {
                let Some(orig) = name_node.field("name") else {
                    continue;
                };
                let Some(alias) = name_node.field("alias") else {
                    continue;
                };
                (orig.text().into_owned(), alias.text().into_owned())
            } else {
                let name = name_node.text().into_owned();
                (name.clone(), name)
            };
            let Some((target_mod, orig_name)) = reexports.get(&imported) else {
                bail!(
                    "{}: `{imported}` is not a re-exported name of {facade_mod}",
                    file.display()
                );
            };
            let spec = if *orig_name == bound {
                orig_name.clone()
            } else {
                format!("{orig_name} as {bound}")
            };
            grouped.entry(target_mod.as_str()).or_default().push(spec);
        }
        if grouped.is_empty() {
            continue;
        }
        let mut lines = Vec::with_capacity(grouped.len());
        for (idx, (target_mod, items)) in grouped.into_iter().enumerate() {
            let prefix = if idx == 0 { "" } else { indent };
            lines.push(format!(
                "{prefix}from {target_mod} import {}",
                items.join(", ")
            ));
        }
        edits.push(ByteEdit {
            start: range.start,
            end: range.end,
            replacement: lines.join("\n"),
        });
    }

    let count = edits.len();
    Ok((apply_byte_edits(source, edits), count))
}

fn apply_byte_edits(source: &str, mut edits: Vec<ByteEdit>) -> String {
    if edits.is_empty() {
        return source.to_owned();
    }
    edits.sort_by(|a, b| b.start.cmp(&a.start).then_with(|| b.end.cmp(&a.end)));
    let mut seen = BTreeSet::new();
    let mut updated = source.to_owned();
    for edit in edits {
        if !seen.insert((edit.start, edit.end)) {
            continue;
        }
        updated.replace_range(edit.start..edit.end, &edit.replacement);
    }
    updated
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn unified_diff(label: &str, original: &str, updated: &str) -> String {
    TextDiff::from_lines(original, updated)
        .unified_diff()
        .header(&format!("a/{label}"), &format!("b/{label}"))
        .to_string()
}

fn print_report(report: &RefactorReport, format: Format) -> Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(report)?),
        Format::Text => {
            let verb = if report.applied { "Applied" } else { "Preview" };
            match (&report.moved, &report.removed) {
                (Some((from, to)), _) => {
                    println!(
                        "{verb} move `{from}` -> `{to}` ({} files, {} import rewrites)",
                        report.files_changed, report.total_rewrites
                    );
                }
                (_, Some(removed)) => {
                    println!(
                        "{verb} inline-facade `{removed}` ({} files, {} import rewrites)",
                        report.files_changed, report.total_rewrites
                    );
                }
                _ => {}
            }
            for change in &report.changes {
                print!("{}", change.diff);
            }
            if !report.applied {
                println!("Pass `--apply` to write these changes to disk.");
            }
        }
    }
    Ok(())
}
