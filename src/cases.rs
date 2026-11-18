//! Run rule examples through xpd's scanner, or the upstream structural harness.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, ensure};
use clap::{Args, ValueEnum};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{catalog::Catalog, cli::Format, pack, scan, venv::Venv};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub(crate) enum Engine {
    #[default]
    Xpd,
    Structural,
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    /// Run domain checks with xpd, or raw structural checks with ast-grep.
    #[arg(long, value_enum, default_value = "xpd")]
    engine: Engine,
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[arg(short, long)]
    test_dir: Option<PathBuf>,
    #[arg(long, default_value = "__snapshots__")]
    snapshot_dir: PathBuf,
    #[arg(short, long)]
    filter: Option<Regex>,
    #[arg(long, conflicts_with_all = ["update_all", "interactive"])]
    skip_snapshot_tests: bool,
    #[arg(short = 'U', long)]
    update_all: bool,
    #[arg(short, long, conflicts_with = "format")]
    interactive: bool,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    #[serde(default)]
    valid: Vec<String>,
    #[serde(default)]
    invalid: Vec<String>,
    /// A source filename for language inference and rule path filters.
    path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Snapshots {
    id: String,
    snapshots: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
struct Outcome {
    id: String,
    source: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<Value>,
}

pub(crate) fn run(options: &Options) -> Result<ExitCode> {
    if matches!(options.engine, Engine::Structural) {
        return structural(options);
    }
    ensure!(
        !options.interactive || io::stdin().is_terminal(),
        "--interactive needs a terminal"
    );
    let root = std::env::current_dir()?;
    let venv = Venv::find_from(
        options
            .config
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(&root),
    );
    let installed = venv.as_ref().map(Venv::distributions);
    let catalog = match &options.config {
        Some(config) => Catalog::load_config(config, installed.as_ref())?,
        None => Catalog::load(&root, installed.as_ref())?,
    };
    let directories = directories(options, &catalog)?;
    let source_root = catalog
        .project
        .as_ref()
        .map_or(root.as_path(), |project| project.root.as_path());
    let packs = &catalog.packs;
    let mut outcomes = Vec::new();
    for (directory, snapshot_dir) in directories {
        ensure!(
            !snapshot_dir.as_os_str().is_empty()
                && snapshot_dir
                    .components()
                    .all(|part| matches!(part, Component::Normal(_))),
            "snapshot directory must be a relative folder without parent components"
        );
        let mut files = Vec::new();
        for entry in ignore::WalkBuilder::new(&directory).build() {
            let entry = entry?;
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "yml" || ext == "yaml")
                && !entry
                    .path()
                    .ancestors()
                    .any(|parent| parent.ends_with(&snapshot_dir))
            {
                files.push(entry.into_path());
            }
        }
        files.sort();
        for path in files {
            let case: Case = serde_saphyr::from_str(&fs::read_to_string(&path)?)
                .with_context(|| format!("cannot read cases in {}", path.display()))?;
            if options
                .filter
                .as_ref()
                .is_some_and(|filter| !filter.is_match(&case.id))
            {
                continue;
            }
            outcomes.extend(run_case(
                options,
                packs,
                &case,
                &directory,
                &snapshot_dir,
                source_root,
            )?);
        }
    }
    ensure!(!outcomes.is_empty(), "no cases matched");
    let failed = outcomes
        .iter()
        .filter(|case| !matches!(case.status, "passed" | "updated"))
        .count();
    if matches!(options.format, Format::Json) {
        println!(
            "{}",
            serde_json::to_string(&json!({"cases": outcomes, "failed": failed}))?
        );
    } else {
        for case in &outcomes {
            println!("{}: {}", case.id, case.status);
        }
        println!("{} cases, {failed} failed", outcomes.len());
    }
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn run_case(
    options: &Options,
    packs: &[pack::Pack],
    case: &Case,
    path: &Path,
    snapshot_dir: &Path,
    root: &Path,
) -> Result<Vec<Outcome>> {
    let selected: Vec<_> = packs
        .iter()
        .flat_map(|pack| &pack.rules)
        .filter(|rule| {
            rule.id == case.id
                || rule.config.id == case.id
                || rule.declared_id == case.id
                || (!case.id.contains('/')
                    && rule.declared_id.rsplit('/').next() == Some(case.id.as_str()))
        })
        .collect();
    let rule = selected
        .first()
        .with_context(|| format!("no rule matches case id {}", case.id))?;
    let filename = source_path(rule, case)?;
    let disk = root.join(&filename);
    let snapshot_path = snapshot_file(path, snapshot_dir, &case.id)?;
    let mut snapshots: Snapshots = if snapshot_path.exists() {
        serde_saphyr::from_str(&fs::read_to_string(&snapshot_path)?)?
    } else {
        Snapshots {
            id: case.id.clone(),
            ..Snapshots::default()
        }
    };
    for snapshot in snapshots.snapshots.values_mut() {
        order_labels(snapshot);
    }
    ensure!(
        snapshots.id == case.id,
        "snapshot id differs from case id {}",
        case.id
    );
    let mut changed = false;
    let mut outcomes = Vec::new();
    for (source, invalid) in case
        .valid
        .iter()
        .map(|s| (s, false))
        .chain(case.invalid.iter().map(|s| (s, true)))
    {
        let found = scan::lint_source(
            scan::File {
                path: &filename,
                disk: Some(&disk),
            },
            source,
            packs,
        )
        .found
        .into_iter()
        .find(|found| {
            found
                .variant
                .as_ref()
                .is_some_and(|variant| selected.iter().any(|rule| rule.variant == *variant))
        });
        let actual = found.map(|found| snapshot(found, source));
        let status = if !invalid && actual.is_some() {
            "unexpected finding"
        } else if invalid && actual.is_none() {
            "missing finding"
        } else if !invalid
            || options.skip_snapshot_tests
            || snapshots.snapshots.get(source) == actual.as_ref()
        {
            "passed"
        } else if options.update_all
            || (options.interactive && accept(&case.id, source, actual.as_ref())?)
        {
            if let Some(value) = actual.as_ref() {
                snapshots.snapshots.insert(source.clone(), value.clone());
                changed = true;
            }
            "updated"
        } else {
            "snapshot mismatch"
        };
        outcomes.push(Outcome {
            id: case.id.clone(),
            source: source.clone(),
            status,
            actual,
        });
    }
    if changed {
        fs::create_dir_all(snapshot_path.parent().context("snapshot has no parent")?)?;
        fs::write(&snapshot_path, serde_saphyr::to_string(&snapshots)?)?;
    }
    Ok(outcomes)
}

fn snapshot(found: scan::Found, source: &str) -> Value {
    let labels: Vec<_> = found.labels.iter().map(|label| {
                let mut value = json!({ "source": &source[label.range.clone()], "style": label.style, "start": label.range.start, "end": label.range.end });
                if let Some(message) = &label.message { value["message"] = json!(message); }
                value
            }).collect();
    let mut result = json!({"labels": labels});
    if let Some(edit) = found.fix {
        let mut fixed = source.to_owned();
        fixed.replace_range(edit.range, &edit.text);
        result["fixed"] = Value::String(fixed);
    }
    order_labels(&mut result);
    result
}

fn source_path(rule: &crate::rule::Rule, case: &Case) -> Result<PathBuf> {
    let file_types = rule.config.language.file_types();
    let extension = file_types
        .definitions()
        .iter()
        .flat_map(ignore::types::FileTypeDef::globs)
        .find_map(|glob| glob.strip_prefix("*."))
        .context("rule language has no default extension; supply a case path")?;
    Ok(case
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("case.{extension}"))))
}

