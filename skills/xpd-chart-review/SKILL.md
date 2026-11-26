---
name: xpd-chart-review
description: Audit and improve charts and presentation tables against Edward Tufte's "The Visual Display of Quantitative Information" with a recorded xpd chart-review ledger across Python and React/TypeScript (Plotly, Dash, AG Grid, Matplotlib, Seaborn, pandas). Use when making, changing, or reviewing charts, dashboards, or data tables.
---

# Review charts and tables against Tufte's book with xpd

Use `xpd chart-review` to audit and improve charts and presentation tables across Python (Plotly, Dash, Dash AG Grid, Matplotlib, Seaborn, pandas) and React/TypeScript (`react-plotly.js`, `plotly.js`, `ag-grid-react`, `ag-grid-community`). It combines static multi-language checks, the Plotly rule pack, optional runtime figure measurement via `--adapter`, and a per-figure checklist covering all 57 principles from Edward Tufte's *The Visual Display of Quantitative Information* (second edition).

| Group | What it checks | Example kinds |
| --- | --- | --- |
| Graphical integrity | Distortions of scale, baseline, dimensions, money, or context (VDQI Ch. 2 to 3) | `non-zero-baseline`, `dual-axis`, `undeflated-money`, `two-point-series`, `pseudo-3d`, `area-by-radius` |
| Data-ink and chartjunk | Non-data frames, grids, patterns, and software chrome (VDQI Ch. 4 to 6) | `default-template`, `explicit-grid`, `boxed-frame`, `hatch-or-pattern`, `excess-ticks` |
| Encoding and form | Weak chart types, unordered palettes, smoothing, and ordering (VDQI Ch. 1, 7, 8) | `pie-chart`, `small-data-chart`, `rainbow-colorscale`, `red-green-encoding`, `redundant-color`, `stacked-area` |
| Words and layout | Legends, labels, typography, aspect ratio, and slope banking (VDQI Ch. 1, 7, 9) | `legend-instead-of-labels`, `column-name-axis-title`, `all-caps-text`, `rotated-ticks`, `portrait-single-panel`, `bank-to-45` |
| Tabular presentation | Borders, striping, alignment, formatting, grouping, and headers (VDQI pp. 33, 178 to 183) | `table-vertical-rules`, `table-zebra-striping`, `table-unformatted-numbers`, `table-raw-header`, `table-centered-numbers`, `table-unaligned-decimals`, `table-missing-group-dividers`, `table-excessive-precision` |

The work is a recorded loop of five stages, and no stage grades its own work. `xpd` measures mechanical findings and builds the per-figure VDQI checklist (`<figure-id>/<question-id>`), you fix defects and judge design questions, and `xpd chart-review gate` proves every commit before `xpd chart-review finish` closes the run.

```text
xpd chart-review start <paths> --check "<tests>" [--adapter "<adapter.py or command>"]
repeat
  list     xpd chart-review <paths> [--adapter "..."] --format json
  plan     xpd chart-review plan <plan.json>
  change   one fix at a time, committed cleanly
  gate     xpd chart-review gate <fix-id>
until no open fix or unanswered checklist item remains
xpd chart-review finish
```

## Survey and start

Find the project's test or build check command, and write a small adapter when the figures can be rendered:

- **Python adapter (`--adapter figures.py`)**: define `figures()` returning a dict of name to figure (`go.Figure`, Matplotlib `Figure`, `dash_ag_grid.AgGrid`, or a normalized `{kind, spec}` dict). `xpd` executes it in the project's Python and measures all 10 runtime facts (canvas, values, colors, x-values, text, slopes, axes, legend, table, and grid).
- **Command adapter (`--adapter "node export_figures.mjs"`)**: print JSON `{"figures": [{"name": "...", "library": "plotly" | "ag-grid", "spec": ...}]}` on stdout to measure React/TypeScript Plotly and AG Grid components with the same Rust analyzer.

Start the recorded run:

```sh
xpd chart-review start <paths> --check "<test-command>" --adapter "<adapter>"
```

The run state persists in `.git/xpd-chart-review.json` (or `.git/xpd-ledger.sqlite` when durable mode is enabled), so it survives across sessions.

## List and judge

Run the stateless survey to read every mechanical finding and every remaining per-figure checklist item:

```sh
xpd chart-review <paths> [--adapter "<adapter>"] --format json
```

- **Mechanical findings** have an `id`, `kind`, `places`, and `evidence`. Fix them directly or record an `"accepted"` reason when an exception shows the data better (VDQI p. 191).
- **Per-figure checklist items** have an `id` of `<figure-id>/<question-id>`, `file`, `figure`, `page`, `principle`, `text`, and `look`. Hybrid questions already covered by a passing runtime adapter are omitted automatically; every remaining item must either be addressed by a design fix or answered in the plan.
- Run `xpd chart-review --questions --format json` to inspect the full 57-question catalog and 10 runtime facts. For a new Plotly figure, read the house playbook with `xpd api xpd/recipe/plotly-chart`.

## Plan

Write `plan.json` with three lists:

- `"fixes"`: ordered commits. Each fix either names mechanical `"findings": ["..."]` or sets `"goal": "design"` with `"questions": ["<figure-id>/<question-id>", ...]` for a judgment-based visual improvement. Set `"changes_interface": true` whenever a fix changes a public API, and call `xpd chart-review approve <fix-id>` after user approval.
- `"accepted"`: mechanical findings left in place, each with a one-sentence `"reason"`.
- `"answered"`: per-figure checklist items audited without code changes, each with `"item"`, `"verdict"` (`"pass"`, `"na"`, or `"accepted"`), and `"reason"`.

```sh
xpd chart-review plan plan.json
```

## Change and gate

Apply one fix at a time, commit it cleanly, and run the gate:

```sh
xpd chart-review gate <fix-id>
```

The gate keeps the fix only when:
- every `--check` command that passed at `start` still passes,
- the total mechanical finding count did not rise, and
- for a mechanical fix, at least one named finding is gone; for a `"goal": "design"` fix, the commit modified at least one file owning the named checklist items (and keeping the fix automatically marks those checklist items fixed).

If a gate fails, repair and re-gate, or restore with `git reset --hard <current>` and run `xpd chart-review drop <fix-id> --reason "..."`. `xpd chart-review status` always shows the next stage.

## Finish and report

`xpd chart-review finish` exits 1 while any fix is open, any new finding has no accepted reason, or any per-figure checklist item remains unanswered. Once every defect and checklist item is resolved:

```sh
xpd chart-review finish
```

Report the before and after finding counts, each kept fix with its commit, any dropped or accepted items with reasons, and the completed per-figure checklist summary.
