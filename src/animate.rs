use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::{env, fs};

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};

const RUNNER: &str = include_str!("../py/animate.py");
const MEDIA: &str = include_str!("../py/animation_media.py");
const WORKER: &str = include_str!("../py/animation_worker.py");
const SEQUENCE: &str = include_str!("../skills/xpd-animate/examples/sequence.py");

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
    Preview,
    Final,
    Still,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Animation {
    /// Show the offline animation workflow or print an editable Manim example.
    Guide {
        #[arg(value_enum)]
        recipe: Option<Recipe>,
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

impl Animation {
    pub(crate) fn run(self) -> Result<ExitCode> {
        let python = env::current_exe()
            .and_then(fs::canonicalize)
            .ok()
            .map(|exe| exe.with_file_name("python"))
            .filter(|python| python.is_file())
            .unwrap_or_else(|| PathBuf::from("python3"));
        let mut child = Command::new(python);
        child.args(["-I", "-c", RUNNER, MEDIA, WORKER, SEQUENCE]);
        match self {
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
                return Ok(crate::cli::print_out(text));
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

#[cfg(unix)]
fn run(mut child: Command) -> Result<ExitCode> {
    use std::os::unix::process::CommandExt;

    if cfg!(not(all(target_os = "linux", target_arch = "x86_64"))) {
        anyhow::bail!("xpd animate currently needs Linux x86_64 and rootless Podman");
    }
    Err(child.exec()).context("xpd animate needs Python 3.10 or newer on PATH")
}

#[cfg(not(unix))]
fn run(_child: Command) -> Result<ExitCode> {
    anyhow::bail!("xpd animate currently needs Linux x86_64 and rootless Podman")
}