fn snapshot_file(directory: &Path, snapshot_dir: &Path, id: &str) -> Result<PathBuf> {
    let snapshot_path = directory.join(snapshot_dir).join(format!(
        "{}-snapshot.yml",
        id.replace(['/', '\\', ':'], "-")
    ));
    let case_root = directory.canonicalize()?;
    let existing = snapshot_path
        .ancestors()
        .find(|path| fs::symlink_metadata(path).is_ok())
        .context("snapshot path has no existing parent")?;
    ensure!(
        existing.canonicalize()?.starts_with(&case_root),
        "snapshot path leaves the case directory"
    );
    Ok(snapshot_path)
}

fn order_labels(snapshot: &mut Value) {
    if let Some(labels) = snapshot.get_mut("labels").and_then(Value::as_array_mut) {
        labels.sort_by(|left, right| {
            let key = |value: &Value| {
                (
                    value["start"].as_u64(),
                    value["end"].as_u64(),
                    value["style"].as_str().unwrap_or_default().to_owned(),
                    value["message"].as_str().unwrap_or_default().to_owned(),
                )
            };
            key(left).cmp(&key(right))
        });
    }
}

fn directories(options: &Options, catalog: &Catalog) -> Result<Vec<(PathBuf, PathBuf)>> {
    if let Some(directory) = &options.test_dir {
        return Ok(vec![(directory.clone(), options.snapshot_dir.clone())]);
    }
    let project = catalog
        .project
        .as_ref()
        .context("no sgconfig.yml found; supply --test-dir")?;
    Ok(project
        .test_configs
        .iter()
        .map(|entry| {
            (
                entry.test_dir.clone(),
                entry
                    .snapshot_dir
                    .clone()
                    .unwrap_or_else(|| options.snapshot_dir.clone()),
            )
        })
        .collect())
}

fn accept(id: &str, source: &str, actual: Option<&Value>) -> Result<bool> {
    eprintln!("{id}\n{source}\n{}", serde_json::to_string_pretty(&actual)?);
    eprint!("Accept snapshot? [y/N] ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().eq_ignore_ascii_case("y"))
}

fn structural(options: &Options) -> Result<ExitCode> {
    ensure!(
        matches!(options.format, Format::Text),
        "--format json is available with --engine xpd"
    );
    let mut args = vec!["xpd".to_owned(), "test".to_owned()];
    for (flag, path) in [
        ("--config", options.config.as_ref()),
        ("--test-dir", options.test_dir.as_ref()),
    ] {
        if let Some(path) = path {
            args.push(format!("{flag}={}", path.display()));
        }
    }
    args.push(format!("--snapshot-dir={}", options.snapshot_dir.display()));
    if let Some(filter) = &options.filter {
        args.push(format!("--filter={filter}"));
    }
    for (flag, enabled) in [
        ("--skip-snapshot-tests", options.skip_snapshot_tests),
        ("--update-all", options.update_all),
        ("--interactive", options.interactive),
    ] {
        if enabled {
            args.push(flag.to_owned());
        }
    }
    ast_grep::main_with_args(args.into_iter())
}
