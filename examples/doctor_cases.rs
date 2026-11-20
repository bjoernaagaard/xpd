//! Runs the real `xpd doctor` on disposable projects and checks what it suggests, record by
//! record, beside what Ruff, ty and `xpd check` report on the same code.
//!
//! ```text
//! cargo build
//! cargo run --example doctor_cases -- target/debug/xpd --ruff <ruff 0.17.4> --ty <ty 0.0.106> \
//!     [--newer-ruff <ruff 0.17.5>]
//! ```
//!
//! Each project lives in a new temporary folder. A case puts only the checkers it names on
//! `PATH`, and points `XDG_CONFIG_HOME` at an empty folder so that no user configuration leaks
//! in. Three grouped projects cover every recommended and conditional record of the catalog:
//! one where each applies, one where each is already in effect, and one without the evidence
//! that a conditional record needs. A failure names the case and the record.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use xpd::doctor::catalog::{self, Catalog, Target, When};

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

/// Which suggestions of the catalog a grouped case expects.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wanted {
    Recommended,
    Conditional,
}

/// The checkers a case puts on `PATH`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bin {
    Ruff,
    Ty,
    NewerRuff,
}

struct Runner {
    xpd: PathBuf,
    ruff: PathBuf,
    ty: PathBuf,
    newer_ruff: Option<PathBuf>,
    scratch: PathBuf,
    catalog: Catalog,
    failures: Vec<String>,
    passed: usize,
}

fn run() -> Result<bool> {
    let mut args = std::env::args_os().skip(1);
    let mut xpd = None;
    let (mut ruff, mut ty, mut newer_ruff) = (None, None, None);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .map(PathBuf::from)
                .context("a flag needs a path")
        };
        match arg.to_str() {
            Some("--ruff") => ruff = Some(value()?),
            Some("--ty") => ty = Some(value()?),
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
            "usage: doctor_cases <xpd> --ruff <path> --ty <path> [--newer-ruff <path>]",
        )?)?,
        ruff: absolute(ruff.context("--ruff is required")?)?,
        ty: absolute(ty.context("--ty is required")?)?,
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

