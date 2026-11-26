//! Runs the real `xpd doctor` on disposable projects and checks what it plans and suggests,
//! record by record, beside what Ruff, ty and `xpd check` report on the same code.
//!
//! ```text
//! cargo build
//! cargo run --example doctor_cases -- target/debug/xpd --ruff <ruff 0.17.4> --ty <ty 0.0.106> \
//!     --uv <uv> [--newer-ruff <ruff 0.17.5>]
//! ```
//!
//! Each project lives in a new temporary folder. A case puts only the checkers it names on
//! `PATH`, and points `XDG_CONFIG_HOME` at an empty folder so that no user configuration leaks
//! in. Three grouped projects cover every profile value and conditional record of the catalog:
//! one where each applies, one where each is already in effect, and one without the evidence
//! that a conditional record needs. The profile cases run `--check` and `--apply` with uv on
//! `PATH`: drift, an apply, the empty plan after it, exceptions and a newer revision. A failure
//! names the case and the record.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use xpd::doctor::catalog::{self, Catalog, Decision, Kind, Target};

/// The Ruff settings of the profile whose changes name a rule or a selector instead of the
/// setting: `lint.ignore` and `lint.per-file-ignores`, and `lint.explicit-preview-rules`, which
/// the plan removes only where a project sets it.
const PROFILE_SELECTORS: [&str; 3] = [
    "lint.ignore",
    "lint.per-file-ignores",
    "lint.explicit-preview-rules",
];

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("doctor_cases: {error:#}");
            ExitCode::from(2)
        }
    }
}

/// Which records of the catalog a grouped case expects: the profile's values, which the plan
/// changes, or the conditional suggestions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wanted {
    Profile,
    Conditional,
}

/// The checkers a case puts on `PATH`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bin {
    Ruff,
    Ty,
    NewerRuff,
    Uv,
}

struct Runner {
    xpd: PathBuf,
    ruff: PathBuf,
    ty: PathBuf,
    uv: PathBuf,
    newer_ruff: Option<PathBuf>,
    scratch: PathBuf,
    catalog: Catalog,
    failures: Vec<String>,
    passed: usize,
}

fn run() -> Result<bool> {
    let mut args = std::env::args_os().skip(1);
    let mut xpd = None;
    let (mut ruff, mut ty, mut uv, mut newer_ruff) = (None, None, None, None);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .map(PathBuf::from)
                .context("a flag needs a path")
        };
        match arg.to_str() {
            Some("--ruff") => ruff = Some(value()?),
            Some("--ty") => ty = Some(value()?),
            Some("--uv") => uv = Some(value()?),
            Some("--newer-ruff") => newer_ruff = Some(value()?),
            _ => xpd = Some(PathBuf::from(arg)),
        }
    }
    let absolute = |path: PathBuf| {
        fs::canonicalize(&path).with_context(|| format!("no file {}", path.display()))
    };
    let scratch = std::env::temp_dir().join(format!("xpd-doctor-cases-{}", std::process::id()));
    fs::create_dir_all(&scratch)?;
    let mut runner = Runner {
        xpd: absolute(xpd.context(
            "usage: doctor_cases <xpd> --ruff <path> --ty <path> --uv <path> [--newer-ruff <path>]",
        )?)?,
        ruff: absolute(ruff.context("--ruff is required")?)?,
        ty: absolute(ty.context("--ty is required")?)?,
        uv: absolute(uv.context("--uv is required")?)?,
        newer_ruff: newer_ruff.map(absolute).transpose()?,
        scratch: scratch.clone(),
        catalog: Catalog::packaged()?,
        failures: Vec::new(),
        passed: 0,
    };
    let result = runner.all();
    fs::remove_dir_all(&scratch)?;
    result?;
    for failure in &runner.failures {
        println!("FAIL {failure}");
    }
    println!(
        "{} checks passed, {} failed",
        runner.passed,
        runner.failures.len()
    );
    Ok(runner.failures.is_empty())
}

/// The Python file in which every code trigger of the catalog finds evidence.
const EVERY_TRIGGER: &str = r#"# /// script
# dependencies = []
# ///
import json
import logging
import numpy as np
import pandas as pd
from json import loads
from typing import Any, Iterator
from pydantic import BaseModel, validate_call

def a(v: Any, *args: Any) -> int:
    return v

def b(payload: str) -> int:
    return loads(payload)

def g(v: Any) -> Iterator[int]:
    yield v

def h(v: Any) -> None:
    target: int = v
    other: int = json.loads("1")

def plain(x):
    import re
    return re

def copy(xs: list[int], pairs: list[tuple[int, int]], total: float, count: int) -> list[int]:
    out = []
    for x in xs:
        out.append(x)
    d = {}
    for k, v in pairs:
        d[k] = v
    first = list(xs)[0]
    low = sorted(xs)[0]
    logging.info("%s", str(xs))
    ratio = total / count
    return out  # noqa

def deco(f):
    def wrapper(*a):
        return f(*a)
    return wrapper

def maybe(flag: bool) -> int:
    if flag:
        value = 1
    return value

def count(n: int | None) -> int:
    if not n:
        return 0
    return n

def frame(df: pd.DataFrame):
    return df.values

class M(BaseModel):
    x: int

@validate_call
def v(x: int) -> int:
    return x  # ty: ignore[invalid-return-type]

def check(x: object) -> bool:
    return isinstance(x, (list, dict))

def fit(X: pd.DataFrame) -> pd.DataFrame:
    return X
"#;

impl Runner {
    fn all(&mut self) -> Result<()> {
        self.grouped()?;
        self.named()?;
        self.types()?;
        self.policies()?;
        self.configurations()?;
        self.versions()?;
        self.pytest_settings()?;
        self.pytest_plugins()?;
        self.profile()?;
        self.tools()?;
        self.reviewed()?;
        self.reviewed_again()?;
        Ok(())
    }

    fn pytest_settings(&mut self) -> Result<()> {
        for (name, config, expected) in [
            (
                "pytest-fresh",
                "[project]\nname = 'example'\nversion = '0.1.0'\n",
                &[
                    "pytest:--import-mode=importlib",
                    "pytest:-p no:doctest",
                    "pytest:-ra",
                    "pytest:filterwarnings",
                    "pytest:markers",
                    "pytest:strict",
                    "pytest:timeout",
                ][..],
            ),
            (
                "pytest-configured",
                "[tool.pytest]\nstrict = true\naddopts = ['-rA']\n",
                &[
                    "pytest:--import-mode=importlib",
                    "pytest:-p no:doctest",
                    "pytest:filterwarnings",
                    "pytest:markers",
                    "pytest:timeout",
                ],
            ),
            (
                "pytest-ini-options",
                "[tool.pytest.ini_options]\naddopts = '-q --strict-markers'\n",
                &[
                    "pytest:--import-mode=importlib",
                    "pytest:--strict-markers",
                    "pytest:-p no:doctest",
                    "pytest:-ra",
                    "pytest:filterwarnings",
                    "pytest:markers",
                    "pytest:strict",
                    "pytest:timeout",
                ],
            ),
        ] {
            let project = self.project(name, &[("pyproject.toml", config)])?;
            let report = self.doctor(&project, &[])?;
            let found: Vec<_> = suggested(&report)
                .into_iter()
                .filter(|id| id.starts_with("pytest:"))
                .collect();
            self.check(
                (found != expected).then(|| format!("{name}: pytest changes are {found:?}")),
            );
            let text = self.doctor_text(&project, &[])?;
            let expected_edit = match name {
                "pytest-fresh" => {
                    r#"+addopts = ["-ra", "--import-mode=importlib", "-p", "no:doctest"]"#
                }
                "pytest-configured" => {
                    r#"+addopts = ['-rA', "--import-mode=importlib", "-p", "no:doctest"]"#
                }
                _ => r#"+addopts = "-q -ra --import-mode=importlib -p no:doctest""#,
            };
            self.check(
                (!text.contains(expected_edit)).then(|| format!("{name}: missing {expected_edit}")),
            );
            self.check(
                (!text.contains(&format!(
                    "pytest not found; its advice is for pytest {} and unverified locally",
                    catalog::PYTEST_RELEASE
                )))
                .then(|| format!("{name}: pytest should be unverified locally")),
            );
        }
        Ok(())
    }

