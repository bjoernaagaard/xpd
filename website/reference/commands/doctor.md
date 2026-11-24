Run `xpd doctor` at the root of a Python project to compare it with the profile, the one target setup for Ruff, ty and pytest that ships with xpd. Doctor prints a unified diff of `pyproject.toml` with the reason for each change, the lock files the project needs, and then the conditional advice that the project's code points to. `xpd doctor --apply` writes the diff, and `xpd doctor --check` exits 1 while the project differs from the profile.

The plan reads files only. It compares the effect of each setting from the pinned releases' rules and defaults, so a rule that Ruff runs by default, or that a selector such as `ALL` turns on, needs no edit. Doctor runs the installed Ruff and ty only for the conditional advice, and uv only to check and write lock files.

## What the profile sets

The profile sets seven Ruff rules. A missing code goes into `lint.extend-select`, which keeps the project's own selection. An exact code in `lint.ignore` or in a per-file ignore goes, unless an exception records it.

| Code | What Ruff checks | What to do about a finding |
| --- | --- | --- |
| `PTH118` | Paths joined with `os.path.join` | Use a `Path` join where the caller accepts a path object. Preserve string or bytes contracts. |
| `PERF101` | A list created just to iterate over it | Iterate over the original iterable when a snapshot is unnecessary. |
| `PERF102` | A dictionary loop that discards the key or value | Iterate over the keys or values you actually use. |
| `PGH003` | A blanket `type: ignore` | Name the diagnostic you intend to suppress. |
| `PGH004` | A blanket `noqa` | Name the Ruff rule you intend to suppress. |
| `RUF017` | Lists combined with repeated addition | Flatten the inputs in one pass to avoid copying earlier items repeatedly. |
| `RUF100` | A `noqa` that no longer suppresses a finding | Remove the unused suppression. |

The profile sets three ty rules under `[tool.ty.rules]`. A higher level than the profile's needs no edit. Doctor removes a lower level that an override table sets.

| Rule | Level | Why enable it |
| --- | --- | --- |
| `blanket-ignore-comment` | `error` | A suppression without a diagnostic name can hide a new error on the same line. |
| `disjoint-cast` | `error` | Casting between types that cannot overlap makes a promise the value cannot satisfy. |
| `missing-type-argument` | `warn` | A bare generic such as `list` leaves the element type unspecified. |

The profile sets three pytest settings in the pytest table of `pyproject.toml`: `[tool.pytest]`, or `[tool.pytest.ini_options]` when the project already uses it.

| Setting | Where | What it changes |
| --- | --- | --- |
| `strict = true` | The pytest table | An unknown marker, an invalid setting, a repeated parameter id and an `xfail` test that passes each fail the run. |
| `pytest -ra` | `addopts` | The summary lists each skipped test and each expected failure. |
| `pytest --import-mode=importlib` | `addopts` | pytest imports a test file without adding its folder to `sys.path`. |

`strict` needs pytest 9.0. When the project's virtual environment has pytest 8, the profile sets `pytest --strict-markers` and `pytest --strict-config` in `addopts` and `xfail_strict = true` in the table instead.

A project that declares pytest gets two plugins in the dependency list that holds pytest, which is `dev` in `[dependency-groups]` when no list holds it.

| Plugin | Reason |
| --- | --- |
| `pytest-timeout` | A test that hangs fails and prints where it stopped. |
| `pytest-randomly` | The tests run in a new order each time, so a test that depends on another test fails. |

`pytest-timeout` stops no test until `timeout` is set. Doctor suggests the setting as conditional advice and leaves the number of seconds to you.

The profile removes these from every project, and no exception keeps them.

| In the project | Reason |
| --- | --- |
| `pytest-rerunfailures`, `flaky` | A rerun hides the run where the code was wrong. |
| `pytest-mock` | A test with a mock checks the mock and not the code. |
| `pytest --reruns` and `pytest --reruns-delay` in `addopts`, `reruns` and `reruns_delay` in the table | The same as a rerun plugin. |
| `strict_xfail = false` and `xfail_strict = false` | An `xfail` test that passes no longer fails the run. |

