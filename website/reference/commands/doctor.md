Run `xpd doctor` at the root of a Python project to compare it with the profile, the one target setup for Ruff, ty and pytest that ships with xpd. Doctor prints a unified diff of `pyproject.toml` with the reason for each change, the lock files the project needs, and then the conditional advice that the project's code points to. `xpd doctor --apply` writes the diff, and `xpd doctor --check` exits 1 while the project differs from the profile.

The plan reads files. It compares the effect of the project's own settings, so a selector such as `ALL` or `PERF` that turns a rule on needs no edit. A tool's default does not count, because another release of the tool can change it, so the project names each rule and level itself. Doctor runs uv to check and write lock files, the pinned Ruff to confirm that it accepts the settings, and the installed Ruff and ty for the conditional advice.

## What doctor shows of the project

Doctor shows only the settings it owns, so that its output never carries a token from another part of `pyproject.toml`. It owns `[tool.ruff]`, `[tool.ty]`, the pytest table, `[tool.xpd]`, `[dependency-groups]`, and `dependencies` and `optional-dependencies` of `[project]`.

- **The diff.** Each line outside those tables, such as a line of `[tool.uv]`, `[[tool.uv.index]]`, `[tool.uv.sources]` or `[tool.uv.pip]`, shows as `<hidden>`. A line shows only when everything on it belongs to those tables, so a line that also holds another setting is hidden, such as `tool = { uv = { ... }, ruff = { ... } }` or `tool.uv.index-url = "..."` beside `tool.ruff.lint...` lines. Table headers and blank lines stay, so the diff keeps its shape. A change on a hidden line shows in the list of changes and not in the diff.
- **URLs.** In every line doctor prints, a URL keeps its scheme, host and path, and its user and password, query and fragment become `****`, as in `https://****@example.com/simple?****`. In a message of uv or Ruff that doctor passes on, a URL keeps its scheme and host only. The source links of the profile's records, which ship with xpd, print whole with their anchors, and so does only a URL that is exactly one of those links.
- **Parse errors.** When `pyproject.toml` does not parse, doctor names the line, the column and the reason, and leaves out the text of the line.
- **Files.** To check the Ruff settings, doctor writes the `[tool.ruff]` table alone as a `ruff.toml` and no other part of `pyproject.toml`. It sits in a new folder of the system's temporary folder, with a name no other process can predict and mode 0700, the file mode 0600, and doctor removes the folder when Ruff ends, also after an error. The journal of `--apply` and the session files of the Stop hook are mode 0600 too, in a folder of mode 0700 for the journal.

This holds for text and JSON, for `--check` and `--apply`, and for the Stop hook. A token in the path of a URL in a table that doctor owns, such as a dependency `pkg @ https://host/<token>/pkg.whl`, still shows, so keep such tokens in uv's settings or in the environment.

## What the profile sets

The profile sets seven Ruff rules. A code that no selector of the project turns on goes into `lint.extend-select`, which keeps the project's own selection, also when Ruff runs it by default. An exact code in `lint.ignore` or in a per-file ignore goes, unless an exception records it.

| Code | What Ruff checks | What to do about a finding |
| --- | --- | --- |
| `PTH118` | Paths joined with `os.path.join` | Use a `Path` join where the caller accepts a path object. Preserve string or bytes contracts. |
| `PERF101` | A list created just to iterate over it | Iterate over the original iterable when a snapshot is unnecessary. |
| `PERF102` | A dictionary loop that discards the key or value | Iterate over the keys or values you actually use. |
| `PGH003` | A blanket `type: ignore` | Name the diagnostic you intend to suppress. |
| `PGH004` | A blanket `noqa` | Name the Ruff rule you intend to suppress. |
| `RUF017` | Lists combined with repeated addition | Flatten the inputs in one pass to avoid copying earlier items repeatedly. |
| `RUF100` | A `noqa` that no longer suppresses a finding | Remove the unused suppression. |

