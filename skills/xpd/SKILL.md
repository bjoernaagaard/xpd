---
name: xpd
description: "Python library habits: run xpd to lint code for slow or wrong use of libraries such as pandas and NumPy, audit repos across workflows, index project libraries, profile data files, prove refactors equivalent, and run recorded review ledgers."
---

# xpd

Use `xpd` whenever you write, change, or review Python code, or charts and presentation tables in Python or React TypeScript. Run `xpd <command> -h` for current flags.

## Recorded review ledgers

Each review command surveys mechanical findings and a per-entity checklist, then gates every fix through `start`, `plan`, `approve`, `gate`, `drop`, `status`, and `finish`. Load the matching skill before starting a run, and use MCP `show_ledger` to inspect active runs visually.

| Command | Scope | Skill |
| --- | --- | --- |
| `xpd chart-review <paths>` | Charts and tables in Plotly, Dash, AG Grid, Matplotlib, Seaborn, pandas, and React TypeScript against Tufte's 57 principles | `xpd-chart-review` |
| `xpd data-prep <paths>` | Raw data ingestion, joins, cleaning, imputation, splits, and feature or target scaling | `xpd-data-prep` |
| `xpd ts-review <paths>` | Time series cadence, intermittency, seasonality, breaks, and group coherence before forecasting | `xpd-ts-review` |
| `xpd feature-review <paths>` | Model feature domain mechanism, publication cutoff, lineage, and development-fold lift | `xpd-feature-review` |
| `xpd opt-review <paths>` | Numerical formulations, solver diagnostics, portfolio validation, and Pareto convergence | `xpd-opt-review` |
| `xpd model-review <paths>` | Model validation design, baseline comparisons, calibration, and conformal interval coverage | `xpd-model-review` |
| `xpd untangle <paths>` | Structural coupling (`--map` for Python and Rust, `--move`, `--inline-facade`), copies, and dead weight | `xpd-untangle` |
| `xpd optimize <paths>` | Measured CPU, memory, and I/O waste with `--bench` and profiler adapters | `xpd-optimize` |
| `xpd harden <paths>` | Silent failures, fixed waits, retries, and unreliable tests with a determinism gate | `xpd-harden` |
| `xpd discover <paths>` | Duplicate public names, bare verb names, and dynamic keys that plain-text search misses | `xpd-discover` |

## Everyday commands

- `xpd check <paths> --format json`: lint without running Python. Fix every `error`. Fix every `warning` unless you state why the pattern fits here with `# ast-grep-ignore: <pack>/<rule>` and a reason above. Use `--fix` (and `--unsafe-fixes` when reviewing diffs). Set `style = "strict"` in `[tool.xpd]` to promote house style rules to warnings.
- `xpd audit <paths> [--since <commit>] [--workflows all] [--fail-on-regression]`: run one read-only rollup across `check`, static ledger workflows, `--since` regression deltas, and recorded ledger debt without running project code.
- `xpd find "<task>"` and `xpd api <symbol>`: search bundled and project-local library symbols, rules, and playbooks before picking an API. Use `xpd library add <package>` (plus `list` and `remove`) to index third-party or internal packages under `[tool.xpd.libraries]`.
- `xpd rules --format json`, `xpd rules outline <paths> --format json`, and `xpd rules run --pattern '<code>' <paths> --json`: read rule docs, map files before reading, and run syntax searches or rewrites.
- `xpd profile <data file>`: inspect dtypes, nulls, cardinality, and memory before choosing frame methods. Needs pandas.
- `xpd graph <files> --format json` (plus `graph impact` and `graph map`): trace step pipelines and Apache Hamilton dataflow statically without running Python.
- `xpd equiv script.py --since HEAD` (or `old.py new.py`) and `xpd refit <contract.json>`: prove data refactors preserve `result` or refit a saved scikit-learn Pipeline against its contract.
- `xpd gates start --contract <contract.json> --train <train.py> --predict <predict.py> --features <features.py>`: run `profile`, `check`, `graph`, `refit`, and `equiv` in a durable workflow (`retry`, `status`, `resume`).
- `xpd doctor --format json`: compare the project against xpd's Ruff, ty, and pytest profile. Run `xpd doctor --apply` only when asked.
- `xpd animate catalog <family>` and `xpd animate guide`: compare Manim primitives and render editable animations.

The xpd MCP server holds only tools that a shell command cannot be (`discovery_gate`, `contract`, `gates`, `show_issues`, `show_dataflow`, `show_columns`, `show_map`, `show_ledger`, and `animate_*`), so run CLI commands in the shell even when MCP is connected.
