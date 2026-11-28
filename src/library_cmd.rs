//! CLI commands for managing layered project library indexes (`xpd library add/list/remove`).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Table, value};

use crate::catalog;
use crate::embed::Model;
use crate::index::{self, Index};
use crate::library_cache;
use crate::output::Format;
use crate::settings::Settings;
use crate::venv::Venv;

/// Manage local and third-party Python library indexes configured in `[tool.xpd.libraries]`.
#[derive(Debug, Subcommand)]
pub(crate) enum LibraryCommand {
    /// Index a third-party or local Python package without importing it and register it in `pyproject.toml`.
    Add {
        /// Python import name of the package, such as `pydantic` or `skpower`.
        package: String,
        /// Source root or `site-packages` folder holding the package. Default: auto-detected from local `src/`, project root, or `.venv`.
        #[arg(long)]
        source: Option<PathBuf>,
        /// Version to record in the index. Default: wheel metadata, `pyproject.toml`, or `0.0.0-local`.
        #[arg(long)]
        version: Option<String>,
        /// Output path for the index JSON. Default: `.xpd/indexes/<package>.json`.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Skip pre-computing local `EmbeddingGemma` vectors for the index.
        #[arg(long)]
        no_embed: bool,
        /// Also scaffold a minimal local rule pack folder (`rules/<package>/`) and `xpd.yml`/`sgconfig.yml` if absent.
        #[arg(long)]
        scaffold_rules: bool,
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// List configured project libraries and optionally bundled libraries.
    List {
        /// Include xpd's bundled libraries in the output.
        #[arg(long)]
        all: bool,
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
    /// Remove a configured library from `[tool.xpd.libraries]`.
    Remove {
        /// Python import name to remove from `[tool.xpd.libraries]`.
        package: String,
        /// Keep the index JSON file on disk instead of deleting `.xpd/indexes/<package>.json`.
        #[arg(long)]
        keep_index: bool,
        /// Output format. Default: text on a terminal, json otherwise.
        #[arg(long, value_enum)]
        format: Option<Format>,
    },
}

#[derive(Debug, Serialize)]
struct AddReport {
    package: String,
    version: String,
    source: PathBuf,
    index: PathBuf,
    pyproject: PathBuf,
    entries: usize,
    vectors_cached: usize,
    vectors_embedded: usize,
    scaffolded_rules: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct LibraryRow {
    package: String,
    source: String,
    version: String,
    entries: usize,
    vectors_cached: usize,
    index: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct RemoveReport {
    package: String,
    pyproject: PathBuf,
    removed_index: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmbedVectors {
    Compute,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleScaffold {
    Create,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListScope {
    ProjectOnly,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexDisposal {
    DeleteManaged,
    Keep,
}

impl LibraryCommand {
    pub(crate) fn run(self, default_format: Format) -> Result<ExitCode> {
        match self {
            Self::Add {
                package,
                source,
                version,
                output,
                no_embed,
                scaffold_rules,
                format,
            } => add_library(
                &package,
                source.as_deref(),
                version.as_deref(),
                output.as_deref(),
                if no_embed {
                    EmbedVectors::Skip
                } else {
                    EmbedVectors::Compute
                },
                if scaffold_rules {
                    RuleScaffold::Create
                } else {
                    RuleScaffold::Skip
                },
                format.unwrap_or(default_format),
            ),
            Self::List { all, format } => list_libraries(
                if all {
                    ListScope::All
                } else {
                    ListScope::ProjectOnly
                },
                format.unwrap_or(default_format),
            ),
            Self::Remove {
                package,
                keep_index,
                format,
            } => remove_library(
                &package,
                if keep_index {
                    IndexDisposal::Keep
                } else {
                    IndexDisposal::DeleteManaged
                },
                format.unwrap_or(default_format),
            ),
        }
    }
}

fn add_library(
    package: &str,
    source: Option<&Path>,
    version: Option<&str>,
    output: Option<&Path>,
    embed_vectors: EmbedVectors,
    rule_scaffold: RuleScaffold,
    format: Format,
) -> Result<ExitCode> {
    let cwd = env::current_dir().context("cannot determine current directory")?;
    let project_root = find_project_root(&cwd);
    let pyproject_path = project_root.join("pyproject.toml");
    let source_root = match source {
        Some(path) => resolve_explicit_source(path, package)?,
        None => detect_package_source(&project_root, package)?,
    };
    let resolved_version = match version {
        Some(ver) => ver.to_owned(),
        None => infer_version(&source_root, &project_root, package),
    };
    let output_path = match output {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => project_root.join(path),
        None => project_root
            .join(".xpd")
            .join("indexes")
            .join(format!("{package}.json")),
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create index folder {}", parent.display()))?;
    }
    let categories = catalog::current_categories()?;
    let mut built = index::generate(
        &source_root,
        package,
        &resolved_version,
        index::GriffeFallback::UvRun,
    )?;
    built.classify(&categories);
    let serialized = serde_json::to_vec(&built)?;
    fs::write(&output_path, &serialized)
        .with_context(|| format!("cannot write index {}", output_path.display()))?;

    let cache_key = index::file_cache_key(&output_path);
    let mut vectors = library_cache::load_cached_vectors(&cache_key, &built.entries);
    let vectors_embedded = if embed_vectors == EmbedVectors::Skip {
        0
    } else if let Some(model) = Model::open() {
        library_cache::ensure_cached_vectors(&cache_key, &built.entries, &mut vectors, &model)?
    } else {
        0
    };
    let vectors_cached = library_cache::cached_vector_count(&vectors);

    let stored_path = relative_or_absolute(&project_root, &output_path);
    record_library_in_pyproject(&pyproject_path, package, &stored_path)?;

    let scaffolded_rules = if rule_scaffold == RuleScaffold::Create {
        Some(scaffold_rule_pack(&project_root, package)?)
    } else {
        None
    };

    let report = AddReport {
        package: built.package,
        version: built.version,
        source: source_root,
        index: output_path,
        pyproject: pyproject_path,
        entries: built.entries.len(),
        vectors_cached,
        vectors_embedded,
        scaffolded_rules,
    };
    match format {
        Format::Json => println!("{}", serde_json::to_string(&report)?),
        Format::Text => {
            println!(
                "Added {} ({}) with {} symbols to {}",
                report.package,
                report.version,
                report.entries,
                report.index.display()
            );
            println!(
                "  Vectors cached: {}/{} (newly embedded: {})",
                report.vectors_cached, report.entries, report.vectors_embedded
            );
            println!("  Registered in: {}", report.pyproject.display());
            if let Some(pack_dir) = &report.scaffolded_rules {
                println!("  Rule pack: {}", pack_dir.display());
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn list_libraries(scope: ListScope, format: Format) -> Result<ExitCode> {
    let cwd = env::current_dir().context("cannot determine current directory")?;
    let settings = Settings::find(&cwd)?;
    let mut rows = Vec::new();
    for (package, path) in &settings.libraries {
        let bytes = fs::read(path)
            .with_context(|| format!("cannot read configured index {}", path.display()))?;
        let parsed: Index = serde_json::from_slice(&bytes)
            .with_context(|| format!("cannot parse configured index {}", path.display()))?;
        let cache_key = index::file_cache_key(path);
        let vectors = library_cache::load_cached_vectors(&cache_key, &parsed.entries);
        rows.push(LibraryRow {
            package: package.clone(),
            source: "project".to_owned(),
            version: parsed.version,
            entries: parsed.entries.len(),
            vectors_cached: library_cache::cached_vector_count(&vectors),
            index: Some(path.clone()),
        });
    }
    if scope == ListScope::All {
        for (package, entries) in index::bundled_package_counts() {
            rows.push(LibraryRow {
                package: package.to_owned(),
                source: "bundled".to_owned(),
                version: "bundled".to_owned(),
                entries,
                vectors_cached: entries,
                index: None,
            });
        }
    }
    match format {
        Format::Json => println!("{}", serde_json::to_string(&rows)?),
        Format::Text => {
            if rows.is_empty() {
                println!("No project libraries configured in [tool.xpd.libraries].");
                println!("Run `xpd library add <package>` to index a library.");
            } else {
                for row in &rows {
                    match &row.index {
                        Some(path) => println!(
                            "{} ({}, {} entries, {} vectors) -> {}",
                            row.package,
                            row.version,
                            row.entries,
                            row.vectors_cached,
                            path.display()
                        ),
                        None => println!("{} (bundled, {} entries)", row.package, row.entries),
                    }
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn remove_library(package: &str, disposal: IndexDisposal, format: Format) -> Result<ExitCode> {
    let cwd = env::current_dir().context("cannot determine current directory")?;
    let project_root = find_project_root(&cwd);
    let pyproject_path = project_root.join("pyproject.toml");
    ensure!(
        pyproject_path.is_file(),
        "no pyproject.toml found at {}",
        pyproject_path.display()
    );
    let raw = fs::read_to_string(&pyproject_path)
        .with_context(|| format!("cannot read {}", pyproject_path.display()))?;
    let mut doc: DocumentMut = raw
        .parse()
        .with_context(|| format!("cannot parse {}", pyproject_path.display()))?;
    let removed_raw = doc
        .get_mut("tool")
        .and_then(Item::as_table_mut)
        .and_then(|tool| tool.get_mut("xpd"))
        .and_then(Item::as_table_mut)
        .and_then(|xpd| xpd.get_mut("libraries"))
        .and_then(Item::as_table_mut)
        .and_then(|libs| libs.remove(package))
        .and_then(|item| item.as_str().map(str::to_owned));
    let Some(stored_path) = removed_raw else {
        bail!(
            "{} has no entry named {package} under [tool.xpd.libraries]",
            pyproject_path.display()
        );
    };
    fs::write(&pyproject_path, doc.to_string())
        .with_context(|| format!("cannot write {}", pyproject_path.display()))?;

    let mut removed_index = None;
    if disposal == IndexDisposal::DeleteManaged {
        let candidate = PathBuf::from(&stored_path);
        let full = if candidate.is_absolute() {
            candidate
        } else {
            project_root.join(candidate)
        };
        let managed_dir = project_root.join(".xpd").join("indexes");
        if full.starts_with(&managed_dir) && full.is_file() {
            fs::remove_file(&full)
                .with_context(|| format!("cannot remove index {}", full.display()))?;
            removed_index = Some(full);
        }
    }

    let report = RemoveReport {
        package: package.to_owned(),
        pyproject: pyproject_path,
        removed_index,
    };
    match format {
        Format::Json => println!("{}", serde_json::to_string(&report)?),
        Format::Text => {
            println!(
                "Removed {} from {}",
                report.package,
                report.pyproject.display()
            );
            if let Some(path) = &report.removed_index {
                println!("  Deleted {}", path.display());
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn find_project_root(start: &Path) -> PathBuf {
    let bound = crate::project::boundary();
    for folder in start
        .ancestors()
        .take_while(|dir| bound.as_deref().is_none_or(|root| dir.starts_with(root)))
    {
        if folder.join("pyproject.toml").is_file() {
            return folder.to_path_buf();
        }
    }
    start.to_path_buf()
}

fn contains_package(root: &Path, package: &str) -> bool {
    root.join(package).join("__init__.py").is_file()
        || root.join(format!("{package}.py")).is_file()
        || root.join(format!("{package}.pyi")).is_file()
        || root.join(package).is_dir()
}

fn resolve_explicit_source(source: &Path, package: &str) -> Result<PathBuf> {
    let abs = fs::canonicalize(source).or_else(|_| std::path::absolute(source))?;
    if contains_package(&abs, package) {
        return Ok(abs);
    }
    if abs.join("src").is_dir() && contains_package(&abs.join("src"), package) {
        return Ok(abs.join("src"));
    }
    if abs.file_name().and_then(|name| name.to_str()) == Some(package)
        && let Some(parent) = abs.parent()
    {
        return Ok(parent.to_path_buf());
    }
    bail!(
        "cannot locate package `{package}` under {}; pass the directory containing `{package}/` or `{package}.py`",
        abs.display()
    )
}

fn detect_package_source(project_root: &Path, package: &str) -> Result<PathBuf> {
    let src_dir = project_root.join("src");
    if src_dir.is_dir() && contains_package(&src_dir, package) {
        return Ok(src_dir);
    }
    if contains_package(project_root, package) {
        return Ok(project_root.to_path_buf());
    }
    if let Some(venv) = Venv::find_from(project_root) {
        for site in venv.site_dirs() {
            if site.is_dir() && contains_package(&site, package) {
                return Ok(site);
            }
        }
    }
    bail!(
        "cannot find package `{package}` in {}, src/, or the active virtual environment; pass --source <dir>",
        project_root.display()
    )
}

fn infer_version(source_root: &Path, project_root: &Path, package: &str) -> String {
    if let Ok(version) = index::source_version(source_root, package) {
        return version;
    }
    for candidate in [
        source_root.join("pyproject.toml"),
        source_root.parent().map_or_else(
            || project_root.join("pyproject.toml"),
            |p| p.join("pyproject.toml"),
        ),
        project_root.join("pyproject.toml"),
    ] {
        if let Ok(raw) = fs::read_to_string(&candidate)
            && let Ok(doc) = raw.parse::<DocumentMut>()
            && let Some(ver) = doc
                .get("project")
                .and_then(Item::as_table)
                .and_then(|proj| proj.get("version"))
                .and_then(Item::as_str)
            && !ver.trim().is_empty()
        {
            return ver.to_owned();
        }
    }
    "0.0.0-local".to_owned()
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn record_library_in_pyproject(
    pyproject_path: &Path,
    package: &str,
    index_path: &str,
) -> Result<()> {
    let existing = if pyproject_path.is_file() {
        fs::read_to_string(pyproject_path)
            .with_context(|| format!("cannot read {}", pyproject_path.display()))?
    } else {
        String::new()
    };
    let mut doc: DocumentMut = existing
        .parse()
        .with_context(|| format!("cannot parse {}", pyproject_path.display()))?;
    if !doc.contains_key("tool") {
        let mut table = Table::new();
        table.set_implicit(true);
        doc["tool"] = Item::Table(table);
    }
    let tool = doc["tool"]
        .as_table_mut()
        .with_context(|| format!("{}: [tool] is not a table", pyproject_path.display()))?;
    if !tool.contains_key("xpd") {
        let mut table = Table::new();
        table.set_implicit(true);
        tool["xpd"] = Item::Table(table);
    }
    let xpd = tool["xpd"]
        .as_table_mut()
        .with_context(|| format!("{}: [tool.xpd] is not a table", pyproject_path.display()))?;
    if !xpd.contains_key("libraries") {
        xpd["libraries"] = Item::Table(Table::new());
    }
    let libraries = xpd["libraries"].as_table_mut().with_context(|| {
        format!(
            "{}: [tool.xpd.libraries] is not a table",
            pyproject_path.display()
        )
    })?;
    libraries[package] = value(index_path);
    fs::write(pyproject_path, doc.to_string())
        .with_context(|| format!("cannot write {}", pyproject_path.display()))
}

fn scaffold_rule_pack(project_root: &Path, package: &str) -> Result<PathBuf> {
    let sgconfig = project_root.join("sgconfig.yml");
    if !sgconfig.exists() {
        fs::write(&sgconfig, "ruleDirs:\n  - rules\n")
            .with_context(|| format!("cannot write {}", sgconfig.display()))?;
    }
    let pack_dir = project_root.join("rules").join(package);
    fs::create_dir_all(&pack_dir)
        .with_context(|| format!("cannot create {}", pack_dir.display()))?;
    let xpd_yml = project_root.join("xpd.yml");
    let existing = if xpd_yml.exists() {
        fs::read_to_string(&xpd_yml)
            .with_context(|| format!("cannot read {}", xpd_yml.display()))?
    } else {
        "packs:\n".to_owned()
    };
    let entry_marker = format!("  {package}:");
    if !existing.contains(&entry_marker) {
        let mut updated = existing;
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        let _ = writeln!(updated, "  {package}:\n    dir: rules/{package}");
        fs::write(&xpd_yml, updated)
            .with_context(|| format!("cannot write {}", xpd_yml.display()))?;
    }
    Ok(pack_dir)
}