The profile sets three ty rules under `[tool.ty.rules]`. A higher level than the profile's needs no edit. When an override table gives a rule a lower level, by its name or through `all`, doctor sets the rule to the profile's level in that table. This holds for `[[tool.ty.overrides]]` tables and for an inline `overrides = [{ include = [...], rules = { ... } }]` array alike. ty applies a rule's own entry before `all`.

| Rule | Level | Why enable it |
| --- | --- | --- |
| `blanket-ignore-comment` | `error` | A suppression without a diagnostic name can hide a new error on the same line. |
| `disjoint-cast` | `error` | Casting between types that cannot overlap makes a promise the value cannot satisfy. |
| `missing-type-argument` | `warn` | A bare generic such as `list` leaves the element type unspecified. |

The profile sets three pytest settings in the pytest table of `pyproject.toml`: `[tool.pytest]`, or `[tool.pytest.ini_options]` when the project already uses it.

| Setting | Where | What it changes |
| --- | --- | --- |
| `strict = true` | The pytest table | An unknown marker, an invalid setting, a repeated parameter id and an `xfail` test that passes each fail the run. |
| `pytest -ra` | `addopts`, before a `--` that ends the options | The summary lists each skipped test and each expected failure. |
| `pytest --import-mode=importlib` | `addopts` | pytest imports a test file without adding its folder to `sys.path`. |

`strict` needs pytest 9.0. For pytest 8, the profile sets `pytest -ra`, `pytest --import-mode=importlib`, `pytest --strict-markers` and `pytest --strict-config` in `addopts` and `xfail_strict = true` in `[tool.pytest.ini_options]` instead. Doctor takes the pytest release that a fresh checkout gets: the one `uv.lock` locks when it meets every declared requirement, else pytest 8 when a declared requirement such as `pytest==8.4.2` admits no pytest 9, else pytest 9 when a list declares pytest, else the release in the project's virtual environment.

A project that declares pytest gets two plugins in each dependency list that holds pytest, whether that is a dependency group, an extra or `[project]` `dependencies`, and in `dev` of `[dependency-groups]` when only the virtual environment has pytest. A plugin counts only where uv installs it with pytest on every platform:

- in the same list, or in a group that the list includes with `{ include-group = "<name>" }`, and not in an unrelated list
- in an entry without an environment marker, so `"pytest-timeout; python_version < '3.0'"` does not count

Doctor compares the names of dependency groups and extras as uv does, so `Test_Plugins` and `test-plugins` name the same group.

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
| A `-p` in `addopts` that loads one of these plugins, such as `-p pytest_mock`, `-pflaky`, `-p rerunfailures` or `-p=pytest_mock` | It runs the plugin without a dependency on it, or stops pytest with an import error. |

Doctor reads `-p` as pytest 9 does: `-p` takes the next argument, `-p<value>` takes the rest of its own, and spaces around the value go. So `-p=name` names the plugin `=name`, which pytest cannot import.
| `pytest --reruns` and `pytest --reruns-delay` in `addopts`, `reruns` and `reruns_delay` in the table | The same as a rerun plugin. |
| `strict_config`, `strict_markers`, `strict_parametrization_ids`, `strict_xfail` or `xfail_strict` set to `false` | Each turns off part of `strict`, such as the failure on an unknown marker or on an `xfail` test that passes. |

The profile pins the tools that check the project, at the releases its rules and levels were reviewed for, as exact requirements in `[dependency-groups]` `dev`. Doctor replaces a looser requirement there, such as `"ruff"`, with the pin.

| Tool | Requirement in `dev` |
| --- | --- |
| Ruff | `ruff==0.17.4` |
| ty | `ty==0.0.106` |

`uv.lock` must lock these releases. A lock that holds another release of Ruff or ty, or none, is out of date, so `--check` exits 1 and `--apply` runs `uv lock`. A requirement for Ruff or ty in another list that the pin does not meet is a conflict, because uv locks one release for every list. The `script` kind gets no pin, since its `pyproject.toml` holds tool settings only. Run the tools with `uv run ruff` and `uv run ty`.

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
@@ -6,9 +6,24 @@
 [dependency-groups]
 dev = [
     "pytest>=9.2.0",
-    "pytest-rerunfailures",
     "pytest-timeout",
+    "pytest-randomly",
+    "ruff==0.17.4",
+    "ty==0.0.106",
 ]

 [tool.pytest.ini_options]
