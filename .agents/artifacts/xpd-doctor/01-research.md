---
date: 2026-11-18T21:56:00Z
repos:
  - identifier: bjoernaagaard/xpd
    sha: 6109ea55ddcd84556fa2da02bd1c0a1f6e53a316
topic: Optional Ruff and ty recommendations for xpd doctor
type: research
tags: [research, xpd, ruff, ty, uv]
status: complete
---

# What xpd doctor can recommend

Which project settings catch inefficient Python, preserve xpd's preferred style, and expose code that silences the type checker? The strongest starting point is a small set of named rules with an explanation for each. The same pyproject.toml works with or without uv. Ruff and ty each read their own table. uv can run the installed tools with `uv run ruff check` and `uv run ty check`. No setting under `[tool.uv]` enables either checker. [uv configuration](https://docs.astral.sh/uv/concepts/configuration-files/)

## Three tools catch different parts of the same expression

This example was created for the investigation. It ran with Ruff 0.17.4, ty 0.0.106 and the local xpd 0.36.0 release binary. The source checkout was at the 0.37.0 release commit.

```python
from typing import Any, cast
import os

def destination(value: Any) -> str:
    return os.path.join("out", cast(str, value))
```

| Check run | Observed output |
| --- | --- |
| `ruff check --isolated --select ANN401,PTH118 example.py` | ANN401 on the parameter and PTH118 on the join |
| `ty check --error disjoint-cast example.py` | All checks passed! |
| `xpd check example.py --format json` | One warning, python/typing-cast, on line 5 |

A cast from Any supplies no evidence about the value. Give the producer an accurate return type or accept object and narrow it at the boundary. xpd already catches common spellings of cast and broad dictionary signatures. Its syntax matcher enumerates spellings, so its coverage depends on how a name was imported. See [typing-cast](../../../rules/python/typing-cast.yml) and [dict-str-any](../../../rules/python/dict-str-any.yml).

## Ruff can flag wasted work without pushing complex comprehensions

The following is a candidate snippet for an unconfigured project. In an existing table, add missing codes to the existing array. Each code is a separate recommendation.

```toml
[tool.ruff.lint]
extend-select = ["PTH118", "PERF101", "PERF102", "ANN401", "PGH003", "PGH004", "RUF100"]
```

| Code | What earns its place | Limit |
| --- | --- | --- |
| PTH118 | Prefer pathlib when joining paths | A readability preference. Path objects can cost more and callers may require strings. |
| PERF101 | Flag a redundant list around an iterable in a loop | A snapshot can be intentional when mutating the source. Review the suggestion. |
| PERF102 | Use keys or values when a loop discards half of each item | Small savings. xpd's frame and array rules cover larger data operations. |
| ANN401 | Surface explicit Any in function signatures | Inferred Any and Any hidden inside data structures need other checks. |
| PGH003, PGH004 | Require a code on type ignores and noqa suppressions | A narrow suppression can still hide a real defect. |
| RUF100 | Find noqa comments that no longer suppress a diagnostic | Evaluate with the project's full rule set. Changing selection can make a valid suppression look stale. |