Each value has a reviewed record with its reason and source. [Read the profile](https://github.com/bjoernaagaard/xpd/blob/main/src/doctor/profile.toml) and [the records that explain it](https://github.com/bjoernaagaard/xpd/blob/main/src/doctor/decisions.jsonl).

## Example

The example project reruns each failed test twice.

```toml title="pyproject.toml"
[project]
name = "example"
version = "0.1.0"
requires-python = ">=3.12"

[dependency-groups]
dev = [
    "pytest>=9.2.0",
    "pytest-rerunfailures",
    "pytest-timeout",
]

[tool.pytest.ini_options]
addopts = "--reruns 2 --strict-markers"
```

Doctor prints the diff, then one entry for each change. The `...` lines stand for the entries left out here and for the conditional advice at the end.

```console
$ xpd doctor --format text
Profile revision 1, kind application, suggested because [project] has no [build-system], so uv runs it without building it
  A program that runs from its checkout or as an installed command, locked in uv.lock.
pyproject.toml records no profile yet.

--- pyproject.toml
+++ pyproject.toml
@@ -6,9 +6,22 @@
 [dependency-groups]
 dev = [
     "pytest>=9.2.0",
-    "pytest-rerunfailures",
     "pytest-timeout",
+    "pytest-randomly",
 ]
 
 [tool.pytest.ini_options]
-addopts = "--reruns 2 --strict-markers"
+addopts = "--strict-markers -ra --import-mode=importlib"
+strict = true
+
+[tool.ruff.lint]
+extend-select = ["PTH118", "PGH003", "PGH004"]
+
+[tool.ty.rules]
+blanket-ignore-comment = "error"
+disjoint-cast = "error"
+missing-type-argument = "warn"
+
+[tool.xpd.profile]
+revision = 1
+kind = "application"

Changes
...
  pytest --reruns: --reruns 2 -> removed, tool.pytest.ini_options.addopts at line 14
    Keep the first failure visible so you can find and fix its cause.
    Source: https://github.com/pytest-dev/pytest-rerunfailures/blob/master/README.rst
...
Locks
  uv.lock is missing. --apply runs uv lock.

xpd doctor --apply --kind application writes these changes.
...
```

Ruff 0.17.4 runs `PERF101`, `PERF102`, `RUF017` and `RUF100` by default, so the diff adds only the other three codes. `xpd doctor --apply --kind application` writes this diff, runs `uv lock`, and records the revision and kind last. A second `xpd doctor --check` then prints `The project matches profile revision 1.` and exits 0.

## Kinds of project

The first apply needs a kind, which `pyproject.toml` records. Doctor suggests one: `application` for a project with `[project.scripts]` or without `[build-system]`, `library` for one with `[build-system]`, and `script` when `pyproject.toml` has no `[project]` and a Python file has inline script metadata. Every kind gets the same Ruff, ty and pytest settings. The kinds differ in the lock files they need.

| Kind | What it is | Lock |
| --- | --- | --- |
| `library` | A package that others install. It keeps its build backend. | `uv.lock` |
| `application` | A program that runs from its checkout or as an installed command. | `uv.lock` |
| `pipeline` | An application that turns data into results. | `uv.lock` |
| `script` | Standalone scripts with inline script metadata. `pyproject.toml` holds only tool settings. | `<script>.lock` beside each script, by `uv lock --script` |

## One place for each tool

The profile keeps each tool's settings in its table of `pyproject.toml` at the project root. Doctor reports these as conflicts, which no edit of `pyproject.toml` resolves:

- a `ruff.toml` or `.ruff.toml` at the root or below it, or a `pyproject.toml` below the root with a `[tool.ruff]` table
- an `extend` key in `[tool.ruff]`
- a `ty.toml` at the root
- `pytest.toml`, `.pytest.toml`, `pytest.ini`, `.pytest.ini`, a `[pytest]` section in `tox.ini`, or a `[tool:pytest]` section in `setup.cfg`
- a per-file ignore that turns a rule of the profile off through a group selector, such as `PERF`
- a uv workspace, in the project or above it
- a profile revision that a newer xpd recorded

Move the settings into `pyproject.toml`, and run doctor again.

## Record a deliberate exception

A project records an exception in `pyproject.toml`, beside the revision and kind, with the reason it needs one.

```toml
[tool.xpd.profile]
revision = 1
kind = "application"

[[tool.xpd.profile.exceptions]]
tool = "ruff"
id = "PTH118"
files = ["scripts/**"]
reason = "The scripts hand str paths to a tool that rejects Path objects."
```

| `tool` | `id` | Scope and value | What doctor writes |
| --- | --- | --- | --- |
| `ruff` | A rule of the profile | `files` names file patterns. Without `files`, the exception covers the whole project. | A per-file ignore for each pattern, or the code in `lint.ignore` |
| `ty` | A rule of the profile | `value` names a level below the profile's. It covers the whole project. | The rule at that level in `[tool.ty.rules]` |
| `pytest` | A setting, argument or plugin that the profile adds | It covers the whole project. | Nothing. The profile stops requiring it. |

Doctor marks each exception `valid`, `invalid`, `redundant` when it changes nothing, or `orphaned` when the profile no longer sets its `id`. Any other mark than `valid` fails `--check` and stops `--apply` until you fix or delete the exception. A setting that differs from the profile without an exception is a change in the plan.

## Upgrades

Each xpd release holds one revision of the profile. When a newer xpd brings a new revision, doctor shows the change, and `--apply` writes it and records the new revision. A project whose recorded revision comes from a newer xpd gets a conflict, so an older xpd never writes over it.

## What `--apply` does

Apply plans first and writes nothing while the plan has a conflict, an exception that is not valid, or no kind. It then writes `pyproject.toml`, runs `uv lock` for a missing or out-of-date lock, checks that no drift remains, and records the revision and kind last. It never syncs an environment, installs Git hooks, runs project scripts or publishes.

Apply keeps the prior content of each file it touches in the folder `.xpd-apply` at the project root while it runs. When a lock refresh or the final check fails, apply puts every file back and exits 2. When an apply stops before it ends, the next one restores the files from that folder first.

`--check` runs `uv lock --check` for each lock that exists, so it needs uv. Plain `xpd doctor` runs no uv.

## Stop hook

In a project whose `pyproject.toml` has a `[tool.xpd.profile]` table, [`xpd hook`](../hook/) compares the project with its profile when an agent session begins and when a turn ends. It holds the turn for a difference that the session made, and it notes the differences that were there before. See the hook page.

## Conditional advice

Below the plan, doctor prints advice that depends on the project's code and choices, for Ruff 0.17.4, ty 0.0.106 and pytest 9.2.0. Doctor consults Ruff and ty in the project's environment or on `PATH`, and it gets the installed pytest version from the project's virtual environment. Advice for a missing tool is marked "unverified locally". Conditional advice names the decision you need to make and the evidence that raised it. For example, seeing scalar division is a reason to consider a ty warning. It does not establish that a particular divisor is zero.

| Project decision | Ruff rules or settings Doctor considers |
| --- | --- |
| Enforce typed API boundaries | `ANN401`, plus `allow-star-arg-any` and `ignore-fully-untyped` in the annotation settings. |
| Control when imports run | `PLC0415` for local imports, `TID254` and `TID255` for lazy imports, and `ban-lazy` for an explicit eager-import policy. |
| Keep annotations needed at runtime | `runtime-evaluated-base-classes` and `runtime-evaluated-decorators`, including Pydantic's `BaseModel` and `validate_call`. |
| Migrate numerical code | `NPY201` for NumPy 2 migrations and `PD011` for pandas code using `.values`. |
| Simplify loops or avoid unnecessary work | `PERF402` and `PERF403` for loops that copy items, `FURB192` for sorting to find an extreme, `RUF015` for selecting the first item, and `RUF065` for logging. |
| Maintain suppressions and preview policy | `RUF101` for obsolete rule codes, `RUF102` for unknown codes, and `lint.explicit-preview-rules` to opt into preview rules individually. |
| Diagnose an unexpected Ruff result | Doctor can suggest `ruff check --show-settings PATH` or `ruff check --show-files PATH` to inspect effective settings or included files. |

| Project decision | Conditional ty advice |
| --- | --- |
| Check arithmetic, conditional bindings or optional values | Consider `warn` for `division-by-zero`, `possibly-unresolved-reference` or `truthiness-test-of-none-union`, according to the code. |
| Protect typed boundaries | Consider `error` for `unsound-assignment`, `unsound-return` or `unsound-yield`. For decorators your project owns, consider `warn` for `dynamic-function-decorator-return`. |
| Check package metadata | Consider `missing-direct-dependency = "warn"` for packaged projects with complete dependency metadata that accept preview checks. |
| Change how ty analyzes the project | Consider disabling `respect-type-ignore-comments` when adopting ty-specific suppressions, enabling `strict-generic-narrowing` for generic container checks, or setting `return-type-inference` to `"always"` during gradual migration. |
| Run trusted standalone scripts with inline dependencies | Consider `project-manager.script = "uv"` with uv 0.12.3 or newer. |

The catalog also records which upstream defaults to leave in place and which settings inherit another decision. Each record identifies its release, reason and upstream source.

## JSON

For a machine-readable report, run:

```sh
xpd doctor --format json | jq '.profile.changes[] | {tool, id, path, old, new}'
```

`profile` holds the plan: `revision`, `applied`, `kind`, `changes`, `conflicts`, `exceptions`, `locks` and the `diff`. Read `tools` for version information, `suggestions` for conditional advice, `policies` for deliberate choices Doctor keeps, and `notes` for limits it encountered. `xpd doctor --check --format json` prints the plan alone, and `xpd doctor --apply --format json` prints what it wrote.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Doctor printed its report, `--check` found the project matches the profile, or `--apply` wrote the plan or had nothing to write. |
| 1 | `--check` found a difference from the profile: a value, a missing or out-of-date lock, a conflict, or an exception that is not valid. |
| 2 | Doctor could not read the project, or `--apply` wrote nothing because of a conflict, an exception that is not valid, a missing kind or a failed step. |