    fn pytest_plugins(&mut self) -> Result<()> {
        let remove = self.project(
            "pytest-remove",
            &[
                (
                    "pyproject.toml",
                    concat!(
                        "[dependency-groups]\n",
                        "dev = [\n",
                        "    'pytest>=9.2',\n",
                        "    'Pytest_RerunFailures>=16',\n",
                        "]\n",
                        "[project.optional-dependencies]\n",
                        "test = ['pytest-mock', 'flaky']\n",
                        "[tool.pytest.ini_options]\n",
                        "addopts = [\n",
                        "    '-q', '--reruns', '2',\n",
                        "    '--reruns-delay=0.5',\n",
                        "]\n",
                        "reruns = 2\n",
                        "reruns_delay = 0.5\n",
                        "strict_xfail = false\n",
                        "xfail_strict = 'false'\n",
                    ),
                ),
                ("uv.lock", ""),
            ],
        )?;
        let text = self.doctor_text(&remove, &[])?;
        self.expect_output(
            "pytest-remove",
            "doctor text",
            &text,
            &[
                "pytest --reruns: --reruns 2 -> removed, tool.pytest.ini_options.addopts at line 9",
                "pytest --reruns-delay: --reruns-delay=0.5 -> removed, tool.pytest.ini_options.addopts at line 9",
                "pytest reruns: 2 -> removed, tool.pytest.ini_options.reruns at line 13",
                "pytest reruns_delay: 0.5 -> removed, tool.pytest.ini_options.reruns_delay at line 14",
                "pytest strict_xfail: false -> removed, tool.pytest.ini_options.strict_xfail at line 15",
                "pytest xfail_strict: 'false' -> removed, tool.pytest.ini_options.xfail_strict at line 16",
                r#"pytest plugin pytest-rerunfailures: "Pytest_RerunFailures>=16" -> removed, dependency-groups.dev at line 2"#,
                r#"pytest plugin pytest-mock: "pytest-mock" -> removed, project.optional-dependencies.test at line 7"#,
                r#"pytest plugin flaky: "flaky" -> removed, project.optional-dependencies.test at line 7"#,
                "-    '-q', '--reruns', '2',\n-    '--reruns-delay=0.5',\n+    '-q',",
                "uv.lock is out of date. --apply runs uv lock.",
            ],
            &["uv add --dev"],
        );
        let report = self.doctor(&remove, &[])?;
        let removals = report["profile"]["changes"]
            .as_array()
            .context("the plan's changes")?
            .iter()
            .filter(|change| change["new"] == "removed")
            .count();
        self.check((removals != 9).then(|| format!("pytest-remove: {removals} JSON removals")));

        let timeout = self.project(
            "pytest-timeout",
            &[(
                "pyproject.toml",
                "[dependency-groups]\nqa = ['pytest', 'pytest-timeout']\n",
            )],
        )?;
        let text = self.doctor_text(&timeout, &[])?;
        self.expect_output(
            "pytest-timeout",
            "doctor text",
            &text,
            &[
                r#"pytest timeout: unset -> "60""#,
                r#"+qa = ['pytest', 'pytest-timeout', "pytest-randomly==5.1.0"]"#,
            ],
            &[
                "uv add --dev",
                "pytest plugin pytest-timeout",
                r#"timeout = "<seconds>""#,
            ],
        );
        Ok(())
    }

    /// Counts a passed check, or records its failure.
    fn check(&mut self, failure: Option<String>) {
        match failure {
            None => self.passed += 1,
            Some(failure) => self.failures.push(failure),
        }
    }

    /// A new project folder with `files`, each a path and its text.
    fn project(&self, name: &str, files: &[(&str, &str)]) -> Result<PathBuf> {
        let dir = self.scratch.join(name);
        for (path, text) in files {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().context("a file has a folder")?)?;
            fs::write(&path, text)?;
        }
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// A folder with only the checkers `bins`, for `PATH`.
    fn bin_dir(&self, bins: &[Bin]) -> Result<PathBuf> {
        let mut name = String::from("bin");
        for bin in bins {
            name.push(match bin {
                Bin::Ruff => 'r',
                Bin::Ty => 't',
                Bin::NewerRuff => 'n',
                Bin::Uv => 'u',
            });
        }
        let dir = self.scratch.join(name);
        if dir.is_dir() {
            return Ok(dir);
        }
        fs::create_dir_all(&dir)?;
        for bin in bins {
            let source = match bin {
                Bin::Ruff => &self.ruff,
                Bin::Ty => &self.ty,
                Bin::NewerRuff => self
                    .newer_ruff
                    .as_ref()
                    .context("--newer-ruff is required")?,
                Bin::Uv => &self.uv,
            };
            let target = dir.join(source.file_name().context("an executable has a name")?);
            if fs::hard_link(source, &target).is_err() {
                fs::copy(source, &target)?;
            }
        }
        Ok(dir)
    }

    fn command(&self, program: &Path, dir: &Path, bins: &[Bin]) -> Result<Command> {
        let mut command = Command::new(program);
        command
            .current_dir(dir)
            .env("PATH", self.bin_dir(bins)?)
            .env("XDG_CONFIG_HOME", self.scratch.join("config"))
            .env_remove("VIRTUAL_ENV");
        Ok(command)
    }