Avoid enabling PERF as a family by default. PERF401 can suggest a comprehension that xpd warns about once it spans several lines. Compare [Ruff rules](https://docs.astral.sh/ruff/rules/), [PTH118](https://docs.astral.sh/ruff/rules/os-path-join/) and [xpd's comprehension rule](../../../rules/python/comprehension-multi-line.yml).

### A lazy import needs both a rule and a policy

```toml
[tool.ruff.lint]
extend-select = ["TID254"]

[tool.ruff.lint.flake8-tidy-imports]
ban-lazy = "all"
```

Combine this code with the preceding array when using both snippets. A list of named modules narrows the ban. With preview disabled, Ruff 0.17.4 reported TID254 for explicit lazy syntax targeting Python 3.15. A literal `__lazy_modules__ = ["os"]` and `import os` also triggered the rule targeting 3.11. Function-local imports are a different choice, covered by PLC0415. Optional dependencies are a reason to keep that choice separate. [Setting](https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_ban-lazy) · [TID254](https://docs.astral.sh/ruff/rules/lazy-import-mismatch/)

## ty can catch ways code loses type information

These four rules default to ignore in ty 0.0.106. The levels below are candidate policy choices.

```toml
[tool.ty.rules]
disjoint-cast = "error"
blanket-ignore-comment = "error"
missing-type-argument = "warn"
dynamic-function-decorator-return = "warn"
```

| Rule | Evidence it asks for |
| --- | --- |
| disjoint-cast | Source and target types must overlap. A cast from Any can still pass. |
| blanket-ignore-comment | Name the diagnostic in a ty ignore directive. |
| missing-type-argument | Supply the element type of a generic such as list. |
| dynamic-function-decorator-return | Keep a decorator's dynamic return type from erasing a function's checked signature. |

For a project migrating other checkers' suppressions, separately consider `respect-type-ignore-comments = false` under `[tool.ty.analysis]`. This exposes errors previously hidden from ty by a Python type-ignore comment. In a temporary project the default check reported one redundant cast. The candidate rules plus this switch reported five diagnostics, adding a disjoint cast, a bare generic, a blanket ty ignore, and an invalid assignment. A cast from Any still passed. Inspect project policy before proposing this switch.

Redundant casts, stale ignores, unknown rule names in ty suppressions, some redundant conditions, and unused awaitables already warn. ty's strict redundant-condition variant covers more cases at a higher false positive rate. Warnings can fail CI. Use xpd for costly pandas and NumPy operations, and ty for claims it can check from types. [ty rules](https://docs.astral.sh/ty/reference/rules/) · [ty settings](https://docs.astral.sh/ty/reference/configuration/)

## The active configuration determines whether advice helps

| Situation | Evidence doctor would need |
| --- | --- |
| Ruff uses ALL, family selectors, ignore lists, inherited config or per-file overrides | Check a representative source path with `ruff check --show-settings PATH`. A root TOML scan alone cannot prove a rule applies to every file. |
| A standalone config exists | Ruff prefers .ruff.toml then ruff.toml over pyproject.toml in the same directory. ty.toml takes priority over a sibling pyproject.toml. Name the active file and use standalone table names there. |
| A tool is absent or older | Offer optional guidance. Check support with `ruff rule CODE`, `ruff config OPTION` or `ty explain rule NAME` when available. An unknown option and a disabled rule need different advice. |
| A project explicitly ignores a rule | Show that choice and its consequence. Appending a selector can be redundant or change precedence. |
| A tool cannot read a config | Report the parse error and file. The effective state is unknown. |

[How Ruff resolves configuration](https://docs.astral.sh/ruff/configuration/) · [How ty resolves configuration](https://docs.astral.sh/ty/configuration/)

In xpd's own [pyproject.toml](../../../pyproject.toml), Ruff selects ALL with six exclusions. On py/equiv.py, resolved settings included PTH118 and PERF401, preview disabled, and an empty ban-lazy list. Ruff reported “All checks passed!” for the file. Across py and python, replacing the selected rules with a short candidate list produced nine stale-noqa reports. Extending the real configuration produced none. This is a concrete reason to validate advice against the project's existing selection.

## Where the command fits in xpd

[main.rs](../../../src/main.rs) enters the [Clap CLI](../../../src/cli.rs), which handles command dispatch and text or JSON output. [package_check.rs](../../../src/package_check.rs) already walks parent directories for pyproject.toml and looks for tool table headers. That scan cannot resolve rule arrays, ignore lists, inherited files or effective severity. Cargo.toml declares no direct TOML parser. The [pandas pack](../../../rules/pandas/pack.yml) already makes a conditional recommendation for pandas-stubs when it detects a checker.

The existing [style sample](../../../samples/python/style.py) produced three distinct xpd warnings at lines 114, 115 and 128 for a broad dictionary signature, a cast, and untyped JSON use. [Pre-commit checks](../../../lefthook.yml) exercise samples and Ruff. The [release workflow](../../../.github/workflows/release.yml) smoke-tests the installed CLI. No doctor command or doctor-specific tests were found. This research ran the tools on samples and project files. It did not run the release workflow.

A first implementation can print the applicable recommendation, reason, destination file and TOML fragment. Missing tools can still receive examples marked unverified. For acceptance, run the CLI in a fresh project, this repository, a project with shadowing standalone config, and a project without Astral executables. A second run after adopting a suggestion should recognize the effective setting. The remaining design choice is how much effective configuration doctor promises to resolve when the tool cannot answer directly.
