---
name: xpd
description: "Python library habits: run xpd to lint code for slow or wrong use of libraries such as pandas and NumPy, profile data files, prove refactors equivalent, and run recorded review ledgers."
---

# xpd

Use `xpd` whenever you write, change, or review Python code, or charts and presentation tables in Python or React TypeScript. Run `xpd <command> -h` for the current flags.

## Recorded review ledgers

Each review command surveys mechanical findings and a per-entity checklist, then gates every fix through `start`, `plan`, `approve`, `gate`, `drop`, `status`, and `finish`. Load the matching skill before starting a run.

| Command | Scope | Skill |
| --- | --- | --- |
| `xpd chart-review <paths>` | Charts and presentation tables in Plotly, Dash, AG Grid, Matplotlib, Seaborn, pandas, and React TypeScript against Tufte's 57 principles | `xpd-chart-review` |
| `xpd data-prep <paths>` | Raw data ingestion, joins, cleaning, imputation, splits, and feature or target scaling | `xpd-data-prep` |
| `xpd ts-review <paths>` | Time series cadence, intermittency, seasonality, breaks, and group coherence before forecasting | `xpd-ts-review` |
| `xpd feature-review <paths>` | Model feature domain mechanism, publication cutoff, lineage, and development-fold lift | `xpd-feature-review` |
| `xpd opt-review <paths>` | Numerical formulations, solver diagnostics, portfolio validation, and Pareto convergence | `xpd-opt-review` |
| `xpd model-review <paths>` | Model validation design, baseline comparisons, calibration, and conformal interval coverage | `xpd-model-review` |
| `xpd untangle <paths>` | Structural coupling, hidden links, copies, and dead weight | `xpd-untangle` |
| `xpd optimize <paths>` | Measured CPU, memory, and I/O waste with `--bench` and profiler adapters | `xpd-optimize` |
| `xpd harden <paths>` | Silent failures, fixed waits, retries, and unreliable tests with a determinism gate | `xpd-harden` |

## Everyday commands

- `xpd check <paths> --format json`: lint without running Python. Fix every `error`. Fix every `warning` unless you can state why the pattern is right here, and write `# ast-grep-ignore: <pack>/<rule>` with the reason on the line above. Add `--fix` for safe rewrites and `--unsafe-fixes` when you will review the diff.
- `xpd check` with `style = "strict"` in `[tool.xpd]` of `pyproject.toml` or `~/.config/xpd/config.toml` promotes house style rules to warnings. With the xpd plugin installed, `xpd hook` lints each edit and holds the end of your turn while a file you changed has an `error` or `warning`.
- `xpd find "<task>"`: search indexed symbols, rules, and playbooks before picking a library API. Read a playbook with `xpd api xpd/recipe/<name>` (`pandas-pipeline`, `feature-module`, `linear-script`, `plotly-chart`, `nixtla-forecasting`, `model-review`, `opt-review`), and inspect signatures and defaults with `xpd api <Class>.<method>` instead of importing the library.
- `xpd rules --format json`: read every rule with its documentation before changing code it flags. Use `xpd rules outline <paths> --format json` to map files before reading them, and `xpd rules run --pattern '<code>' <paths> --json` (with optional `--rewrite` and `--update-all`) for syntax searches.
- `xpd profile <data file>`: inspect dtypes, nulls, cardinality, and memory before choosing frame methods. Needs pandas.
- `xpd graph <files> --format json`: trace step pipelines and Apache Hamilton dataflow without running Python. Reuse existing columns from `pipelines[].steps`, pass `pipelines[].source` to `columns=` in `pd.read_parquet`, and return `frame.assign(...)` or `frame.join(...)` instead of mutating an input frame.
- `xpd equiv script.py --since HEAD` (or `xpd equiv old.py new.py`): prove a data logic refactor preserves `result` on real rows. Needs pandas.
- `xpd refit <contract.json>`: refit a saved scikit-learn Pipeline on raw training rows and compare scores with the contract. Needs pandas and scikit-learn.
- `xpd doctor --format json`: compare the project against xpd's Ruff, ty, and pytest profile. Run `xpd doctor --apply` only when the user asks.
- `xpd animate guide` or MCP `animate_guide`: build and preview an editable Manim animation before rendering the final video.

The xpd MCP server holds only tools that a shell command cannot be (`discovery_gate`, `contract`, `gates`, `show_issues`, `show_dataflow`, `show_columns`, and `animate_*`), so run the CLI commands above in the shell even when MCP is connected.
