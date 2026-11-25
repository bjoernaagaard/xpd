Run `xpd doctor` at the root of a Python project to compare it with the profile, the one target setup for Ruff, ty, pytest, packaging, the files that are not Python, and the hooks and CI that run them, that ships with xpd. Doctor prints a unified diff of `pyproject.toml` and of each file the profile owns, with the reason for each change, the lock files the project needs, and then the conditional advice that the project's code points to. `xpd doctor --apply` writes the diff, and `xpd doctor --check` exits 1 while the project differs from the profile.

The plan reads files. It compares the effect of the project's own settings, so an extra selector in `lint.ignore` that turns off only rules the profile turns off, or a ty level above the profile's, needs no edit. A tool's default does not count, because another release of the tool can change it, so the project names each setting, rule and level itself. Doctor runs uv to check and write lock files, the pinned Ruff to confirm that it accepts the settings, and the installed Ruff and ty for the conditional advice.

## What doctor shows of the project

Doctor shows only the settings it owns, so that its output never carries a token from another part of `pyproject.toml`. It owns `[tool.ruff]`, `[tool.ty]`, the pytest table, `[tool.xpd]`, `[dependency-groups]`, `dependencies`, `optional-dependencies` and `requires-python` of `[project]`, `exclude-newer` of `[tool.uv]`, and the files that [Files that are not Python](#files-that-are-not-python) and [Hooks and CI](#hooks-and-ci) list.

- **The diff.** Each line outside those tables, such as a line of `[tool.uv]`, `[[tool.uv.index]]`, `[tool.uv.sources]` or `[tool.uv.pip]`, shows as `<hidden>`. A line shows only when everything on it belongs to those tables, so a line that also holds another setting is hidden, such as `tool = { uv = { ... }, ruff = { ... } }` or `tool.uv.index-url = "..."` beside `tool.ruff.lint...` lines. Table headers and blank lines stay, so the diff keeps its shape. A change on a hidden line shows in the list of changes and not in the diff.
- **URLs.** In every line doctor prints, a URL keeps its scheme, host and path, and its user and password, query and fragment become `****`, as in `https://****@example.com/simple?****`. In a message of uv or Ruff that doctor passes on, a URL keeps its scheme and host only. The source links of the profile's records, which ship with xpd, print whole with their anchors, and so does only a URL that is exactly one of those links.
- **Parse errors.** When `pyproject.toml` does not parse, doctor names the line, the column and the reason, and leaves out the text of the line.
- **Files.** To check the Ruff settings, doctor writes the `[tool.ruff]` table alone as a `ruff.toml` and no other part of `pyproject.toml`. It sits in a new folder of the system's temporary folder, with a name no other process can predict and mode 0700, the file mode 0600, and doctor removes the folder when Ruff ends, also after an error. The journal of `--apply` and the session files of the Stop hook are mode 0600 too, in a folder of mode 0700 for the journal.

This holds for text and JSON, for `--check` and `--apply`, and for the Stop hook. A token in the path of a URL in a table that doctor owns, such as a dependency `pkg @ https://host/<token>/pkg.whl`, still shows, so keep such tokens in uv's settings or in the environment.

## What the profile sets

The profile is strict: every Ruff rule runs unless a reason below turns it off, and preview is on, so the experimental rules and the formatter's preview style run too. Each value has a reviewed record with its reason and source. [Read the profile](https://github.com/bjoernaagaard/xpd/blob/main/src/doctor/profile.toml) and [the records that explain it](https://github.com/bjoernaagaard/xpd/blob/main/src/doctor/decisions.jsonl).

### Ruff selection and preview

The profile owns `lint.select` and holds it at `["ALL"]`. A project whose selection lacks `ALL` gets `select = ["ALL"]` in `[tool.ruff.lint]`, and an old `select` at the top of `[tool.ruff]` goes. `preview`, `lint.preview` and `format.preview` are `true`, and `lint.explicit-preview-rules = true` goes, because it keeps each preview rule off until a selector names its code. With Ruff 0.17.4 the project then runs 887 rules.

`lint.ignore` holds the profile's list below. A selector in `lint.ignore` or `lint.extend-ignore` that turns off any other rule goes, whether it names one rule, such as `S101`, or a group, such as `D`, unless an exception records the rule. A selector that turns off only rules of the list, such as `D100`, stays.

| Selector | Why it is off |
| --- | --- |
| `D1` | Missing docstrings stay allowed, and a docstring that exists follows PEP 257. |
| `CPY` | The project's license covers each file, so no file repeats a notice. |
| `W191`, `E111`, `E114`, `E117` | Ruff format chooses indentation, so these conflict with it. |
| `E501` | Ruff format wraps lines as far as it can. The remaining long lines are left to judgment. |
| `Q` | Ruff format chooses quotes and escapes. Covers `Q000` through `Q004`. |
| `COM812`, `COM819` | Ruff format adds and keeps commas where its layout needs them. |
| `ISC001` | Ruff format can collapse split literals onto one line. |
| `ISC002` | Split literals may wrap across lines, and `allow-multiline` stays `true`. |
| `D203`, `D206`, `D300` | Ruff format sets blank lines before class docstrings, docstring indentation and docstring quotes. |
| `TC` | Annotation imports stay available for code that reads annotations at run time. |
| `ANN401` | `Any` is allowed where a dynamic interface needs it. ty keeps checking the types. |
| `EM`, `TRY003` | An exception may take its message directly, also on a standard exception class. |
| `PLR2004` | A number that explains itself may stand in a calculation or an assertion. |
| `PLR0913`, `PLR0917` | A clear signature may have several parameters, and positional ones are judged at the API boundary. |
| `PLR0912`, `C901` | Branches and complexity are judged in context, since a cap can reward splitting flat code into helpers. |
| `FBT003` | A positional boolean is allowed where the caller cannot change the API. |
| `TD` | Work is tracked in issues. `FIX` still reports markers such as TODO. |

With preview on, Ruff 0.17.4 also runs `RUF201`, which reports each rule code in the selectors of `pyproject.toml`, such as `ANN401` in `lint.ignore`, and asks for the rule's name. The profile writes the codes, so `ruff check` on `pyproject.toml` reports them, and `ruff check --fix` replaces each code with the rule's name, such as `any-type`. Doctor counts a rule's name as its code in every selector.

`lint.pydocstyle.convention = "pep257"` also turns off the docstring checks that PEP 257 does not ask for: `D212`, `D213`, `D214`, `D215`, `D404` through `D411`, `D413`, `D415`, `D416`, `D417` and `D420`.

`lint.per-file-ignores` holds these patterns. A per-file ignore that turns off another rule for a pattern goes, unless an exception records the rule for that pattern.

| Pattern | Off | Why |
| --- | --- | --- |
| `test_*.py`, `*_test.py` | `S101` | pytest checks with `assert`. |
| `e2e/test_*.py` | `S101`, `INP001` | Standalone end-to-end tests check with `assert` and need no package. |
| `py/*.py` | `INP001`, `T201` | Standalone scripts print to stdout and need no package. |

### Banned APIs

`lint.flake8-tidy-imports.banned-api` bans these APIs, so Ruff reports each use as `TID251`. An entry the project has keeps its own message, and the profile adds each entry it lacks.

| API | Message |
| --- | --- |
| `unittest` | Use pytest for a real end-to-end, property, or differential test. |
| `unittest.mock`, `mock`, `pytest_mock` | Exercise the real code with controlled input and real dependencies. |
| `flaky`, `pytest.mark.flaky` | Fix the flaky test or remove it and record why. |
| `pytest.mark.repeat` | Run the test once and fix the cause of any instability. |
| `pytest.skip`, `pytest.mark.skip` | Fix or remove the test, or use a conditional skip for an unsupported platform or unavailable required resource. |
| `pytest_socket` | Exercise the real configured service when the flow requires a network connection. |
| `toml` | Use tomllib for TOML reads and an approved writer for TOML writes. |
| `tomli` | Use the standard library tomllib to read TOML. |
| `typing.cast`, `typing_extensions.cast` | Fix the type at its source or narrow it with a runtime check. |

`pytest.mark.skipif` stays allowed for an unsupported platform or a missing resource.

### Other Ruff settings

| Setting | Value | Why |
| --- | --- | --- |
| `required-version` | `"==0.17.4"` | Ruff stops on another release, so every run uses the reviewed one. |
| `line-length` | `100` | Code uses 100 columns. |
| `indent-width` | `4` | The standard indentation. |
| `fix`, `unsafe-fixes` | `false` | A check leaves the source alone until the developer asks for a fix. |
| `force-exclude` | `true` | Excluded files stay excluded when a tool passes their paths. |
| `respect-gitignore` | `true` | Files that Git ignores stay unchecked. |
| `lint.future-annotations` | `false` | Type-only import moves stay conservative. |
| `lint.flake8-tidy-imports.ban-relative-imports` | `"all"` | Every import names its module from the package root. |
| `lint.flake8-type-checking.strict`, `lint.flake8-type-checking.quote-annotations` | `false` | Annotations stay as they are for code that reads them at run time. |
| `format.quote-style`, `format.indent-style`, `format.nested-string-quote-style`, `format.line-ending` | `"double"`, `"space"`, `"alternating"`, `"auto"` | The formatter's defaults. |
| `format.skip-magic-trailing-comma` | `false` | A trailing comma keeps a collection on several lines. |
| `format.docstring-code-format`, `format.docstring-code-line-length` | `true`, `"dynamic"` | The runnable examples in docstrings are formatted to fit after their indentation. |

Each setting holds exactly this value. Doctor reads a lint setting at its old place at the top of `[tool.ruff]` too, which the `lint` table overrides.

### ty

The profile sets five ty rules under `[tool.ty.rules]`. A higher level than the profile's needs no edit. When an override table gives a rule a lower level, by its name or through `all`, doctor sets the rule to the profile's level in that table. This holds for `[[tool.ty.overrides]]` tables and for an inline `overrides = [{ include = [...], rules = { ... } }]` array alike. ty applies a rule's own entry before `all`.

| Rule | Level | Why |
| --- | --- | --- |
| `blanket-ignore-comment` | `error` | A suppression without a diagnostic name can hide a new error on the same line. |
| `unused-ignore-comment` | `error` | A suppression that silences nothing hides the next error on its line. |
| `disjoint-cast` | `error` | Casting between types that cannot overlap makes a promise the value cannot satisfy. |
| `missing-direct-dependency` | `error` | An import that only another package's dependencies provide breaks when that package drops it. |
| `missing-type-argument` | `warn` | A bare generic such as `list` leaves the element type unspecified. |

`analysis.respect-type-ignore-comments = false` makes a `ty: ignore` comment that names its rule the only suppression ty honors, and `terminal.error-on-warning = false` keeps a warning visible without failing the run.

`missing-direct-dependency` is a preview rule that reads the project's declared dependencies through uv. ty 0.0.106 turns that on with the environment variable `TY_UV=1` and has no setting for it in `pyproject.toml`. `uv check --locked` sets it and runs the ty that `uv.lock` locks, so run ty with `uv check --locked`.

### pytest

The profile sets these values in the pytest table of `pyproject.toml`: `[tool.pytest]`, or `[tool.pytest.ini_options]` when the project already uses it.

| Setting | Where | What it changes |
| --- | --- | --- |
| `strict = true` | The pytest table | An unknown marker, an invalid setting, a repeated parameter id and an `xfail` test that passes each fail the run. |
| `timeout = "60"` | The pytest table | A test that runs longer than 60 seconds fails and prints where it stopped. Give an exceptionally slow test its own timeout marker. |
| `markers` holds `"e2e: drives a full user flow"` | The pytest table | Strict mode accepts the `e2e` marker for tests that drive a whole user flow. A marker of the same name keeps the project's description. |
| `filterwarnings` holds `"error"` | The pytest table, first | Every warning is an error. A filter after it may allow a dependency's warning by message, category and module. |
| `pytest -ra` | `addopts`, before a `--` that ends the options | The summary lists each skipped test and each expected failure. |
| `pytest --import-mode=importlib` | `addopts` | pytest imports a test file without adding its folder to `sys.path`. |
| `pytest -p no:doctest` | `addopts` | Doctests stay out of the required suite. |

A list such as `markers` must hold each of the profile's items and may hold more. The profile pins pytest 9, so a fresh checkout gets pytest 9, and `strict` replaces the flags that pytest 8 needed. A pytest 8 requirement in `dev` becomes the pin, and one in another list is a conflict. For a `script` kind with pytest 8 in its environment, the profile sets `pytest --strict-markers`, `pytest --strict-config` and `xfail_strict = true` in `[tool.pytest.ini_options]` instead.

A project gets two plugins in each dependency list that holds pytest, whether that is a dependency group, an extra or `[project]` `dependencies`, pinned at the release the profile was reviewed for. A plugin counts only where uv installs it with pytest on every platform:

- in the same list, or in a group that the list includes with `{ include-group = "<name>" }`, and not in an unrelated list
- in an entry without an environment marker, so `"pytest-timeout; python_version < '3.0'"` does not count

Doctor compares the names of dependency groups and extras as uv does, so `Test_Plugins` and `test-plugins` name the same group.

| Plugin | Reason |
| --- | --- |
| `pytest-timeout` | A test that hangs fails and prints where it stopped. |
| `pytest-randomly` | The tests run in a new order each time, so a test that depends on another test fails. |

The profile removes these from every project, and no exception keeps them.

| In the project | Reason |
| --- | --- |
| `pytest-rerunfailures`, `flaky`, `pytest-repeat` | A rerun or a repeat hides the run where the code was wrong. |
| `pytest-mock`, `mock` | A test with a mock checks the mock and not the code. |
| `pytest-socket` | A test that needs the network uses the real configured service. |
| A `-p` in `addopts` that loads one of these plugins, such as `-p pytest_mock`, `-pflaky`, `-p rerunfailures` or `-p=pytest_mock` | It runs the plugin without a dependency on it, or stops pytest with an import error. |
| `pytest --reruns`, `pytest --reruns-delay`, `pytest --count` and `pytest --repeat-scope` in `addopts`, `reruns` and `reruns_delay` in the table | The same as a rerun plugin. |
| `pytest --disable-socket`, `pytest --force-enable-socket`, `pytest --allow-hosts` and `pytest --allow-unix-socket` | The same as `pytest-socket`. |
| `pytest --disable-warnings`, `pytest --disable-pytest-warnings` and `pytest -p no:warnings` | Warnings are errors, so hiding them hides failures. |
| `pytest --cov`, `pytest --cov-fail-under`, `pytest -n`, `pytest --dist`, `pytest --snapshot-update`, `pytest --doctest-modules` and `pytest --doctest-glob` | Coverage, workers, snapshot updates and doctests stay out of the required suite, which runs serially. |
| `pytest --strict-markers` and `pytest --strict-config` on pytest 9 | `strict` replaces them. |
| `strict_config`, `strict_markers`, `strict_parametrization_ids`, `strict_xfail` or `xfail_strict` set to `false` | Each turns off part of `strict`, such as the failure on an unknown marker or on an `xfail` test that passes. |

Doctor reads `-p` as pytest 9 does: `-p` takes the next argument, `-p<value>` takes the rest of its own, and spaces around the value go. So `-p=name` names the plugin `=name`, which pytest cannot import.

### Pins

The profile pins the tools that check the project, at the releases its rules and levels were reviewed for, as exact requirements in `[dependency-groups]` `dev`. Doctor replaces a looser requirement there, such as `"ruff"`, with the pin.

| Tool | Requirement in `dev` |
| --- | --- |
| pytest | `pytest==9.2.0` |
| pytest-timeout | `pytest-timeout==2.4.0` |
| pytest-randomly | `pytest-randomly==5.1.0` |
| Hypothesis | `hypothesis==6.170.17` |
| Ruff | `ruff==0.17.4` |
| ty | `ty==0.0.106` |
| Tombi | `tombi==1.11.3` |
| rumdl | `rumdl==0.3.5` |
| yamllint | `yamllint==1.38.0` |
| typos | `typos==1.52.2` |
| ShellCheck, only with shell scripts | `shellcheck-py==0.11.0.1` |
| shfmt, only with shell scripts | `shfmt-py==4.2.0` |
| nbstripout, only with notebooks | `nbstripout==0.9.2` |
| deptry | `deptry==0.25.1` |
| lefthook | `lefthook==2.2.2` |
| jactionlint | `jactionlint==2.6.0` |

`uv.lock` must lock these releases. A lock that holds another release of one of them, or none, is out of date, so `--check` exits 1 and `--apply` runs `uv lock`. A requirement for one of them in another list that the pin does not meet is a conflict, because uv locks one release for every list. The `script` kind gets no pin, since its `pyproject.toml` holds tool settings only. Run the checks with `uv run --locked ruff check`, `uv run --locked ruff format --check`, `uv check --locked` and `uv run --locked pytest`.

A project gets the ShellCheck and shfmt pins when it tracks a shell script: a `.sh` or `.bash` file, or a file without a suffix whose first line runs `sh`, `bash`, `dash` or `ksh`. ShellCheck does not read zsh. It gets the nbstripout pin when it tracks a notebook. A file that Git ignores does not count.

## Packaging

The profile packages a project with `pyproject.toml` and uv. Doctor writes two values itself and reports every other difference as a conflict, since only the owner can choose the replacement.

| Rule | What doctor does |
| --- | --- |
| uv waits 3 days before it takes a new upload | Sets `exclude-newer = "3 days"` in `[tool.uv]` for every kind, and refreshes the locks. For an urgent fix, name the package in `exclude-newer-package` until the upload ages in. uv also reads this for `uv lock --script`. |
| A new project starts at Python 3.14 | Sets `requires-python = ">=3.14"` when `[project]` names none. An existing floor stays, so review it each year. |
| No cap on Python | A conflict for `<`, `<=`, `~=` or `==` in `requires-python`. |
| The build backend fits the code | `hatchling.build` for a pure Python package, `maturin` when `Cargo.toml` is at the root, and `scikit_build_core.build` when `CMakeLists.txt` is. Another backend, or a `[build-system]` without `build-backend`, is a conflict. Compare the old and new wheel before a release on the new backend. |
| Requirements are broad, and exact choices live in `uv.lock` | A conflict for an entry of `dependencies` or of an extra without a lowest release, such as `"attrs"`, or with one exact release, such as `"attrs==25.4.0"`. Use `>=` at the oldest release that works, and a cap or an exclusion only for a known incompatibility. A direct reference with `@` and the profile's own pins stay. |
| uv keeps its own defaults | A conflict for `index-strategy`, `no-build-isolation`, `no-build-isolation-package` or `default-groups` in `[tool.uv]` with another value than uv's own. |
| `deptry` finds missing and unused dependencies | Pins `deptry==0.25.1` in `dev`. Run `uv run --locked deptry .` and review what it reports, since a plugin, an entry point or a dynamic import can need a dependency that no import shows. |

Each kind declares this metadata in `[project]`, or lists it in `dynamic`:

| Kind | Metadata |
| --- | --- |
| `library` | `name`, `version`, `description`, `readme`, `requires-python`, a `Repository` in `[project.urls]`, `license` as an SPDX expression such as `"MIT"`, and `license-files` |
| `application`, `pipeline` | `name`, `version`, `description`, `readme`, `requires-python` and a `Repository` in `[project.urls]` |
| `script` | None, since its `pyproject.toml` holds tool settings only |

These old packaging and hook files are conflicts, each with the way to move it:

| In the project | Move it |
| --- | --- |
| `[tool.poetry]` | Move the metadata, scripts and runtime requirements to `[project]`, turn the groups and markers into `[dependency-groups]`, run `uv lock`, and map named indexes to `[tool.uv.sources]`. |
| `setup.py` | Move the metadata and entry points to `[project]`, read the script for build steps before you remove it, and choose the backend by what the build needs. |
| `setup.cfg` | Move the metadata to `[project]` and each tool's section to its table in `pyproject.toml`. |
| `requirements.txt`, `requirements-dev.txt` and any other `requirements*.txt` at the root | Add the direct requirements with `uv add -r`, put the development ones in the `dev` group, and export a requirements file from `uv.lock` only for a reader that needs one. |
| `tox.ini` | Move the environments that matter to the CI workflow, and run each command with uv on its Python and platform. |
| `.pre-commit-config.yaml` | Move the hooks and their file filters to `lefthook.yml`, and run the project's tools with `uv run --locked`. |

## Files that are not Python

The profile owns these files for every kind. The project keeps the first four as exact copies of the templates in [src/doctor/profiles](https://github.com/bjoernaagaard/xpd/tree/main/src/doctor/profiles), so deleting one or changing a line in it is drift. `.gitignore` and `.gitattributes` keep the project's own lines and must also hold each line of the template. Apply puts the missing ones at the top, so a later line of the project, such as `!keep.pyc` or `*.png binary`, still wins.

| File | Read by | What it sets |
| --- | --- | --- |
| `tombi.toml` | Tombi | TOML 1.0, strict schemas, the order of keys from each file's schema, the order of arrays kept as written, and each of Tombi's lint rules as an error, the use of a deprecated key included |
| `.rumdl.toml` | rumdl | Nine Markdown rules on whitespace and structure: `MD009`, `MD010`, `MD012`, `MD022`, `MD031`, `MD032`, `MD040`, `MD047` and `MD056`, and the files that Git ignores left out |
| `.yamllint.yml` | yamllint | Duplicate keys, trailing spaces and a missing last newline are errors, and the files that `.gitignore` lists are left out. YAML is formatted by hand. |
| `.editorconfig` | Editors and shfmt | UTF-8, LF line ends, a last newline and no trailing spaces, four spaces in Python and two in TOML, YAML and JSON. Markdown keeps its trailing spaces, which can end a line. |
| `.gitignore` | Git | `.venv/`, Python bytecode, the caches of pytest, Ruff, rumdl and Hypothesis, and `build/`, `dist/` and `*.egg-info/` |
| `.gitattributes` | Git | `* text=auto eol=lf`, so Git stores text with LF line ends |

The tools read these files with `uv run --locked`, such as `uv run --locked tombi format --check`, `uv run --locked rumdl check .`, `uv run --locked yamllint .` and `uv run --locked typos`. typos runs with its own defaults, and a project adds its own words in `[tool.typos]` of `pyproject.toml`. A project with shell scripts runs `uv run --locked shellcheck` and `uv run --locked shfmt --diff` on them, and one with notebooks runs `uv run --locked nbstripout --verify` on each notebook, so a notebook is committed without its outputs. Ruff already checks the Python cells of a notebook.

The `script` kind gets the same files and no pins, since its `pyproject.toml` holds tool settings only. Run its tools with `uvx`, such as `uvx tombi@1.11.3 format --check`.

After apply writes `pyproject.toml`, it formats the file with Tombi 1.11.3, so the file it wrote passes `tombi format --check`. A `.gitignore` that ignores a lock file the kind needs, such as `uv.lock`, is a conflict, since every checkout needs the lock.

## Example

The example project reruns each failed test twice.

```toml title="pyproject.toml"
[project]
name = "example"
version = "0.1.0"
description = "An example project."
readme = "README.md"
requires-python = ">=3.12"

[project.urls]
Repository = "https://example.com/example"

[dependency-groups]
dev = [
    "pytest>=9.2.0",
    "pytest-rerunfailures",
    "pytest-timeout",
]

[tool.pytest.ini_options]
addopts = "--reruns 2 --strict-markers"
```

Doctor prints the diff, then one entry for each change. The `...` lines stand for the lines left out here and for the conditional advice at the end.

```console
$ xpd doctor --format text
Profile revision 5, kind application, suggested because [project] has no [build-system], so uv runs it without building it
  A program that runs from its checkout or as an installed command, locked in uv.lock.
pyproject.toml records no profile yet.

--- pyproject.toml
+++ pyproject.toml
@@ -10,10 +10,105 @@

 [dependency-groups]
 dev = [
-    "pytest>=9.2.0",
-    "pytest-rerunfailures",
-    "pytest-timeout",
+    "pytest==9.2.0",
+    "pytest-timeout==2.4.0",
+    "pytest-randomly==5.1.0",
+    "hypothesis==6.170.17",
+    "ruff==0.17.4",
+    "ty==0.0.106",
+    "tombi==1.11.3",
+    "rumdl==0.3.5",
+    "yamllint==1.38.0",
+    "typos==1.52.2",
+    "deptry==0.25.1",
+    "lefthook==2.2.2",
+    "jactionlint==2.6.0",
 ]

 [tool.pytest.ini_options]
-addopts = "--reruns 2 --strict-markers"
+addopts = "-ra --import-mode=importlib -p no:doctest"
+strict = true
+timeout = "60"
+markers = ["e2e: drives a full user flow"]
+filterwarnings = ["error"]
+
+[tool.ruff]
+required-version = "==0.17.4"
+preview = true
...
+[tool.ruff.lint]
+select = ["ALL"]
+ignore = ["ANN401", "C901", "COM812", "COM819", "CPY", "D1", "D203", "D206", "D300", "E111", "E114", "E117", "E501", "EM", "FBT003", "ISC001", "ISC002", "PLR0912", "PLR0913", "PLR0917", "PLR2004", "Q", "TC", "TD", "TRY003", "W191"]
...
+[tool.ty.rules]
+blanket-ignore-comment = "error"
+disjoint-cast = "error"
+missing-direct-dependency = "error"
+missing-type-argument = "warn"
+unused-ignore-comment = "error"
...
+[tool.uv]
+exclude-newer = "3 days"
+
+[tool.xpd.profile]
+revision = 5
+kind = "application"
...
+++ .github/workflows/ci.yml
...
Changes
  ruff lint.select: unset -> ["ALL"], tool.ruff.lint.select
    Profile ["ALL"]. Every Ruff rule runs unless a written reason in the ignore list turns it off.
    Source: https://docs.astral.sh/ruff/linter/#rule-selection
...
  pytest --reruns: --reruns 2 -> removed, tool.pytest.ini_options.addopts at line 19
    Keep the first failure visible so you can find and fix its cause.
    Source: https://github.com/pytest-dev/pytest-rerunfailures/blob/master/README.rst
  pytest --strict-markers: --strict-markers -> removed, tool.pytest.ini_options.addopts at line 19
...
Locks
  uv.lock is missing. --apply runs uv lock.
...
xpd doctor --apply --kind application writes these changes.
...
```

`xpd doctor --apply --kind application` writes this diff, runs `uv lock`, and records the revision and kind last. It also writes the owned files, and formats `pyproject.toml` with Tombi. A second `xpd doctor --check` then prints `The project matches profile revision 5.` and exits 0.

## Kinds of project

The first apply needs a kind, which `pyproject.toml` records. Doctor suggests one: `application` for a project with `[project.scripts]` or without `[build-system]`, `library` for one with `[build-system]`, and `script` when `pyproject.toml` has no `[project]` and a Python file has inline script metadata. Each kind states the layout it needs, and a project that lacks it gets a conflict.

| Kind | What it is | Layout | Lock |
| --- | --- | --- | --- |
| `library` | A package that others install. | `[project]` and `[build-system]` | `uv.lock` |
| `application` | A program that runs from its checkout or as an installed command. | `[project]` | `uv.lock` |
| `pipeline` | An application that turns data into results. | `[project]` | `uv.lock` |
| `script` | Standalone scripts. | At least one Python file with a `# /// script` block, and a `pyproject.toml` that holds the `[tool]` table and nothing else, so the scripts declare their own dependencies and the profile adds no plugin | `<script>.lock` beside each script, by `uv lock --script` |

A kind can add its own Ruff, ty and pytest values to the ones every kind gets, and the profile can own whole files for a kind, such as a workflow, which the project keeps as exact copies of templates that ship with xpd. Revision 5 gives every kind the same tool values. It owns the files that [Files that are not Python](#files-that-are-not-python) and [Hooks and CI](#hooks-and-ci) list, and `conftest.py` at the root for `library`, `application` and `pipeline`. Hypothesis reads its profiles from Python only, so `conftest.py` registers the profile `slow`, which turns off the time limit per example and keeps every other check, beside the built-in `default` and `ci` profiles. Run `uv run --locked pytest --hypothesis-profile slow` for a long property run. A project without `conftest.py` gets the profile's copy. A project that keeps its own `conftest.py` must hold the line `settings.register_profile("slow", deadline=None)` with its import, and doctor reports a conflict until it does, since it never rewrites the project's own file.

## Hooks and CI

The profile owns these files as exact copies of the templates in [src/doctor/profiles](https://github.com/bjoernaagaard/xpd/tree/main/src/doctor/profiles). Every command in them runs with uv alone, so a project needs no other tool manager.

| File | Kinds | What it does |
| --- | --- | --- |
| `lefthook.yml` | Every kind | At commit, checks the staged files, each with the tool for its type: Ruff, Tombi, rumdl, yamllint, jactionlint on the files under `.github`, typos, ShellCheck and shfmt on shell scripts, which lefthook finds by their type also without a suffix, nbstripout on notebooks, and Gitleaks. At push, runs ty with `uv check --locked` and the tests with `uv run --locked pytest`. Every check reads and never writes, so a hook never stages an edit. |
| `.github/workflows/ci.yml` | Every kind | On each pull request, each push to `main` and once a week, installs the locked environment, runs the commit checks on every file and the push checks, checks the workflows, the composite actions and Dependabot with jactionlint, audits `uv.lock` with `uv audit --locked`, and scans the whole history for secrets with Gitleaks 8.30.1, whose download it checks against its SHA-256. It reads the repository only, pins each action by its commit, and cancels an older run of the same pull request. |
| `.github/dependabot.yml` | Every kind | One grouped pull request a week for the actions and one for `uv.lock`, each for releases at least 7 days old |
| `.github/jactionlint.yaml` | Every kind | Runs jactionlint with its `pedantic` rules, and `jactionlint --strict-exit` makes a warning fail the check |
| `.github/workflows/release.yml` | `library` that builds with `hatchling.build` | On a `v` tag, checks that the tag is on `main` and names the version in `pyproject.toml`, builds the sdist and the wheel with `uv build --no-sources`, installs the wheel with its dependencies, and publishes the files of that run from a separate job, the only one with an identity token, with `uv publish --trusted-publishing always` |
| `.python-version` | `library`, `application`, `pipeline` | The Python that uv runs the project, its hooks and CI with. A project without one gets the lowest release that `requires-python` admits, and a release that `requires-python` does not admit is a conflict. |

The `script` kind has no `dev` group, so its `lefthook.yml` and `ci.yml` run each tool at its pinned release with `uvx --exclude-newer=P3D`, such as `uvx --exclude-newer=P3D ruff@0.17.4 check`, audit each script's lock with `uv audit --locked --script`, and have no push checks. Its `dependabot.yml` updates the actions only.

Gitleaks ships no package on PyPI, so the commit check runs the `gitleaks` on `PATH` and lefthook skips it when there is none. CI always scans the whole history with the pinned release, so a secret that a commit without the hook brings in still fails the run.

Apply never installs Git hooks or publishes. After the first apply, run `uv run --locked lefthook install` once in each clone. To publish, register `.github/workflows/release.yml` with the environment `pypi` as a Trusted Publisher of the project on PyPI, and protect the `v` tags. A library that builds native code with maturin or scikit-build-core gets no release workflow, since it needs a wheel for each platform.

## One place for each tool

The profile keeps each tool's settings in its table of `pyproject.toml` at the project root. Doctor reports these as conflicts, which no edit of `pyproject.toml` resolves:

- a `ruff.toml` or `.ruff.toml` at the root or below it, or a `pyproject.toml` below the root with a `[tool.ruff]` table
- an `extend` key in `[tool.ruff]`
- a `ty.toml` at the root or below it, or a `pyproject.toml` below the root with a `[tool.ty]` table
- `pytest.toml`, `.pytest.toml`, `pytest.ini`, `.pytest.ini`, a `[pytest]` section in `tox.ini`, or a `[tool:pytest]` section in `setup.cfg`, at the root or below it, or a `pyproject.toml` below the root with a pytest table
- a key that the pinned Ruff or ty does not know, such as `bogus` in `[tool.ruff.lint]`, because the tool stops with an error on it
- a rule name in `[tool.ty.rules]` or in the `rules` of an override that ty 0.0.106 does not have or has removed, such as `unknown-review-rule`, because ty reports `unknown-rule` and its check fails
- a Ruff rule selector that Ruff 0.17.4 does not know, such as `NOT_A_RULE` in `lint.select`, a removed rule such as `ANN101` in an array that selects it, or a rule's name, such as `assert`, while preview is off
- a value that the pinned Ruff rejects, such as `line-length = "wide"`
- a requirement for Ruff or ty in a dependency list other than `dev` that the profile's pin does not meet
- an argument in `addopts` that overrides a pytest setting of the profile, such as `-o strict=false`, or that turns off a plugin of the profile, such as `-p no:randomly` or `-pno:timeout`, a `-p=<name>` that pytest cannot import, or `pytest --disable-plugin-autoload` without a `-p` that loads each plugin of the profile again
- a layout that the kind does not allow
- a second settings file of a tool whose file the profile owns: `.tombi.toml`, `.config/tombi.toml` or `[tool.tombi]` beside `tombi.toml`, `rumdl.toml`, `.config/rumdl.toml` or `[tool.rumdl]` beside `.rumdl.toml`, `.yamllint` or `.yamllint.yaml` beside `.yamllint.yml`, `.lefthook.yml`, `lefthook.yaml`, `.lefthook.yaml`, `lefthook.toml`, `.lefthook.toml`, `lefthook.json` or `.lefthook.json` beside `lefthook.yml`, `.github/dependabot.yaml` beside `.github/dependabot.yml`, and `.github/jactionlint.yml` beside `.github/jactionlint.yaml`, because the tool would read it first or warn about both
- a `.gitignore` that ignores a lock file the kind needs
- a uv workspace, in the project or above it
- both `[tool.pytest]` and `[tool.pytest.ini_options]` with settings
- a profile revision that a newer xpd recorded

Move the settings into `pyproject.toml`, and run doctor again.

## Record a deliberate exception

A project records an exception in `pyproject.toml`, beside the revision and kind, with the reason it needs one.

```toml
[tool.xpd.profile]
revision = 5
kind = "application"

[[tool.xpd.profile.exceptions]]
tool = "ruff"
id = "PTH118"
files = ["scripts/**"]
reason = "The scripts hand str paths to a tool that rejects Path objects."
```

| `tool` | `id` | Scope and value | What doctor writes |
| --- | --- | --- | --- |
| `ruff` | A rule that the profile turns on | `files` names file patterns that Ruff reads. Without `files`, the exception covers the whole project. | A per-file ignore for each pattern, or the code in `lint.ignore` |
| `ty` | A rule of the profile | `value` names a level below the profile's. It covers the whole project. | The rule at that level in `[tool.ty.rules]` |
| `pytest` | A setting, argument or plugin that the profile adds | It covers the whole project. | Nothing. The profile stops requiring it. |

Doctor marks each exception `valid`, `invalid`, `redundant` when it changes nothing for this project, such as one for a pytest 8 setting in a project on pytest 9, or `orphaned` when the profile no longer sets its `id`. Any other mark than `valid` fails `--check` and stops `--apply` until you fix or delete the exception. A setting that differs from the profile without an exception is a change in the plan. Ruff settings, the banned APIs, the ty settings and the pins take no exception.

## Upgrades

Each xpd release holds one revision of the profile. When a newer xpd brings a new revision, doctor shows the change, and `--apply` writes it and records the new revision. A project whose recorded revision comes from a newer xpd gets a conflict, so an older xpd never writes over it.

## What `--apply` does

Apply plans first and writes nothing while the plan has a conflict, an exception that is not valid, or no kind. A setting that the pinned Ruff or ty rejects is such a conflict, so apply checks the settings it would write before it writes them. It then writes `pyproject.toml` and the owned files, runs `uv lock` for a missing or out-of-date lock, checks that no drift remains and that every lock is up to date, records the revision and kind, and last formats `pyproject.toml` with the pinned Tombi through `uv tool run`. Apply keeps the comments of the items it keeps, leaves a value that already complies as it is written, and keeps the permissions of each file it replaces. It never syncs an environment, installs Git hooks, runs project scripts or publishes.

Apply keeps the prior content of each file it touches in the folder `.xpd-apply` at the project root while it runs. When a lock refresh or the final check fails, apply puts every file back and exits 2. When an apply stops before it ends, the next one restores the files from that folder first.

`--check` runs `uv lock --check` for each lock that exists, so it needs uv.

When the project matches the profile and has a `uv.lock`, `--check` also runs `uv run --locked pytest --markers`, which loads each plugin that `addopts` names and reads each option, and imports no test module. A plugin that is not installed or an option pytest does not know is a conflict, so `--check` exits 1. uv creates or syncs `.venv` for this run when it has to.

Doctor checks the Ruff settings it would leave with Ruff 0.17.4 itself, so the answer is the same before and after `uv sync`. It runs the Ruff of the project's environment or of `PATH` when that is Ruff 0.17.4, and else `uv tool run --from ruff==0.17.4 ruff`, from uv's cache first, with `ruff --config` naming a copy of `[tool.ruff]` alone. `--check` and `--apply` fetch Ruff 0.17.4 from the index when the cache lacks it, and plain `xpd doctor` reads the cache only. When no route works, doctor lists the reason under `Unverified` and never says that the project matches. `--check` then exits 2, and `--apply` writes nothing.

## Stop hook

In a project whose `pyproject.toml` has a `[tool.xpd.profile]` table, [`xpd hook`](../hook/) compares the project with its profile when an agent session begins and when a turn ends. The comparison includes each lock file, which the hook checks with `uv lock --check --offline` from uv's cache. It holds the turn for a difference that the session made, such as a new dependency without `uv lock`, and it notes the differences that were there before and each lock it could not check. It checks Ruff and ty settings against the catalog only and does not run Ruff. See the hook page.

## Conditional advice

Below the plan, doctor prints advice that depends on the project's code and choices, for Ruff 0.17.4, ty 0.0.106 and pytest 9.2.0. Doctor consults Ruff and ty in the project's environment or on `PATH`, and it gets the installed pytest version from the project's virtual environment. Advice for a missing tool is marked "unverified locally". Conditional advice names the decision you need to make and the evidence that raised it. For example, seeing scalar division is a reason to consider a ty warning. It does not establish that a particular divisor is zero.

The profile turns every Ruff rule on or off itself, so Ruff's advice is about settings.

| Project decision | Ruff settings Doctor considers |
| --- | --- |
| Enforce typed API boundaries | `allow-star-arg-any` and `ignore-fully-untyped` in the annotation settings, where the annotation rules run. |
| Control when imports run | `ban-lazy` for an explicit eager-import policy, where the code uses lazy imports. |
| Keep annotations needed at runtime | `runtime-evaluated-base-classes` and `runtime-evaluated-decorators`, including Pydantic's `BaseModel` and `validate_call`, where the `TC` rules run. |
| Diagnose an unexpected Ruff result | Doctor can suggest `ruff check --show-settings PATH` or `ruff check --show-files PATH` to inspect effective settings or included files. |

| Project decision | Conditional ty advice |
| --- | --- |
| Check arithmetic, conditional bindings or optional values | Consider `warn` for `division-by-zero`, `possibly-unresolved-reference` or `truthiness-test-of-none-union`, according to the code. |
| Protect typed boundaries | Consider `error` for `unsound-assignment`, `unsound-return` or `unsound-yield`. For decorators your project owns, consider `warn` for `dynamic-function-decorator-return`. |
| Change how ty analyzes the project | Consider enabling `strict-generic-narrowing` for generic container checks, or setting `return-type-inference` to `"always"` during gradual migration. |
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