    /// The JSON report of `xpd doctor` in `dir`.
    fn doctor(&self, dir: &Path, bins: &[Bin]) -> Result<Value> {
        let output = self
            .command(&self.xpd, dir, bins)?
            .args(["doctor", "--format", "json"])
            .output()?;
        if !output.status.success() {
            bail!(
                "xpd doctor failed in {}: {}",
                dir.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// The exit code, standard output and standard error of `xpd doctor` with `args` in `dir`.
    fn doctor_run(&self, dir: &Path, bins: &[Bin], args: &[&str]) -> Result<(i32, String, String)> {
        let output = self
            .command(&self.xpd, dir, bins)?
            .arg("doctor")
            .args(args)
            .output()?;
        Ok((
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }

    fn doctor_text(&self, dir: &Path, bins: &[Bin]) -> Result<String> {
        let output = self
            .command(&self.xpd, dir, bins)?
            .args(["doctor", "--format", "text"])
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// The combined output of a checker run on `dir`, which never fixes anything.
    fn checker(&self, tool: &str, dir: &Path, args: &[&str]) -> Result<String> {
        let (program, mut fixed): (&Path, Vec<&str>) = match tool {
            "ruff" => (
                &self.ruff,
                vec!["check", "--no-fix", "--output-format", "concise"],
            ),
            "ty" => (&self.ty, vec!["check", "--output-format", "concise"]),
            _ => (&self.xpd, vec!["check", "--format", "text"]),
        };
        fixed.extend(args);
        let output = self
            .command(program, dir, &[Bin::Ruff, Bin::Ty])?
            .args(&fixed)
            .output()?;
        Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }

    /// Checks that `report` suggests each `tool:id` of `present` and none of `absent`.
    fn expect(&mut self, case: &str, report: &Value, present: &[&str], absent: &[&str]) {
        let found = suggested(report);
        for id in present {
            self.check(
                (!(found.contains(*id))).then(|| format!("{case}: expected a suggestion for {id}")),
            );
        }
        for id in absent {
            self.check(
                (found.contains(*id))
                    .then(|| format!("{case}: did not expect a suggestion for {id}")),
            );
        }
    }

    fn expect_output(
        &mut self,
        case: &str,
        what: &str,
        output: &str,
        contains: &[&str],
        lacks: &[&str],
    ) {
        for text in contains {
            self.check(
                (!(output.contains(text)))
                    .then(|| format!("{case}: {what} lacks {text:?}:\n{output}")),
            );
        }
        for text in lacks {
            self.check(
                (output.contains(text)).then(|| format!("{case}: {what} has {text:?}:\n{output}")),
            );
        }
    }

    /// The Ruff and ty records of the profile or the conditional suggestions, each as `tool:id`
    /// with its record key and whether it suggests a command. A Ruff rule of the profile is left
    /// out, since `lint.select` and `lint.ignore` decide it and the plan names the selector it
    /// changes, and so are the settings whose changes name a rule or a selector.
    fn actionable(&self, decision: Wanted) -> Vec<(String, String, bool)> {
        self.catalog
            .records()
            .iter()
            .filter(|record| record.key.tool != catalog::Tool::Pytest)
            .filter(|record| {
                !(record.key.tool == catalog::Tool::Ruff
                    && (record.key.kind == Kind::Rule
                        || PROFILE_SELECTORS.contains(&record.key.id.as_str())))
            })
            .filter_map(|record| {
                let (wanted, command) = match &record.decision {
                    Decision::Profile => (Wanted::Profile, false),
                    Decision::Suggest(suggestion) => (
                        Wanted::Conditional,
                        matches!(suggestion.target, Target::Command(_)),
                    ),
                    _ => return None,
                };
                (wanted == decision).then(|| {
                    (
                        format!("{}:{}", record.key.tool.name(), record.key.id),
                        record.key.to_string(),
                        command,
                    )
                })
            })
            .collect()
    }

    /// The configuration of a project where every profile value and conditional record of the
    /// catalog is in effect: the `pyproject.toml` that `xpd doctor --apply` writes, with each
    /// conditional setting added.
    fn effective_config(&self) -> Result<String> {
        let applied = self.project(
            "effective-applied",
            &[(
                "pyproject.toml",
                "[project]\nname = \"effective\"\nversion = \"0.1.0\"\ndescription = \"A project with every value in effect.\"\nreadme = \"README.md\"\nrequires-python = \">=3.12\"\ndependencies = [\"pydantic>=2\"]\n\n[project.urls]\nRepository = \"https://example.com/effective\"\n",
            )],
        )?;
        let (code, _, err) =
            self.doctor_run(&applied, &[Bin::Uv], &["--apply", "--kind", "application"])?;
        if code != 0 {
            bail!("effective: --apply exits {code}: {err}");
        }
        let mut doc: toml_edit::DocumentMut =
            fs::read_to_string(applied.join("pyproject.toml"))?.parse()?;
        for (_, suggestion) in self.catalog.suggestions() {
            let (path, value) = match &suggestion.target {
                Target::Setting { key, value, .. } => {
                    let prefix = if key.starts_with("lint.") || key.starts_with("format.") {
                        "tool.ruff"
                    } else {
                        "tool.ty"
                    };
                    (
                        format!("{prefix}.{key}"),
                        value.parse::<toml_edit::Value>()?,
                    )
                }
                Target::Level { key, level } => (
                    format!("tool.ty.{key}"),
                    toml_edit::Value::from(level.name()),
                ),
                Target::Select { .. } | Target::Command(_) => continue,
            };
            let parts: Vec<&str> = path.split('.').collect();
            let (key, tables) = parts.split_last().context("a setting has a key")?;
            let mut table: &mut dyn toml_edit::TableLike = doc.as_table_mut();
            for part in tables {
                if !table.contains_key(part) {
                    table.insert(part, toml_edit::Item::Table(toml_edit::Table::new()));
                }
                table = table
                    .get_mut(part)
                    .and_then(toml_edit::Item::as_table_like_mut)
                    .context("a table on the way to a setting")?;
            }
            table.insert(key, toml_edit::Item::Value(value));
        }
        Ok(doc.to_string())
    }

    /// The profile's values and the conditional records in a project where each applies, one
    /// where each is in effect, and one without evidence. The first project turns lint preview
    /// on itself, which the conditional records need, so `lint.preview` is no change there.
    fn grouped(&mut self) -> Result<()> {
        let activation = self.project("activation", &[
            ("pyproject.toml", "[project]\nname = \"activation\"\nversion = \"0.1.0\"\ndependencies = [\"pydantic\"]\n\n[tool.ruff.lint]\npreview = true\nselect = [\"E4\", \"TC\", \"N803\"]\n"),
            ("typed/ruff.toml", "extend = \"../pyproject.toml\"\n\n[lint]\nextend-select = [\"ANN401\", \"ANN001\"]\n\n[lint.flake8-annotations]\nallow-star-arg-any = true\nignore-fully-untyped = true\n"),
            ("typed/api.py", "def double(x: int) -> int:\n    return x * 2\n"),
            ("app.py", EVERY_TRIGGER),
            ("dist/generated.py", "VALUE = 1\n"),
        ])?;
        let report = self.doctor(&activation, &[Bin::Ruff, Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, _) in self
            .actionable(Wanted::Profile)
            .into_iter()
            .filter(|(id, _, _)| id != "ruff:lint.preview")
            .chain(self.actionable(Wanted::Conditional))
        {
            self.check(
                (!(found.contains(&id))).then(|| format!("activation: no suggestion for {key}")),
            );
        }

        let effective_config = self.effective_config()?;
        let effective = self.project("effective", &[
            ("pyproject.toml", &effective_config),
            ("typed/ruff.toml", "extend = \"../pyproject.toml\"\n\n[lint]\nextend-select = [\"ANN401\", \"ANN001\"]\n\n[lint.flake8-annotations]\nallow-star-arg-any = false\nignore-fully-untyped = false\n"),
            ("typed/api.py", "def double(x: int) -> int:\n    return x * 2\n"),
            ("app.py", EVERY_TRIGGER),
        ])?;
        let report = self.doctor(&effective, &[Bin::Ruff, Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, command) in self
            .actionable(Wanted::Profile)
            .into_iter()
            .chain(self.actionable(Wanted::Conditional))
        {
            if command {
                continue;
            }
            self.check(
                (found.contains(&id))
                    .then(|| format!("effective: {key} is suggested though it is in effect")),
            );
        }

        self.enabled_rules(&effective)?;

        let quiet = self.project("quiet", &[
            ("pyproject.toml", "[project]\nname = \"quiet\"\nversion = \"0.1.0\"\n\n[tool.ruff.lint]\nselect = [\"E4\"]\n"),
            ("app.py", "def add(x: int, y: int) -> int:\n    return x + y\n"),
        ])?;
        let report = self.doctor(&quiet, &[Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, _) in self.actionable(Wanted::Profile) {
            self.check(
                (!(found.contains(&id)))
                    .then(|| format!("quiet: no suggestion for the recommended {key}")),
            );
        }
        for (id, key, _) in self.actionable(Wanted::Conditional) {
            self.check(
                (found.contains(&id))
                    .then(|| format!("quiet: {key} is suggested without evidence")),
            );
        }
        Ok(())
    }

    /// The rules that Ruff runs in a project with the profile in effect are the rules whose
    /// profile record says they are on, so the catalog's reasons describe what Ruff does.
    fn enabled_rules(&mut self, project: &Path) -> Result<()> {
        let shown = self
            .command(&self.ruff, project, &[])?
            .args(["check", "--show-settings", "app.py"])
            .output()?;
        let shown = String::from_utf8_lossy(&shown.stdout);
        let enabled: BTreeSet<String> = shown
            .lines()
            .skip_while(|line| *line != "linter.rules.enabled = [")
            .skip(1)
            .take_while(|line| *line != "]")
            .map(|line| {
                let line = line.trim().trim_end_matches(',');
                line.rsplit_once(" (")
                    .map_or(line, |(_, code)| code.trim_end_matches(')'))
                    .to_owned()
            })
            .collect();
        let on: BTreeSet<String> = self
            .catalog
            .ruff_rules()
            .filter(|rule| {
                rule.decision == Decision::Profile && !rule.reason.starts_with("**Off**")
            })
            .map(|rule| rule.key.id.clone())
            .collect();
        let missing: Vec<&String> = on.difference(&enabled).collect();
        let extra: Vec<&String> = enabled.difference(&on).collect();
        self.check(
            (enabled.is_empty() || !missing.is_empty() || !extra.is_empty()).then(|| {
                format!("effective: Ruff runs {} rules; on in the catalog only: {missing:?}; run by Ruff only: {extra:?}", enabled.len())
            }),
        );
        Ok(())
    }

    fn named(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];

        let perf = self.project("perf", &[("app.py", "total = sum([[1], [2]], [])\n")])?;
        let ruff = self.checker("ruff", &perf, &["."])?;
        self.expect_output("performance", "ruff", &ruff, &["RUF017"], &[]);
        let report = self.doctor(&perf, &both)?;
        self.expect(
            "performance on defaults, which the profile selects with ALL",
            &report,
            &["ruff:lint.select"],
            &[],
        );
        fs::write(
            perf.join("pyproject.toml"),
            "[tool.ruff.lint]\nselect = [\"E4\"]\n",
        )?;
        let report = self.doctor(&perf, &both)?;
        self.expect(
            "performance with a narrow select",
            &report,
            &["ruff:lint.select"],
            &[],
        );

        let pandas = self.project("pandas", &[("app.py", "import pandas as pd\n\n\ndef clean(df: pd.DataFrame) -> None:\n    df.dropna(inplace=True)\n")])?;
        let xpd = self.checker("xpd", &pandas, &["."])?;
        self.expect_output(
            "pandas overlap",
            "xpd check",
            &xpd,
            &["pandas/no-inplace"],
            &[],
        );
        let report = self.doctor(&pandas, &both)?;
        self.expect("pandas overlap", &report, &[], &["ruff:PD002"]);

        let numpy = self.project(
            "numpy",
            &[("app.py", "import numpy as np\n\nx = np.random.rand(3)\n")],
        )?;
        let xpd = self.checker("xpd", &numpy, &["."])?;
        self.expect_output(
            "numpy overlap",
            "xpd check",
            &xpd,
            &["numpy/legacy-random"],
            &[],
        );
        let report = self.doctor(&numpy, &both)?;
        self.expect("numpy overlap", &report, &[], &["ruff:NPY002"]);

        let loop_ = "def scaled(items: list[int]) -> list[int]:\n    result = []\n    for item in items:\n        result.append(\n            item * 2\n            + 1\n        )\n    return result\n";
        let comprehension = "def scaled(items: list[int]) -> list[int]:\n    return [\n        item * 2\n        + 1\n        for item in items\n    ]\n";
        let conflict = self.project(
            "perf401",
            &[("app.py", loop_), ("rewritten.py", comprehension)],
        )?;
        let ruff = self.checker("ruff", &conflict, &["--select", "PERF401", "app.py"])?;
        self.expect_output(
            "readability conflict",
            "ruff PERF401",
            &ruff,
            &["PERF401"],
            &[],
        );
        let xpd = self.checker("xpd", &conflict, &["rewritten.py"])?;
        self.expect_output(
            "readability conflict",
            "xpd check",
            &xpd,
            &["python/comprehension-multi-line"],
            &[],
        );
        let report = self.doctor(&conflict, &both)?;
        self.expect(
            "readability conflict: the plan adds PERF401 to the ignore list",
            &report,
            &["ruff:PERF401"],
            &[],
        );

        Ok(())
    }

    /// Typed boundaries, casts and suppressions, beside what ty, Ruff and xpd report.
    fn types(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let boundary = self.project("boundary", &[("app.py", "import json\nfrom collections.abc import Iterator\nfrom typing import Any\n\n\ndef parse(payload: str) -> int:\n    return json.loads(payload)\n\n\ndef store(value: Any) -> None:\n    target: int = value\n    print(target)\n\n\ndef items(value: Any) -> Iterator[int]:\n    yield value\n")])?;
        let unsound = [
            "ty:unsound-return",
            "ty:unsound-assignment",
            "ty:unsound-yield",
        ];
        let report = self.doctor(&boundary, &both)?;
        self.expect("Any crossing a boundary", &report, &unsound, &[]);
        let text = self.doctor_text(&boundary, &both)?;
        self.expect_output(
            "Any crossing a boundary",
            "doctor text",
            &text,
            &["Parse the value with a schema", "leave the value unchecked"],
            &[],
        );
        fs::write(
            boundary.join("pyproject.toml"),
            "[tool.ty.rules]\nunsound-return = \"error\"\nunsound-assignment = \"error\"\nunsound-yield = \"error\"\n",
        )?;
        let ty = self.checker("ty", &boundary, &["."])?;
        self.expect_output(
            "Any crossing a boundary",
            "ty",
            &ty,
            &[
                "error[unsound-return]",
                "error[unsound-assignment]",
                "error[unsound-yield]",
            ],
            &[],
        );
        let report = self.doctor(&boundary, &both)?;
        self.expect(
            "Any crossing a boundary after adoption",
            &report,
            &[],
            &unsound,
        );

        let casts = self.project("casts", &[("app.py", "from typing import Any, cast\n\n\ndef name(count: int, raw: Any) -> str:\n    first = cast(str, count)\n    second = cast(str, raw)\n    return first + second\n"), ("narrowed.py", "def narrowed(raw: object) -> str:\n    if isinstance(raw, str):\n        return raw\n    raise TypeError(raw)\n")])?;
        let xpd = self.checker("xpd", &casts, &["."])?;
        self.expect_output(
            "casts",
            "xpd check",
            &xpd,
            &[
                "app.py:5:13: warning [python/typing-cast]",
                "app.py:6:14: warning [python/typing-cast]",
            ],
            &["narrowed.py"],
        );
        let report = self.doctor(&casts, &both)?;
        self.expect("casts", &report, &["ty:disjoint-cast"], &[]);
        fs::write(
            casts.join("pyproject.toml"),
            "[tool.ty.rules]\ndisjoint-cast = \"error\"\n",
        )?;
        let ty = self.checker("ty", &casts, &["."])?;
        self.expect_output(
            "casts",
            "ty",
            &ty,
            &["app.py:5:13: error[disjoint-cast]", "Found 1 diagnostic"],
            &["narrowed.py"],
        );

        let suppressions = self.project("suppressions", &[("app.py", "import os  # noqa\n\ncount: int = \"a\"  # type: ignore\nother: int = \"b\"  # ty: ignore\n")])?;
        let report = self.doctor(&suppressions, &both)?;
        self.expect(
            "suppressions",
            &report,
            &["ruff:lint.select", "ty:blanket-ignore-comment"],
            &["ruff:PGH003", "ruff:PGH004"],
        );
        fs::write(
            suppressions.join("pyproject.toml"),
            "[tool.ruff.lint]\nselect = [\"ALL\"]\n\n[tool.ty.rules]\nblanket-ignore-comment = \"error\"\n",
        )?;
        let ruff = self.checker("ruff", &suppressions, &["."])?;
        self.expect_output("suppressions", "ruff", &ruff, &["PGH004", "PGH003"], &[]);
        let ty = self.checker("ty", &suppressions, &["."])?;
        self.expect_output(
            "suppressions",
            "ty",
            &ty,
            &["error[blanket-ignore-comment]"],
            &[],
        );
        let report = self.doctor(&suppressions, &both)?;
        self.expect(
            "suppressions after adoption",
            &report,
            &[],
            &["ruff:lint.select", "ty:blanket-ignore-comment"],
        );

        Ok(())
    }

    /// Formatting, which the profile sets, and when imports must load, which the developer
    /// chooses.
    fn policies(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let format = self.project(
            "format",
            &[
                ("app.py", "x = 1\n"),
                (
                    "pyproject.toml",
                    "[tool.ruff.format]\nquote-style = \"single\"\n",
                ),
            ],
        )?;
        let report = self.doctor(&format, &both)?;
        self.expect(
            "formatting setting",
            &report,
            &["ruff:format.quote-style"],
            &[],
        );
        let text = self.doctor_text(&format, &both)?;
        self.expect_output(
            "formatting setting",
            "doctor text",
            &text,
            &["ruff format.quote-style: \"single\" -> \"double\", tool.ruff.format.quote-style at line 2"],
            &[],
        );

        let lazy = self.project("lazy", &[("app.py", "__lazy_modules__ = [\"json\"]\nimport json\n\nprint(json.dumps(1))\n"), ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py311\"\n\n[tool.ruff.lint]\nextend-select = [\"TID254\"]\n")])?;
        let report = self.doctor(&lazy, &both)?;
        self.expect(
            "lazy imports are the project's choice",
            &report,
            &[],
            &["ruff:lint.flake8-tidy-imports.ban-lazy", "ruff:TID254"],
        );

        let cells = self.project("cells", &[("explore.py", "# %%\nimport sys\n\nif len(sys.argv) > 1:\n    name = sys.argv[1]\n\n# %%\nprint(name)\n")])?;
        let report = self.doctor(&cells, &both)?;
        self.expect(
            "a cell script binds a name for a later cell",
            &report,
            &[],
            &["ty:possibly-unresolved-reference"],
        );
        fs::write(
            cells.join("module.py"),
            "def pick(flag: bool) -> int:\n    if flag:\n        value = 1\n    return value\n",
        )?;
        let report = self.doctor(&cells, &both)?;
        self.expect(
            "a module binds a name in one branch",
            &report,
            &["ty:possibly-unresolved-reference"],
            &[],
        );

        let estimator = self.project("estimator", &[("app.py", "class Scaler:\n    def fit(self, X: list[float]) -> \"Scaler\":\n        self.low = min(X)\n        return self\n"), ("pyproject.toml", "[tool.ruff.lint]\nextend-select = [\"N803\"]\n")])?;
        let report = self.doctor(&estimator, &both)?;
        self.expect(
            "a parameter named X",
            &report,
            &["ruff:lint.pep8-naming.extend-ignore-names"],
            &[],
        );
        let ruff = self.checker("ruff", &estimator, &["."])?;
        self.expect_output("a parameter named X", "ruff", &ruff, &["N803"], &[]);
        fs::write(
            estimator.join("pyproject.toml"),
            "[tool.ruff.lint]\nextend-select = [\"N803\"]\n\n[tool.ruff.lint.pep8-naming]\nextend-ignore-names = [\"X\"]\n",
        )?;
        let ruff = self.checker("ruff", &estimator, &["."])?;
        self.expect_output(
            "a parameter named X is allowed",
            "ruff",
            &ruff,
            &[],
            &["N803"],
        );
        let report = self.doctor(&estimator, &both)?;
        self.expect(
            "a parameter named X is allowed",
            &report,
            &[],
            &["ruff:lint.pep8-naming.extend-ignore-names"],
        );
        Ok(())
    }

    fn configurations(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR"));
        let report = self.doctor(checkout, &both)?;
        self.expect(
            "xpd checkout, which selects ALL",
            &report,
            &[],
            &["ruff:PTH118"],
        );

        self.nested()?;
        self.shadow()?;
        self.broken_and_missing()
    }

    /// A nested `ruff.toml` that extends its parent and ignores one rule for one file. The plan
    /// edits the root and names the nested file as a conflict.
    fn nested(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let nested = self.project("nested", &[
            ("pyproject.toml", "[project]\nname = \"nested\"\nversion = \"0.1.0\"\n\n[tool.ruff.lint]\nextend-select = [\"PTH118\"]\n"),
            ("app.py", "import os\n\nPATH = os.path.join(\"a\", \"b\")\n"),
            ("sub/ruff.toml", "extend = \"../pyproject.toml\"\n\n[lint.per-file-ignores]\n\"source.py\" = [\"PTH118\"]\n"),
            ("sub/source.py", "import os\n\nPATH = os.path.join(\"a\", \"b\")\n"),
        ])?;
        let shown = self
            .command(&self.ruff, &nested, &[])?
            .args(["check", "--show-settings", "sub/source.py"])
            .output()?;
        let shown = String::from_utf8_lossy(&shown.stdout);
        self.expect_output(
            "nested scope",
            "ruff --show-settings sub/source.py",
            &shown,
            &["os-path-join (PTH118)", "source.py"],
            &[],
        );
        let report = self.doctor(&nested, &both)?;
        self.expect(
            "nested scope",
            &report,
            &["ruff:lint.select"],
            &["ruff:PTH118"],
        );
        let paths: BTreeSet<&str> = report["profile"]["changes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["tool"] == "ruff" && c["id"] == "lint.select")
            .filter_map(|c| c["path"].as_str())
            .collect();
        self.check(
            (paths != BTreeSet::from(["tool.ruff.lint.select"]))
                .then(|| format!("nested scope: the selection goes to {paths:?}")),
        );
        let conflicts = report["profile"]["conflicts"].to_string();
        self.check(
            (!conflicts.contains("Ruff reads sub/ruff.toml for the files below it"))
                .then(|| format!("nested scope: no conflict for sub/ruff.toml in {conflicts}")),
        );
        let ruff = self.checker("ruff", &nested, &["."])?;
        self.expect_output(
            "nested scope",
            "ruff",
            &ruff,
            &["app.py:3:8: PTH118"],
            &["sub/source.py:3"],
        );

        Ok(())
    }

    /// A `ty.toml` beside a `pyproject.toml` with a `[tool.ty]` table, and an override. The
    /// plan names `ty.toml` as a conflict, and the advice reads it as ty does.
    fn shadow(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let shadow = self.project("shadow", &[
            ("pyproject.toml", "[project]\nname = \"shadow\"\nversion = \"0.1.0\"\n\n[tool.ty.rules]\ndisjoint-cast = \"error\"\n"),
            ("ty.toml", "[rules]\nunsound-return = \"error\"\n\n[[overrides]]\ninclude = [\"legacy/**\"]\n\n[overrides.rules]\nunsound-return = \"ignore\"\n"),
            ("app.py", "from typing import Any\n\n\ndef parse(value: Any) -> int:\n    return value\n"),
            ("legacy/old.py", "from typing import Any\n\n\ndef parse(value: Any) -> int:\n    return value\n"),
        ])?;
        let report = self.doctor(&shadow, &both)?;
        self.expect(
            "shadowing ty.toml",
            &report,
            &["ty:missing-type-argument"],
            &["ty:disjoint-cast", "ty:unsound-return"],
        );
        let conflicts = report["profile"]["conflicts"].to_string();
        self.check(
            (!conflicts.contains("ty reads ty.toml instead of pyproject.toml"))
                .then(|| format!("shadowing ty.toml: no conflict for ty.toml in {conflicts}")),
        );
        let notes = report["notes"].to_string();
        self.check(
            (!(notes.contains("ty.toml takes precedence")))
                .then(|| format!("shadowing ty.toml: no shadow note in {notes}")),
        );
        let policies = report["policies"].to_string();
        self.check(
            (!(policies.contains("unsound-return is ignore for legacy/**")))
                .then(|| format!("shadowing ty.toml: no override policy in {policies}")),
        );
        let app = self.checker("ty", &shadow, &["app.py"])?;
        self.expect_output(
            "shadowing ty.toml",
            "ty check app.py",
            &app,
            &["error[unsound-return]"],
            &[],
        );
        let legacy = self.checker("ty", &shadow, &["legacy/old.py"])?;
        self.expect_output(
            "shadowing ty.toml",
            "ty check legacy/old.py",
            &legacy,
            &["All checks passed"],
            &["unsound-return"],
        );

        Ok(())
    }

    /// Malformed TOML, and each combination of missing checkers.
    fn broken_and_missing(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];
        let broken = self.project(
            "broken",
            &[
                ("pyproject.toml", "[tool.ruff.lint\nselect = [\"E4\"]\n"),
                ("app.py", "x = 1\n"),
            ],
        )?;
        let (code, _, stderr) = self.doctor_run(&broken, &both, &["--format", "json"])?;
        self.check(
            (!(code == 2
                && stderr.contains("pyproject.toml")
                && stderr.contains("TOML parse error")))
            .then(|| format!("malformed TOML: exit {code}, {stderr}")),
        );

        let plain = self.project(
            "plain",
            &[
                (
                    "pyproject.toml",
                    "[project]\nname = \"plain\"\nversion = \"0.1.0\"\n",
                ),
                ("app.py", "import os\n\nPATH = os.path.join(\"a\", \"b\")\n"),
            ],
        )?;
        for (case, bins, ruff_verified, ty_verified) in [
            ("no checker", &[][..], false, false),
            ("only Ruff", &[Bin::Ruff][..], true, false),
            ("only ty", &[Bin::Ty][..], false, true),
        ] {
            let report = self.doctor(&plain, bins)?;
            for (tool, verified) in [("ruff", ruff_verified), ("ty", ty_verified)] {
                let installed = report["tools"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|status| status["tool"] == tool)
                    .map(|status| !status["installed"].is_null());
                self.check(
                    (installed != Some(verified))
                        .then(|| format!("{case}: {tool} found is {installed:?}")),
                );
            }
            self.expect(
                case,
                &report,
                &["ruff:lint.select", "ty:disjoint-cast"],
                &[],
            );
        }
        let text = self.doctor_text(&plain, &[])?;
        self.expect_output(
            "no checker",
            "doctor text",
            &text,
            &[
                &format!("ruff {}", catalog::RUFF_RELEASE),
                &format!("ty {}", catalog::TY_RELEASE),
                "unverified locally",
            ],
            &[],
        );
        Ok(())
    }

    fn versions(&mut self) -> Result<()> {
        if self.newer_ruff.is_none() {
            self.failures.push(
                "versions: --newer-ruff was not given, so the Ruff 0.17.5 case did not run"
                    .to_owned(),
            );
            return Ok(());
        }
        let plain = self.scratch.join("plain");
        let report = self.doctor(&plain, &[Bin::NewerRuff, Bin::Ty])?;
        let notes = report["notes"].to_string();
        self.expect_output(
            "ruff 0.17.5",
            "notes",
            &notes,
            &[
                "ASYNC401",
                "PLE0211",
                "RUF202",
                "ANN201 is missing-return-type-public-function in 0.17.5",
            ],
            &[],
        );
        self.expect("ruff 0.17.5", &report, &["ruff:lint.select"], &[]);
        let statuses: BTreeSet<&str> = report["suggestions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["tool"] == "ruff")
            .filter_map(|s| s["status"].as_str())
            .collect();
        let expected = format!(
            "verified with ruff 0.17.5 (decisions reviewed for {})",
            catalog::RUFF_RELEASE
        );
        self.check(
            (statuses != BTreeSet::from([expected.as_str()]))
                .then(|| format!("ruff 0.17.5: statuses are {statuses:?}")),
        );
        Ok(())
    }
}

impl Runner {
    /// Drift, an apply, the empty plan after it, the rule names that `ruff check --fix` writes in
    /// place of codes, exceptions, and a revision from a newer xpd. Ruff checks the scripts alone
    /// after the apply, since with preview on `RUF201` reports each rule code that the profile's
    /// selectors in `pyproject.toml` name.
    fn profile(&mut self) -> Result<()> {
        let bins = [Bin::Uv];
        let project = self.project(
            "profile",
            &[
                (
                    "pyproject.toml",
                    concat!(
                        "[project]\n",
                        "name = \"profiled\"\n",
                        "version = \"0.1.0\"\n",
                        "requires-python = \">=3.12\"\n",
                        "\n",
                        "[tool.ruff.lint]\n",
                        "ignore = [\"RUF100\"]\n",
                        "\n",
                        "[tool.ruff.lint.per-file-ignores]\n",
                        "\"scripts/*.py\" = [\"PTH118\"]\n",
                    ),
                ),
                (
                    "scripts/run.py",
                    "\"\"\"Build a path.\"\"\"\n\nimport os\n\nPATH = os.path.join(\"a\", \"b\")\n",
                ),
                ("scripts/__init__.py", ""),
            ],
        )?;
        let (code, out, _) = self.doctor_run(&project, &bins, &["--check", "--format", "text"])?;
        self.check((code != 1).then(|| format!("profile: --check on drift exits {code}:\n{out}")));
        self.expect_output(
            "profile drift",
            "doctor --check",
            &out,
            &[
                "-ignore = [\"RUF100\"]\n+ignore = [\"any-type\", \"complex-structure\",",
                "+select = [\"ALL\"]",
                "ruff RUF100 unused-noqa: off, by tool.ruff.lint.ignore = [\"RUF100\"] -> on",
                "ruff PTH118 os-path-join: off for scripts/*.py -> on",
                "uv.lock is missing. --apply runs uv lock.",
                "kind application, suggested because [project] has no [build-system]",
            ],
            &[],
        );
        let (code, _, err) = self.doctor_run(&project, &bins, &["--apply"])?;
        self.check(
            (!(code == 2 && err.contains("--kind application")))
                .then(|| format!("profile: apply without a kind exits {code}: {err}")),
        );

        let exception = concat!(
            "\n[[tool.xpd.profile.exceptions]]\n",
            "tool = \"ruff\"\n",
            "id = \"PTH118\"\n",
            "files = [\"scripts/*.py\"]\n",
            "reason = \"The scripts hand str paths to a tool that rejects Path objects.\"\n",
        );
        let file = project.join("pyproject.toml");
        let mut text = fs::read_to_string(&file)?;
        text.push_str(exception);
        fs::write(&file, &text)?;
        let args = ["--apply", "--kind", "application", "--format", "text"];
        let (code, out, err) = self.doctor_run(&project, &bins, &args)?;
        self.check((code != 0).then(|| format!("profile: apply exits {code}: {err}")));
        self.expect_output(
            "profile apply",
            "doctor --apply",
            &out,
            &[
                "Applied profile revision 2, kind application, to pyproject.toml.",
                "Ran uv lock, which wrote uv.lock.",
            ],
            &["off for scripts/*.py -> on"],
        );
        let written = fs::read_to_string(&file)?;
        self.expect_output(
            "profile apply",
            "pyproject.toml",
            &written,
            &[
                "\"scripts/*.py\" = [\"PTH118\"]",
                "[tool.xpd.profile]\nrevision = 2\nkind = \"application\"",
            ],
            &["ignore = [\"RUF100\"]"],
        );
        self.check(
            (!project.join("uv.lock").is_file())
                .then(|| "profile: apply wrote no uv.lock".to_owned()),
        );
        self.check(
            project
                .join(".xpd-apply")
                .exists()
                .then(|| "profile: apply left its journal".to_owned()),
        );
        let ruff = self.checker("ruff", &project, &["scripts"])?;
        self.expect_output(
            "profile apply",
            "ruff",
            &ruff,
            &["All checks passed"],
            &["PTH118"],
        );

        let (code, out, _) = self.doctor_run(&project, &bins, &["--check", "--format", "text"])?;
        self.check(
            (code != 0).then(|| format!("profile: --check after apply exits {code}:\n{out}")),
        );
        self.expect_output(
            "profile second plan",
            "doctor --check",
            &out,
            &[
                "The project matches profile revision 2.",
                "ruff PTH118 for scripts/*.py: valid",
            ],
            &["---"],
        );
        let (code, out, _) = self.doctor_run(&project, &bins, &["--apply", "--format", "text"])?;
        self.check(
            (!(code == 0 && out.contains("Nothing to apply")))
                .then(|| format!("profile: second apply exits {code}:\n{out}")),
        );
        let fixed = self
            .command(&self.ruff, &project, &[Bin::Ruff])?
            .args(["check", "--fix", "--select", "RUF201", "pyproject.toml"])
            .output()?;
        let written = fs::read_to_string(&file)?;
        self.check(
            (!(fixed.status.success() && written.contains("\"any-type\"")))
                .then(|| format!("profile: ruff check --fix left the codes\n{written}")),
        );
        let (code, out, _) = self.doctor_run(&project, &bins, &["--check", "--format", "text"])?;
        self.check(
            (code != 0).then(|| {
                format!("profile: --check after Ruff named the rules exits {code}:\n{out}")
            }),
        );

        let before = fs::read_to_string(&file)?;
        fs::write(
            &file,
            format!(
                "{before}\n[[tool.xpd.profile.exceptions]]\ntool = \"pytest\"\nid = \"pytest-mock\"\nreason = \"Mocks are fast.\"\n"
            ),
        )?;
        let (code, out, _) = self.doctor_run(&project, &bins, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1 && out.contains("pytest pytest-mock: invalid")))
                .then(|| format!("profile: a forbidden exception exits {code}:\n{out}")),
        );
        let (code, _, err) = self.doctor_run(&project, &bins, &["--apply"])?;
        self.check(
            (!(code == 2 && err.contains("no exception keeps it")))
                .then(|| format!("profile: apply with a forbidden exception exits {code}: {err}")),
        );

        fs::write(&file, before.replace("revision = 2", "revision = 99"))?;
        let (code, _, err) = self.doctor_run(&project, &bins, &["--apply"])?;
        self.check(
            (!(code == 2 && err.contains("records profile revision 99")))
                .then(|| format!("profile: apply on a newer revision exits {code}: {err}")),
        );
        Ok(())
    }
}

impl Runner {
    /// The faults an independent review found, each run the way it reported them: pytest strict
    /// options turned off or overridden, a pytest 8 requirement, a ty override that lowers `all`,
    /// a script kind without scripts, an invalid file pattern, an exception for another pytest
    /// release, plugin placement, a comment in a dependency list, the Stop baseline, and an
    /// ambiguous pytest layout.
    /// A project that holds only the settings of Ruff, ty and pytest: the apply leaves its build
    /// backend, its hooks and uv's settings alone, writes each rule of the ignore list by its
    /// name, so that the pinned Ruff has nothing to report on `pyproject.toml`, and the check
    /// that follows holds the same part.
    fn tools(&mut self) -> Result<()> {
        let head = "[project]\nname = \"scoped\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n\n[build-system]\nrequires = [\"uv_build>=0.9\"]\nbuild-backend = \"uv_build\"\n";
        let scoped = self.project(
            "scoped",
            &[
                ("pyproject.toml", head),
                (".pre-commit-config.yaml", "repos: []\n"),
                ("src/scoped/__init__.py", "\"\"\"A package.\"\"\"\n"),
            ],
        )?;
        let bins = [Bin::Ruff, Bin::Ty, Bin::Uv];
        let (code, _, err) = self.doctor_run(&scoped, &bins, &["--apply", "--kind", "library"])?;
        self.check(
            (code != 2)
                .then(|| format!("tools: the whole profile exits {code} on uv_build: {err}")),
        );
        let (code, _, err) = self.doctor_run(
            &scoped,
            &bins,
            &["--apply", "--kind", "library", "--tools", "ruff,ty,pytest"],
        )?;
        self.check((code != 0).then(|| format!("tools: --apply --tools exits {code}: {err}")));
        let written = fs::read_to_string(scoped.join("pyproject.toml"))?;
        self.expect_output(
            "tools",
            "pyproject.toml",
            &written,
            &[
                "tools = [\"ruff\", \"ty\", \"pytest\"]",
                "build-backend = \"uv_build\"",
                "\"unnecessary-assign\"",
                "\"ruff==0.17.4\"",
                "\"pytest==9.2.0\"",
            ],
            &["exclude-newer", "tombi==", "lefthook==", "\"RET504\""],
        );
        for (file, wanted) in [
            ("uv.lock", true),
            ("conftest.py", true),
            (".pre-commit-config.yaml", true),
            ("lefthook.yml", false),
            (".github/workflows/ci.yml", false),
            ("tombi.toml", false),
            (".python-version", false),
        ] {
            let exists = scoped.join(file).exists();
            self.check((exists != wanted).then(|| {
                format!(
                    "tools: {file} {} after the apply",
                    if exists { "exists" } else { "is missing" }
                )
            }));
        }
        let ruff = self.checker("ruff", &scoped, &["pyproject.toml"])?;
        self.expect_output(
            "tools",
            "ruff on pyproject.toml",
            &ruff,
            &["All checks passed"],
            &["RUF201", "rule-codes-in-selectors"],
        );
        let (code, out, _) = self.doctor_run(&scoped, &bins, &["--check", "--format", "text"])?;
        self.check(
            (code != 0).then(|| format!("tools: --check after the apply exits {code}:\n{out}")),
        );
        self.expect_output(
            "tools",
            "doctor --check",
            &out,
            &[
                "kind library, tools ruff, ty, pytest",
                "The project matches profile revision",
            ],
            &[],
        );
        Ok(())
    }

    fn reviewed(&mut self) -> Result<()> {
        self.reviewed_pytest()?;
        self.reviewed_layout()?;
        self.reviewed_lists()?;
        self.stop_baseline()
    }

    /// Strict options turned off or overridden, and a pytest 8 requirement.
    fn reviewed_pytest(&mut self) -> Result<()> {
        let uv = [Bin::Uv];
        let head =
            "[project]\nname = \"reviewed\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n";

        let strict = self.project(
            "strict-off",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\"pytest>=9\"]\n\n[tool.pytest]\nstrict = true\nstrict_markers = false\naddopts = [\"-ra\", \"--import-mode=importlib\"]\n"),
            )],
        )?;
        let (code, out, _) = self.doctor_run(&strict, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1 && out.contains("pytest strict_markers: false -> removed")))
                .then(|| format!("strict-off: --check exits {code}:\n{out}")),
        );

        let overridden = self.project(
            "overridden",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\"pytest>=9\"]\n\n[tool.pytest]\nstrict = true\naddopts = [\"-ra\", \"--import-mode=importlib\", \"-o\", \"strict=false\", \"-p\", \"no:randomly\"]\n"),
            )],
        )?;
        let (code, out, _) = self.doctor_run(&overridden, &uv, &["--check", "--format", "text"])?;
        self.expect_output(
            "overridden",
            "doctor --check",
            &out,
            &[
                "addopts overrides strict with -o strict=false",
                "addopts turns off the plugin randomly with -p no:randomly",
            ],
            &[],
        );
        self.check((code != 1).then(|| format!("overridden: --check exits {code}")));
        let (code, _, _) =
            self.doctor_run(&overridden, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 2).then(|| format!("overridden: --apply exits {code}")));

        let eight = self.project(
            "pytest-eight",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\"pytest==8.4.2\"]\n"),
            )],
        )?;
        let text = self.doctor_text(&eight, &uv)?;
        self.expect_output(
            "pytest 8 declared",
            "doctor text",
            &text,
            &[
                r#"pin pytest: "pytest==8.4.2" -> "pytest==9.2.0", dependency-groups.dev"#,
                "+[tool.pytest]\n",
                "+strict = true",
            ],
            &["+[tool.pytest.ini_options]", "+xfail_strict = true"],
        );
        let elsewhere = self.project(
            "pytest-eight-extra",
            &[(
                "pyproject.toml",
                &format!("{head}\n[project.optional-dependencies]\ntest = [\"pytest==8.4.2\"]\n"),
            )],
        )?;
        let (code, out, _) = self.doctor_run(&elsewhere, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1
                && out.contains("project.optional-dependencies.test requires \"pytest==8.4.2\", and the profile pins pytest==9.2.0")))
            .then(|| format!("pytest 8 in an extra: --check exits {code}:\n{out}")),
        );

        Ok(())
    }

    /// A ty override that lowers `all`, a script kind without scripts, and an invalid pattern.
    fn reviewed_layout(&mut self) -> Result<()> {
        let uv = [Bin::Uv];
        let head =
            "[project]\nname = \"reviewed\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n";
        let lowered = self.project(
            "ty-all",
            &[
                (
                    "pyproject.toml",
                    &format!("{head}\n[[tool.ty.overrides]]\ninclude = [\"sample.py\"]\n\n[tool.ty.overrides.rules]\nall = \"ignore\"\n"),
                ),
                ("sample.py", "x: int = \"a\"  # ty: ignore\n"),
            ],
        )?;
        let (code, _, err) =
            self.doctor_run(&lowered, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("ty-all: --apply exits {code}: {err}")));
        let ty = self.checker("ty", &lowered, &["sample.py"])?;
        self.expect_output("ty-all", "ty", &ty, &["error[blanket-ignore-comment]"], &[]);

        let script = self.project(
            "script-kind",
            &[("pyproject.toml", head), ("tool.py", "print(1)\n")],
        )?;
        let (code, _, err) = self.doctor_run(&script, &uv, &["--apply", "--kind", "script"])?;
        self.check(
            (!(code == 2
                && err.contains("keeps pyproject.toml to the [tool] table, and it has [project]")
                && err.contains("needs Python files with inline script metadata")))
            .then(|| format!("script-kind: --apply exits {code}: {err}")),
        );

        let glob = self.project(
            "bad-glob",
            &[(
                "pyproject.toml",
                &format!("{head}\n[[tool.xpd.profile.exceptions]]\ntool = \"ruff\"\nid = \"PTH118\"\nfiles = [\"[\"]\nreason = \"The scripts take str paths.\"\n"),
            )],
        )?;
        let (code, _, err) = self.doctor_run(&glob, &uv, &["--apply", "--kind", "application"])?;
        self.check(
            (!(code == 2 && err.contains("is no file pattern Ruff reads")))
                .then(|| format!("bad-glob: --apply exits {code}: {err}")),
        );

        Ok(())
    }

    /// An exception for another pytest release, plugin placement, a comment in a dependency list,
    /// and two pytest tables.
    fn reviewed_lists(&mut self) -> Result<()> {
        let uv = [Bin::Uv];
        let head =
            "[project]\nname = \"reviewed\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n";
        let inactive = self.project(
            "inactive",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\"pytest>=9\"]\n\n[[tool.xpd.profile.exceptions]]\ntool = \"pytest\"\nid = \"--strict-config\"\nreason = \"The config has a plugin's keys.\"\n"),
            )],
        )?;
        let (code, out, _) = self.doctor_run(&inactive, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1 && out.contains("pytest --strict-config: redundant")))
                .then(|| format!("inactive: --check exits {code}:\n{out}")),
        );
        let (code, _, _) =
            self.doctor_run(&inactive, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 2).then(|| format!("inactive: --apply exits {code}")));

        let runtime = self.project(
            "runtime-pytest",
            &[(
                "pyproject.toml",
                "[project]\nname = \"runtime\"\nversion = \"0.1.0\"\ndependencies = [\"pytest\"]\n",
            )],
        )?;
        let report = self.doctor(&runtime, &uv)?;
        let paths: BTreeSet<String> = report["profile"]["changes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["id"] == "pytest-timeout" || c["id"] == "pytest-randomly")
            .filter_map(|c| c["path"].as_str().map(str::to_owned))
            .collect();
        self.check(
            (paths
                != BTreeSet::from([
                    "dependency-groups.dev".to_owned(),
                    "project.dependencies".to_owned(),
                ]))
            .then(|| format!("runtime-pytest: plugins go to {paths:?}")),
        );

        let comment = self.project(
            "comment",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\n    \"pytest-rerunfailures\",\n    # Keep pytest with the dev dependencies.\n    \"pytest>=9\",\n]\n"),
            )],
        )?;
        let (code, _, err) =
            self.doctor_run(&comment, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("comment: --apply exits {code}: {err}")));

        let both = self.project(
            "both-tables",
            &[(
                "pyproject.toml",
                &format!("{head}\n[tool.pytest]\nstrict = true\n\n[tool.pytest.ini_options]\naddopts = \"-q\"\n"),
            )],
        )?;
        let (code, out, _) = self.doctor_run(&both, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 2
                && out.contains(
                    "[tool.pytest] and [tool.pytest.ini_options] both contain settings",
                )))
            .then(|| format!("both-tables: --check exits {code}:\n{out}")),
        );
        Ok(())
    }

    /// A Stop after an edit that only moves an invalid exception down one line holds nothing,
    /// and the session file records the profile revision of the baseline.
    fn stop_baseline(&mut self) -> Result<()> {
        let project = self.project(
            "stop-baseline",
            &[(
                "pyproject.toml",
                "[project]\nname = \"stop\"\nversion = \"0.1.0\"\n\n[tool.xpd.profile]\nrevision = 1\nkind = \"application\"\n\n[[tool.xpd.profile.exceptions]]\ntool = \"ruff\"\nid = \"PTH118\"\nreason = \"\"\n",
            )],
        )?;
        let cache = self.scratch.join("hook-cache");
        let hook = |event: &str| -> Result<String> {
            let mut child = self
                .command(&self.xpd, &project, &[])?
                .arg("hook")
                .env("XDG_CACHE_HOME", &cache)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()?;
            std::io::Write::write_all(
                child.stdin.as_mut().context("the hook takes stdin")?,
                event.as_bytes(),
            )?;
            let output = child.wait_with_output()?;
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        };
        let cwd = project.display().to_string();
        hook(&format!(
            r#"{{"hook_event_name":"SessionStart","session_id":"baseline","cwd":"{cwd}","source":"startup"}}"#
        ))?;
        let session = fs::read_to_string(cache.join("xpd").join("sessions").join("baseline"))?;
        let file = project.join("pyproject.toml");
        let text = fs::read_to_string(&file)?;
        fs::write(&file, format!("# Project metadata\n{text}"))?;
        let answer = hook(&format!(
            r#"{{"hook_event_name":"Stop","session_id":"baseline","cwd":"{cwd}"}}"#
        ))?;
        self.check(
            (!session.contains(r#""drift":{"revision":2"#))
                .then(|| format!("stop-baseline: the session file holds {session}")),
        );
        self.check(
            (answer.contains("\"block\"") || !answer.contains("did not make"))
                .then(|| format!("stop-baseline: Stop answered {answer}")),
        );
        Ok(())
    }
}

impl Runner {
    /// The faults a second review found: a Ruff release whose defaults differ, a pytest lock that
    /// the apply refreshes, plugin autoload turned off, a nested pytest file, positional
    /// arguments in `addopts`, an `addopts` that already complies, a script kind with a
    /// dependency table, and the permissions of `pyproject.toml`.
    fn reviewed_again(&mut self) -> Result<()> {
        let uv = [Bin::Uv];
        let head =
            "[project]\nname = \"again\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n";

        let old_ruff = self.project(
            "old-ruff",
            &[
                (
                    "pyproject.toml",
                    &format!("{head}\n[dependency-groups]\ndev = [\"ruff==0.9.10\"]\n"),
                ),
                ("example.py", "x = 1\n"),
            ],
        )?;
        let (code, _, err) =
            self.doctor_run(&old_ruff, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("old-ruff: --apply exits {code}: {err}")));
        let shown = self
            .command(&self.uv, &old_ruff, &uv)?
            .args([
                "run",
                "--quiet",
                "--locked",
                "ruff",
                "check",
                "--show-settings",
                "example.py",
            ])
            .output()?;
        let shown = String::from_utf8_lossy(&shown.stdout);
        for code in [
            "PTH118", "PERF101", "PERF102", "PGH003", "PGH004", "RUF017", "RUF100",
        ] {
            self.check(
                (!shown.contains(&format!("({code})")))
                    .then(|| format!("old-ruff: ruff 0.9.10 does not run {code}")),
            );
        }

        let stale = self.project(
            "stale-lock",
            &[(
                "pyproject.toml",
                &format!("{head}\n[dependency-groups]\ndev = [\"pytest==9.2.0\"]\n"),
            )],
        )?;
        let locked = self
            .command(&self.uv, &stale, &uv)?
            .args(["lock", "--quiet"])
            .status()?;
        self.check((!locked.success()).then(|| "stale-lock: uv lock failed".to_owned()));
        let file = stale.join("pyproject.toml");
        fs::write(
            &file,
            fs::read_to_string(&file)?.replace("pytest==9.2.0", "pytest==8.4.2"),
        )?;
        let (code, _, err) = self.doctor_run(&stale, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("stale-lock: --apply exits {code}: {err}")));
        let written = fs::read_to_string(&file)?;
        self.check(
            (!(written.contains("\"pytest==9.2.0\"")
                && written.contains("[tool.pytest]\nstrict = true")
                && !written.contains("xfail_strict")))
            .then(|| format!("stale-lock: the file is\n{written}")),
        );

        let autoload = self.project(
            "autoload",
            &[("pyproject.toml", &format!("{head}\n[dependency-groups]\ndev = [\"pytest==9.2.0\"]\n\n[tool.pytest]\naddopts = [\"--disable-plugin-autoload\"]\n"))],
        )?;
        let (code, out, _) = self.doctor_run(&autoload, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1
                && out.contains(
                    "--disable-plugin-autoload, so pytest-timeout and pytest-randomly do not run",
                )))
            .then(|| format!("autoload: --check exits {code}:\n{out}")),
        );

        let nested = self.project(
            "nested-pytest",
            &[
                ("pyproject.toml", head),
                (
                    "tests/pytest.ini",
                    "[pytest]\naddopts = --import-mode=prepend\n",
                ),
            ],
        )?;
        let (code, out, _) = self.doctor_run(&nested, &uv, &["--check", "--format", "text"])?;
        self.check(
            (!(code == 1 && out.contains("pytest reads tests/pytest.ini when it runs below it")))
                .then(|| format!("nested-pytest: --check exits {code}:\n{out}")),
        );
        self.reviewed_addopts(head)
    }

    /// Positional arguments after `--`, a compliant `addopts` with extra spaces, a script kind with
    /// a dependency table, and the permissions of `pyproject.toml`.
    fn reviewed_addopts(&mut self, head: &str) -> Result<()> {
        let uv = [Bin::Uv];
        let positional = self.project(
            "positional",
            &[(
                "pyproject.toml",
                &format!("{head}\n[tool.pytest.ini_options]\naddopts = \"-q -- tests\"\n"),
            )],
        )?;
        let (code, _, err) =
            self.doctor_run(&positional, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("positional: --apply exits {code}: {err}")));
        let written = fs::read_to_string(positional.join("pyproject.toml"))?;
        self.check(
            (!written
                .contains("addopts = \"-q -ra --import-mode=importlib -p no:doctest -- tests\""))
            .then(|| format!("positional: the file is\n{written}")),
        );

        let spaced = "addopts = \"  -ra    --import-mode=importlib  -p  no:doctest \"";
        let compliant = self.project(
            "spaced",
            &[(
                "pyproject.toml",
                &format!("{head}\n[tool.pytest.ini_options]\n{spaced}\nstrict = true\n"),
            )],
        )?;
        let (code, _, err) =
            self.doctor_run(&compliant, &uv, &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("spaced: --apply exits {code}: {err}")));
        let (code, out, _) = self.doctor_run(&compliant, &uv, &["--check", "--format", "text"])?;
        let written = fs::read_to_string(compliant.join("pyproject.toml"))?;
        self.check(
            (!(code == 0 && !out.contains("addopts") && written.contains(spaced)))
                .then(|| format!("spaced: --check exits {code}:\n{out}\n{written}")),
        );

        let script = self.project(
            "script-groups",
            &[
                ("pyproject.toml", "[dependency-groups]\ndev = [\"pytest==9.2.0\"]\n"),
                ("tool.py", "# /// script\n# requires-python = \">=3.12\"\n# dependencies = []\n# ///\nprint(1)\n"),
            ],
        )?;
        let (code, _, err) = self.doctor_run(&script, &uv, &["--apply", "--kind", "script"])?;
        self.check(
            (!(code == 2
                && err.contains(
                    "keeps pyproject.toml to the [tool] table, and it has [dependency-groups]",
                )))
            .then(|| format!("script-groups: --apply exits {code}: {err}")),
        );
        self.permissions(head)
    }

    /// Apply keeps the permissions of the `pyproject.toml` it replaces.
    #[cfg(unix)]
    fn permissions(&mut self, head: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let private = self.project("private", &[("pyproject.toml", head)])?;
        let file = private.join("pyproject.toml");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
        let (code, _, err) =
            self.doctor_run(&private, &[Bin::Uv], &["--apply", "--kind", "application"])?;
        self.check((code != 0).then(|| format!("private: --apply exits {code}: {err}")));
        let mode = fs::metadata(&file)?.permissions().mode() & 0o777;
        self.check((mode != 0o600).then(|| format!("private: the mode is {mode:o}")));
        Ok(())
    }

    #[cfg(not(unix))]
    fn permissions(&mut self, _head: &str) -> Result<()> {
        Ok(())
    }
}

/// Each suggestion of `report`, and each change of its plan but the profile record, as
/// `tool:id`.
fn suggested(report: &Value) -> BTreeSet<String> {
    let changes = report["profile"]["changes"]
        .as_array()
        .into_iter()
        .flatten();
    report["suggestions"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(changes)
        .filter(|s| s["tool"] != "xpd")
        .map(|s| {
            format!(
                "{}:{}",
                s["tool"].as_str().unwrap_or_default(),
                s["id"].as_str().unwrap_or_default()
            )
        })
        .collect()
}