-addopts = "--reruns 2 --strict-markers"
+addopts = "--strict-markers -ra --import-mode=importlib"
+strict = true
+
+[tool.ruff.lint]
+extend-select = ["PTH118", "PERF101", "PERF102", "PGH003", "PGH004", "RUF017", "RUF100"]
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

`xpd doctor --apply --kind application` writes this diff, runs `uv lock`, and records the revision and kind last. A second `xpd doctor --check` then prints `The project matches profile revision 1.` and exits 0.

## Kinds of project

The first apply needs a kind, which `pyproject.toml` records. Doctor suggests one: `application` for a project with `[project.scripts]` or without `[build-system]`, `library` for one with `[build-system]`, and `script` when `pyproject.toml` has no `[project]` and a Python file has inline script metadata. Each kind states the layout it needs, and a project that lacks it gets a conflict.

| Kind | What it is | Layout | Lock |
| --- | --- | --- | --- |
| `library` | A package that others install. | `[project]` and `[build-system]` | `uv.lock` |
| `application` | A program that runs from its checkout or as an installed command. | `[project]` | `uv.lock` |
| `pipeline` | An application that turns data into results. | `[project]` | `uv.lock` |
| `script` | Standalone scripts. | At least one Python file with a `# /// script` block, and a `pyproject.toml` that holds the `[tool]` table and nothing else, so the scripts declare their own dependencies and the profile adds no plugin | `<script>.lock` beside each script, by `uv lock --script` |

A kind can add its own Ruff, ty and pytest values to the ones every kind gets, and the profile can own whole files for a kind, such as a workflow, which the project keeps as exact copies of templates that ship with xpd. Revision 1 gives every kind the same values and owns no file.

## One place for each tool

The profile keeps each tool's settings in its table of `pyproject.toml` at the project root. Doctor reports these as conflicts, which no edit of `pyproject.toml` resolves:

- a `ruff.toml` or `.ruff.toml` at the root or below it, or a `pyproject.toml` below the root with a `[tool.ruff]` table
- an `extend` key in `[tool.ruff]`
- a `ty.toml` at the root or below it, or a `pyproject.toml` below the root with a `[tool.ty]` table
- `pytest.toml`, `.pytest.toml`, `pytest.ini`, `.pytest.ini`, a `[pytest]` section in `tox.ini`, or a `[tool:pytest]` section in `setup.cfg`, at the root or below it, or a `pyproject.toml` below the root with a pytest table
- a key that the pinned Ruff or ty does not know, such as `bogus` in `[tool.ruff.lint]`, because the tool stops with an error on it
- a rule name in `[tool.ty.rules]` or in the `rules` of an override that ty 0.0.106 does not have or has removed, such as `unknown-review-rule`, because ty reports `unknown-rule` and its check fails
- a Ruff rule selector that Ruff 0.17.4 does not know, such as `NOT_A_RULE` in `lint.select`, or a removed rule such as `ANN101` in an array that selects it
- a value that the pinned Ruff rejects, such as `line-length = "wide"`
- a requirement for Ruff or ty in a dependency list other than `dev` that the profile's pin does not meet
- a per-file ignore that turns a rule of the profile off through a group selector, such as `PERF`
- an argument in `addopts` that overrides a pytest setting of the profile, such as `-o strict=false`, or that turns off a plugin of the profile, such as `-p no:randomly` or `-pno:timeout`, a `-p=<name>` that pytest cannot import, or `--disable-plugin-autoload` without a `-p` that loads each plugin of the profile again
- a layout that the kind does not allow
- a uv workspace, in the project or above it
- both `[tool.pytest]` and `[tool.pytest.ini_options]` with settings
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
| `ruff` | A rule of the profile | `files` names file patterns that Ruff reads. Without `files`, the exception covers the whole project. | A per-file ignore for each pattern, or the code in `lint.ignore` |
| `ty` | A rule of the profile | `value` names a level below the profile's. It covers the whole project. | The rule at that level in `[tool.ty.rules]` |
| `pytest` | A setting, argument or plugin that the profile adds | It covers the whole project. | Nothing. The profile stops requiring it. |

