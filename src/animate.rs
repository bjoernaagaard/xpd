use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::{env, fs};

use anyhow::{Context, Result, bail};
use clap::{Subcommand, ValueEnum};
use serde::Serialize;

use crate::index::{Entry, Index, Kind};
use crate::output::Format;

const RUNNER: &str = include_str!("../py/animate.py");
const MEDIA: &str = include_str!("../py/animation_media.py");
const WORKER: &str = include_str!("../py/animation_worker.py");
const SEQUENCE: &str = include_str!("../skills/xpd-animate/examples/sequence.py");
const MANIM_INDEX: &str = include_str!("../indexes/specialized/manim.json");

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Recipe {
    Sequence,
    Chart,
    Math,
    Geometry,
    Sorting,
    #[value(name = "three_d")]
    ThreeD,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Profile {
    Draft,
    Preview,
    Final,
    Still,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Animation {
    #[command(flatten)]
    Workflow(crate::animate_workflow::Workflow),
    /// Show the offline animation workflow or print an editable Manim example.
    Guide {
        #[arg(value_enum)]
        recipe: Option<Recipe>,
    },
    /// List Manim Module Index families or compare every sibling primitive in one family side by side.
    Catalog {
        /// Family module or suffix, such as `indication`, `creation`, `transform`, `graphing`, or `rate_functions`.
        family: Option<String>,
        /// Print machine-readable JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Check the installed rootless Podman runtime and pinned Manim image.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Download and verify the pinned Manim image once.
    Setup,
    /// Render a named Manim scene. Use "-" to read a JSON source bundle from stdin.
    Render {
        file: PathBuf,
        scene: String,
        /// Draft at 480p15, preview at 1080p60, final at 4K60, or a 720p still.
        #[arg(long, value_enum, default_value = "preview")]
        profile: Profile,
        /// Include an additional project-relative file, such as a helper, image or data file.
        #[arg(long)]
        asset: Vec<PathBuf>,
        /// Render again without reusing a finished artifact or cached segments.
        #[arg(long)]
        no_cache: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct FamilySummary {
    module: String,
    short_name: String,
    symbols: usize,
}

#[derive(Debug, Serialize)]
struct CatalogOverview {
    version: String,
    total_symbols: usize,
    families: Vec<FamilySummary>,
}

#[derive(Debug, Serialize)]
struct CatalogSymbol {
    name: String,
    defined_in: String,
    callable: String,
    parameters: Vec<String>,
    summary: String,
}

#[derive(Debug, Serialize)]
struct CatalogFamilyResult {
    version: String,
    query: String,
    modules: Vec<String>,
    symbols: Vec<CatalogSymbol>,
}

pub(crate) fn runner() -> Command {
    let python = env::current_exe()
        .and_then(fs::canonicalize)
        .ok()
        .map(|exe| exe.with_file_name("python"))
        .filter(|python| python.is_file())
        .unwrap_or_else(|| PathBuf::from("python3"));
    let mut child = Command::new(python);
    child.args(["-I", "-c", RUNNER, MEDIA, WORKER, SEQUENCE]);
    child
}

impl Animation {
    pub(crate) fn run(self) -> Result<ExitCode> {
        if !matches!(self, Self::Guide { .. } | Self::Catalog { .. })
            && !cfg!(all(target_os = "linux", target_arch = "x86_64"))
        {
            anyhow::bail!("xpd animate currently needs Linux x86_64 and rootless Podman");
        }
        let mut child = runner();
        match self {
            Self::Workflow(workflow) => return workflow.run(),
            Self::Catalog { family, json } => {
                return run_catalog(
                    family.as_deref(),
                    if json { Format::Json } else { Format::Text },
                );
            }
            Self::Guide { recipe } => {
                let text = match recipe {
                    None => include_str!("../skills/xpd-animate/SKILL.md"),
                    Some(Recipe::Sequence) => SEQUENCE,
                    Some(Recipe::Chart) => include_str!("../skills/xpd-animate/examples/chart.py"),
                    Some(Recipe::Math) => include_str!("../skills/xpd-animate/examples/math.py"),
                    Some(Recipe::Geometry) => {
                        include_str!("../skills/xpd-animate/examples/geometry.py")
                    }
                    Some(Recipe::Sorting) => {
                        include_str!("../skills/xpd-animate/examples/sorting.py")
                    }
                    Some(Recipe::ThreeD) => {
                        include_str!("../skills/xpd-animate/examples/three_d.py")
                    }
                };
                return Ok(crate::output::print_out(text));
            }
            Self::Doctor { json } => {
                child.arg("doctor");
                if json {
                    child.arg("--json");
                }
            }
            Self::Setup => {
                child.arg("setup");
            }
            Self::Render {
                file,
                scene,
                profile,
                asset,
                no_cache,
                json,
            } => {
                let profile = match profile {
                    Profile::Draft => "draft",
                    Profile::Preview => "preview",
                    Profile::Final => "final",
                    Profile::Still => "still",
                };
                child.args(["render", "--profile", profile]);
                if json {
                    child.arg("--json");
                }
                if no_cache {
                    child.arg("--no-cache");
                }
                for path in asset {
                    child.arg("--asset").arg(path);
                }
                child.arg("--").arg(file).arg(scene);
            }
        }
        run(child)
    }
}

fn entry_module(entry: &Entry) -> &str {
    entry.defined_in.as_deref().unwrap_or(&entry.package)
}

fn short_family_name(module: &str) -> &str {
    module.rsplit('.').next().unwrap_or(module)
}

fn first_line(text: &str) -> String {
    text.split("\n\n")
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn matches_family(module: &str, query: &str) -> bool {
    let module_lower = module.to_ascii_lowercase();
    let query_lower = query.trim().to_ascii_lowercase();
    if module_lower == query_lower {
        return true;
    }
    if module_lower.ends_with(&format!(".{query_lower}")) {
        return true;
    }
    module_lower
        .split('.')
        .any(|segment| segment == query_lower)
}

fn print_catalog_overview(
    version: String,
    total_symbols: usize,
    grouped: &BTreeMap<&str, Vec<&Entry>>,
    format: Format,
) -> Result<ExitCode> {
    let families: Vec<FamilySummary> = grouped
        .iter()
        .map(|(&module, entries)| FamilySummary {
            module: module.to_owned(),
            short_name: short_family_name(module).to_owned(),
            symbols: entries.len(),
        })
        .collect();
    let overview = CatalogOverview {
        version,
        total_symbols,
        families,
    };
    match format {
        Format::Json => println!("{}", serde_json::to_string(&overview)?),
        Format::Text => {
            println!(
                "Manim {} Module Index ({} symbols across {} modules):",
                overview.version,
                overview.total_symbols,
                overview.families.len()
            );
            for row in &overview.families {
                println!(
                    "  {:<42} {:>3} symbols  (query: {})",
                    row.module, row.symbols, row.short_name
                );
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_catalog(family: Option<&str>, format: Format) -> Result<ExitCode> {
    let index: Index =
        serde_json::from_str(MANIM_INDEX).context("cannot parse bundled Manim index")?;
    let mut grouped: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();
    for entry in index
        .entries
        .iter()
        .filter(|entry| entry.kind == Kind::Symbol)
    {
        grouped.entry(entry_module(entry)).or_default().push(entry);
    }

    let Some(query) = family else {
        return print_catalog_overview(index.version, index.entries.len(), &grouped, format);
    };

    let matched_modules: Vec<&str> = grouped
        .keys()
        .copied()
        .filter(|module| matches_family(module, query))
        .collect();
    if matched_modules.is_empty() {
        let suggestions: Vec<&str> = grouped
            .keys()
            .filter(|module| module.starts_with("manim.animation."))
            .map(|module| short_family_name(module))
            .collect();
        bail!(
            "no Manim module family matches `{query}`. Try one of: {}, or run `xpd animate catalog` to list all modules.",
            suggestions.join(", ")
        );
    }

    let mut symbols = Vec::new();
    for &module in &matched_modules {
        if let Some(entries) = grouped.get(module) {
            for entry in entries {
                symbols.push(CatalogSymbol {
                    name: entry.name.clone(),
                    defined_in: module.to_owned(),
                    callable: entry
                        .callable
                        .map_or_else(|| "symbol".to_owned(), |kind| kind.as_str().to_owned()),
                    parameters: entry.parameters_written.clone(),
                    summary: first_line(&entry.summary),
                });
            }
        }
    }
    let result = CatalogFamilyResult {
        version: index.version,
        query: query.to_owned(),
        modules: matched_modules.into_iter().map(str::to_owned).collect(),
        symbols,
    };
    match format {
        Format::Json => println!("{}", serde_json::to_string(&result)?),
        Format::Text => {
            println!(
                "Manim {} family `{}` ({} symbols across {}):",
                result.version,
                result.query,
                result.symbols.len(),
                result.modules.join(", ")
            );
            for sym in &result.symbols {
                let summary = if sym.summary.is_empty() {
                    "-"
                } else {
                    sym.summary.as_str()
                };
                println!("  {} ({}) [{}]", sym.name, sym.callable, sym.defined_in);
                println!("    {summary}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(unix)]
fn run(mut child: Command) -> Result<ExitCode> {
    use std::os::unix::process::CommandExt;

    Err(child.exec()).context("xpd animate needs Python 3.10 or newer on PATH")
}

#[cfg(not(unix))]
fn run(_child: Command) -> Result<ExitCode> {
    anyhow::bail!("xpd animate currently needs Linux x86_64 and rootless Podman")
}