lazy import os
__lazy_modules__ = ["json"]

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
        Ok(())
    }

    fn pytest_settings(&mut self) -> Result<()> {
        for (name, config, expected) in [
            (
                "pytest-fresh",
                "[project]\nname = 'example'\nversion = '0.1.0'\n",
                &[
                    "pytest:--import-mode=importlib",
                    "pytest:-ra",
                    "pytest:strict",
                ][..],
            ),
            (
                "pytest-configured",
                "[tool.pytest]\nstrict = true\naddopts = ['-rA']\n",
                &["pytest:--import-mode=importlib"],
            ),
            (
                "pytest-ini-options",
                "[tool.pytest.ini_options]\naddopts = '-q --strict-markers'\n",
                &[
                    "pytest:--import-mode=importlib",
                    "pytest:-ra",
                    "pytest:strict",
                ],
            ),
        ] {
            let project = self.project(name, &[("pyproject.toml", config)])?;
            let report = self.doctor(&project, &[])?;
            let found: Vec<_> = suggested(&report)
                .into_iter()
                .filter(|id| id.starts_with("pytest:"))
                .collect();
            self.check((found != expected).then(|| format!("{name}: pytest advice is {found:?}")));
            let text = self.doctor_text(&project, &[])?;
            let expected_edit = match name {
                "pytest-fresh" => r#"addopts = ["-ra", "--import-mode=importlib"]"#,
                "pytest-configured" => {
                    r#"add "--import-mode=importlib" to the existing addopts array at line 3"#
                }
                _ => "append -ra --import-mode=importlib inside the addopts string at line 2",
            };
            self.check(
                (!text.contains(expected_edit)).then(|| format!("{name}: missing {expected_edit}")),
            );
            self.check(
                (!text.contains(&format!(
                    "unverified locally (decisions for pytest {})",
                    catalog::PYTEST_RELEASE
                )))
                .then(|| format!("{name}: pytest advice should say unverified locally")),
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
                "remove --reruns 2 from addopts at line 10",
                "remove --reruns-delay=0.5 from addopts at line 11",
                "remove reruns = 2 at line 13",
                "remove reruns_delay = 0.5 at line 14",
                "remove strict_xfail = false at line 15",
                "remove xfail_strict = 'false' at line 16",
                r#"remove "Pytest_RerunFailures>=16" from dev at line 4"#,
                r#"remove "pytest-mock" from test at line 7"#,
                r#"remove "flaky" from test at line 7"#,
                r#"add "pytest-timeout", "pytest-randomly" to the existing dev array at line 2"#,
                "uv add --dev pytest-timeout",
            ],
            &[],
        );
        let report = self.doctor(&remove, &[])?;
        let removals = report["suggestions"]
            .as_array()
            .context("doctor suggestions")?
            .iter()
            .filter(|suggestion| suggestion["decision"] == "remove")
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
                r#"timeout = "<seconds>""#,
                r#"add "pytest-randomly" to the existing qa array at line 2"#,
                "Evidence: pyproject.toml:2 pytest-timeout",
            ],
            &["uv add --dev", r#"add "pytest-timeout""#],
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

    /// The Ruff and ty recommendations, each as `tool:id` with its record key.
    fn actionable(&self, decision: Wanted) -> Vec<(String, String, bool)> {
        self.catalog
            .suggestions()
            .filter(|(record, suggestion)| {
                if record.key.tool == catalog::Tool::Pytest {
                    return false;
                }
                let conditional = matches!(suggestion.when, When::If { .. });
                conditional == (decision == Wanted::Conditional)
            })
            .map(|(record, suggestion)| {
                (
                    format!("{}:{}", record.key.tool.name(), record.key.id),
                    record.key.to_string(),
                    matches!(suggestion.target, Target::Command(_)),
                )
            })
            .collect()
    }

    fn grouped(&mut self) -> Result<()> {
        let activation = self.project("activation", &[
            ("pyproject.toml", "[project]\nname = \"activation\"\nversion = \"0.1.0\"\ndependencies = [\"pydantic\"]\n\n[tool.ruff.lint]\npreview = true\nselect = [\"E4\", \"TC\"]\n"),
            ("typed/ruff.toml", "extend = \"../pyproject.toml\"\n\n[lint]\nextend-select = [\"ANN401\", \"ANN001\"]\n\n[lint.flake8-annotations]\nallow-star-arg-any = true\nignore-fully-untyped = true\n"),
            ("typed/api.py", "def double(x: int) -> int:\n    return x * 2\n"),
            ("app.py", EVERY_TRIGGER),
            ("dist/generated.py", "VALUE = 1\n"),
        ])?;
        let report = self.doctor(&activation, &[Bin::Ruff, Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, _) in self
            .actionable(Wanted::Recommended)
            .into_iter()
            .chain(self.actionable(Wanted::Conditional))
        {
            self.check(
                (!(found.contains(&id))).then(|| format!("activation: no suggestion for {key}")),
            );
        }

        let codes: Vec<String> = self
            .catalog
            .suggestions()
            .filter_map(|(_, s)| match &s.target {
                Target::Select { code, .. } => Some(format!("\"{code}\"")),
                _ => None,
            })
            .collect();
        let levels: Vec<String> = self
            .catalog
            .suggestions()
            .filter_map(|(_, s)| match &s.target {
                Target::Level { key, level } => Some(format!(
                    "{} = \"{}\"",
                    key.trim_start_matches("rules."),
                    level.name()
                )),
                _ => None,
            })
            .collect();
        let effective_config = format!(
            "[project]\nname = \"effective\"\nversion = \"0.1.0\"\ndependencies = [\"pydantic\"]\n\n\
             [tool.ruff.lint]\npreview = true\nexplicit-preview-rules = true\nselect = [\"E4\", \"TC\", {}]\n\n\
             [tool.ruff.lint.flake8-tidy-imports]\nban-lazy = \"all\"\n\n\
             [tool.ruff.lint.flake8-type-checking]\nruntime-evaluated-base-classes = {{ \"pydantic.BaseModel\" = \"required\" }}\nruntime-evaluated-decorators = {{ \"pydantic.validate_call\" = \"required\" }}\n\n\
             [tool.ty.rules]\n{}\n\n\
             [tool.ty.analysis]\nrespect-type-ignore-comments = false\nreturn-type-inference = \"always\"\nstrict-generic-narrowing = true\n\n\
             [tool.ty.project-manager]\nscript = \"uv\"\n",
            codes.join(", "),
            levels.join("\n")
        );
        let effective = self.project("effective", &[
            ("pyproject.toml", &effective_config),
            ("typed/ruff.toml", "extend = \"../pyproject.toml\"\n\n[lint]\nextend-select = [\"ANN401\", \"ANN001\"]\n\n[lint.flake8-annotations]\nallow-star-arg-any = false\nignore-fully-untyped = false\n"),
            ("typed/api.py", "def double(x: int) -> int:\n    return x * 2\n"),
            ("app.py", EVERY_TRIGGER),
        ])?;
        let report = self.doctor(&effective, &[Bin::Ruff, Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, command) in self
            .actionable(Wanted::Recommended)
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

        let quiet = self.project("quiet", &[
            ("pyproject.toml", "[project]\nname = \"quiet\"\nversion = \"0.1.0\"\n\n[tool.ruff.lint]\nselect = [\"E4\"]\n"),
            ("app.py", "def add(x: int, y: int) -> int:\n    return x + y\n"),
        ])?;
        let report = self.doctor(&quiet, &[Bin::Ty])?;
        let found = suggested(&report);
        for (id, key, _) in self.actionable(Wanted::Recommended) {
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

    fn named(&mut self) -> Result<()> {
        let both = [Bin::Ruff, Bin::Ty];

        let perf = self.project("perf", &[("app.py", "total = sum([[1], [2]], [])\n")])?;
        let ruff = self.checker("ruff", &perf, &["."])?;
        self.expect_output("performance", "ruff", &ruff, &["RUF017"], &[]);
        let report = self.doctor(&perf, &both)?;
        self.expect("performance on defaults", &report, &[], &["ruff:RUF017"]);
        fs::write(perf.join("ruff.toml"), "[lint]\nselect = [\"E4\"]\n")?;
        let report = self.doctor(&perf, &both)?;
        self.expect(
            "performance with a narrow select",
            &report,
            &["ruff:RUF017"],
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
        self.expect("readability conflict", &report, &[], &["ruff:PERF401"]);

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
            &["ruff:PGH003", "ruff:PGH004", "ty:blanket-ignore-comment"],
            &[],
        );
        fs::write(
            suppressions.join("pyproject.toml"),
            "[tool.ruff.lint]\nextend-select = [\"PGH003\", \"PGH004\"]\n\n[tool.ty.rules]\nblanket-ignore-comment = \"error\"\n",
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
            &["ruff:PGH003", "ruff:PGH004", "ty:blanket-ignore-comment"],
        );

        Ok(())
    }

    /// Choices that belong to the developer: formatting, and when imports must load.
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
            "formatting preference",
            &report,
            &[],
            &["ruff:format.quote-style"],
        );
        self.check(
            (report.to_string().contains("quote-style"))
                .then(|| "formatting preference: the report names quote-style".to_owned()),
        );

        let lazy = self.project("lazy", &[("app.py", "__lazy_modules__ = [\"json\"]\nimport json\n\nprint(json.dumps(1))\n"), ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py311\"\n\n[tool.ruff.lint]\nextend-select = [\"TID254\"]\n")])?;
        let report = self.doctor(&lazy, &both)?;
        self.expect(
            "import policy",
            &report,
            &["ruff:lint.flake8-tidy-imports.ban-lazy"],
            &["ruff:TID254"],
        );
        let text = self.doctor_text(&lazy, &both)?;
        self.expect_output(
            "import policy",
            "doctor text",
            &text,
            &[
                "If this module must load eagerly",
                "app.py:1 __lazy_modules__",
            ],
            &[],
        );
        fs::write(
            lazy.join("pyproject.toml"),
            "[tool.ruff]\ntarget-version = \"py311\"\n\n[tool.ruff.lint]\nextend-select = [\"TID254\"]\n\n[tool.ruff.lint.flake8-tidy-imports]\nban-lazy = \"all\"\n",
        )?;
        let ruff = self.checker("ruff", &lazy, &["."])?;
        self.expect_output(
            "import policy",
            "ruff",
            &ruff,
            &["TID254 Use an eager import"],
            &[],
        );
        let report = self.doctor(&lazy, &both)?;
        self.expect(
            "import policy in effect",
            &report,
            &[],
            &["ruff:lint.flake8-tidy-imports.ban-lazy"],
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

    /// A nested `ruff.toml` that extends its parent and ignores one rule for one file.
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
        self.expect("nested scope", &report, &["ruff:PGH003"], &["ruff:PTH118"]);
        let destinations: BTreeSet<String> = report["suggestions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["tool"] == "ruff" && s["id"] == "PGH003")
            .map(|s| {
                format!(
                    "{} [{}] {}",
                    s["file"].as_str().unwrap_or_default(),
                    s["table"].as_str().unwrap_or_default(),
                    s["scope"].as_str().unwrap_or_default()
                )
            })
            .collect();
        self.check(
            (!(destinations.contains("pyproject.toml [tool.ruff.lint] pyproject.toml")
                && destinations.contains("sub/ruff.toml [] sub/ruff.toml")))
            .then(|| format!("nested scope: PGH003 destinations are {destinations:?}")),
        );
        let policies = report["policies"].to_string();
        self.check(
            (!(policies.contains("PTH118 runs, except where")
                && policies.contains("sub/ruff.toml")))
            .then(|| format!("nested scope: no per-file policy for PTH118 in {policies}")),
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

    /// A `ty.toml` beside a `pyproject.toml` with a `[tool.ty]` table, and an override.
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
            &["ty:disjoint-cast"],
            &["ty:unsound-return"],
        );
        let ty_files: BTreeSet<&str> = report["suggestions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["tool"] == "ty")
            .filter_map(|s| s["file"].as_str())
            .collect();
        self.check(
            (ty_files != BTreeSet::from(["ty.toml"]))
                .then(|| format!("shadowing ty.toml: ty suggestions go to {ty_files:?}")),
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
        let report = self.doctor(&broken, &both)?;
        let notes = report["notes"].to_string();
        self.check(
            (!(notes.contains("pyproject.toml")
                && notes.contains("cannot resolve Ruff's settings")))
            .then(|| format!("malformed TOML: Ruff's note is {notes}")),
        );
        self.check(
            (!(notes.contains("cannot read ty's settings") && notes.contains("TOML parse error")))
                .then(|| format!("malformed TOML: ty's note is {notes}")),
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
            for (tool, verified, pinned) in [
                ("ruff", ruff_verified, catalog::RUFF_RELEASE),
                ("ty", ty_verified, catalog::TY_RELEASE),
            ] {
                let statuses: BTreeSet<&str> = report["suggestions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|s| s["tool"] == tool)
                    .filter_map(|s| s["status"].as_str())
                    .collect();
                let expected = if verified {
                    format!("verified with {tool} {pinned} (decisions reviewed for {pinned})")
                } else {
                    format!("unverified locally (decisions for {tool} {pinned})")
                };
                self.check(
                    (statuses != BTreeSet::from([expected.as_str()]))
                        .then(|| format!("{case}: {tool} statuses are {statuses:?}")),
                );
            }
            self.expect(case, &report, &["ruff:PTH118", "ty:disjoint-cast"], &[]);
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
        self.expect("ruff 0.17.5", &report, &["ruff:PTH118"], &[]);
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

/// Each suggestion of `report` as `tool:id`.
fn suggested(report: &Value) -> BTreeSet<String> {
    report["suggestions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            format!(
                "{}:{}",
                s["tool"].as_str().unwrap_or_default(),
                s["id"].as_str().unwrap_or_default()
            )
        })
        .collect()
}