Doctor marks each exception `valid`, `invalid`, `redundant` when it changes nothing for this project, such as one for a pytest 8 setting in a project on pytest 9, or `orphaned` when the profile no longer sets its `id`. Any other mark than `valid` fails `--check` and stops `--apply` until you fix or delete the exception. A setting that differs from the profile without an exception is a change in the plan.

## Upgrades

Each xpd release holds one revision of the profile. When a newer xpd brings a new revision, doctor shows the change, and `--apply` writes it and records the new revision. A project whose recorded revision comes from a newer xpd gets a conflict, so an older xpd never writes over it.

## What `--apply` does

Apply plans first and writes nothing while the plan has a conflict, an exception that is not valid, or no kind. A setting that the pinned Ruff or ty rejects is such a conflict, so apply checks the settings it would write before it writes them. It then writes `pyproject.toml` and the owned files, runs `uv lock` for a missing or out-of-date lock, checks that no drift remains and that every lock is up to date, and records the revision and kind last. Apply keeps the comments of the items it keeps, leaves a value that already complies as it is written, and keeps the permissions of each file it replaces. It never syncs an environment, installs Git hooks, runs project scripts or publishes.

Apply keeps the prior content of each file it touches in the folder `.xpd-apply` at the project root while it runs. When a lock refresh or the final check fails, apply puts every file back and exits 2. When an apply stops before it ends, the next one restores the files from that folder first.

`--check` runs `uv lock --check` for each lock that exists, so it needs uv.

Doctor checks the Ruff settings it would leave with Ruff 0.17.4 itself, so the answer is the same before and after `uv sync`. It runs the Ruff of the project's environment or of `PATH` when that is Ruff 0.17.4, and else `uv tool run --from ruff==0.17.4 ruff`, from uv's cache first, with `--config` naming a copy of `[tool.ruff]` alone. `--check` and `--apply` fetch Ruff 0.17.4 from the index when the cache lacks it, and plain `xpd doctor` reads the cache only. When no route works, doctor lists the reason under `Unverified` and never says that the project matches. `--check` then exits 2, and `--apply` writes nothing.

## Stop hook

In a project whose `pyproject.toml` has a `[tool.xpd.profile]` table, [`xpd hook`](../hook/) compares the project with its profile when an agent session begins and when a turn ends. The comparison includes each lock file, which the hook checks with `uv lock --check --offline` from uv's cache. It holds the turn for a difference that the session made, such as a new dependency without `uv lock`, and it notes the differences that were there before and each lock it could not check. It checks Ruff and ty settings against the catalog only and does not run Ruff. See the hook page.

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

`profile` holds the plan: `revision`, `applied`, `kind`, `changes`, `conflicts` (each with `ambiguous`, which makes `--check` exit 2), `exceptions`, `unverified` (what doctor could not check), `locks` (each with its `state` and the `reason` it is out of date, when doctor can tell) and the `diff`. Read `tools` for version information, `suggestions` for conditional advice, `policies` for deliberate choices Doctor keeps, and `notes` for limits it encountered. `xpd doctor --check --format json` prints the plan alone, and `xpd doctor --apply --format json` prints what it wrote.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Doctor printed its report, `--check` found the project matches the profile, or `--apply` wrote the plan or had nothing to write. |
| 1 | `--check` found a difference from the profile: a value, a missing or out-of-date lock, a conflict, or an exception that is not valid. |
| 2 | Doctor could not read the project, `--check` found a layout it cannot hold to one setup (a uv workspace, two pytest tables with settings, or a revision from a newer xpd) or could not run Ruff 0.17.4 on the settings, or `--apply` wrote nothing because of a conflict, an exception that is not valid, a missing kind or a failed step. |
