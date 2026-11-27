---
name: xpd-chart-review
description: Audit and improve charts and presentation tables against Edward Tufte's "The Visual Display of Quantitative Information" with a recorded xpd chart-review ledger across Python and React TypeScript (Plotly, Dash, AG Grid, Matplotlib, Seaborn, pandas). Use when making, changing, or reviewing charts, dashboards, or data tables.
---

# Review charts and tables with xpd

Use `xpd chart-review` to audit charts and presentation tables across Python (Plotly, Dash, Dash AG Grid, Matplotlib, Seaborn, pandas) and React TypeScript (`react-plotly.js`, `plotly.js`, `ag-grid-react`, `ag-grid-community`). It measures mechanical defects across five Tufte groups and builds a per-figure checklist covering all 57 principles from *The Visual Display of Quantitative Information* (second edition).

| Group | Example finding kinds |
| --- | --- |
| Graphical integrity | `non-zero-baseline`, `dual-axis`, `extra-dimensions`, `distorted-size`, `undeflated-money`, `missing-context` |
| Data-ink and chartjunk | `default-template`, `boxed-frame`, `heavy-grid`, `redundant-color`, `pattern-fill`, `decorative-gauge` |
| Encoding and form | `pie-chart`, `small-data-chart`, `rainbow-colorscale`, `red-green-only`, `stacked-area`, `unsorted-categories` |
| Words and layout | `legend-instead-of-labels`, `column-name-axis-title`, `all-caps-text`, `rotated-ticks`, `vertical-aspect-ratio`, `poor-banking` |
| Tabular presentation | `table-vertical-rules`, `table-zebra-striping`, `table-unformatted-numbers`, `table-raw-header`, `table-centered-numbers`, `table-unaligned-decimals` |

## Adapters

- **Python adapter (`--adapter figures.py`)**: define `figures()` returning a dict of name to figure (`go.Figure`, Matplotlib `Figure`, Dash `AgGrid`, or a plain spec dict). `xpd` executes it in the project's Python and measures all 10 runtime facts.
- **Command adapter (`--adapter "node export_figures.mjs"`)**: print JSON `{"figures": [{"name": "...", "library": "plotly", "spec": ...}]}` (or `"library": "ag-grid"`) on stdout to measure React TypeScript specs with the same Rust analyzer.

For a new Plotly figure, read the house playbook with `xpd api xpd/recipe/plotly-chart`. Run `xpd chart-review --questions --format json` for the full 57-principle catalog and 10 runtime facts.

## Recorded run

1. **Start**: run `xpd chart-review start <paths> --check "<test-command>" [--adapter "<adapter>"]` from a clean commit.
2. **List**: run `xpd chart-review <paths> [--adapter "<adapter>"] --format json` to read stable finding IDs and `<figure-id>/<question-id>` checklist items. Hybrid questions already checked by a passing adapter drop off the checklist automatically.
3. **Plan**: write `plan.json` with ordered `fixes` (each naming mechanical `findings` or `"goal": "design"` with checklist `questions`), `accepted` findings with one-sentence reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd chart-review plan plan.json`.
4. **Approve**: run `xpd chart-review approve <fix-id>` after user approval for any fix marked `changes_interface: true`.
5. **Gate**: commit one fix and run `xpd chart-review gate <fix-id>`. The gate keeps the commit when checks pass, mechanical findings do not rise, and a named finding is gone or a `design` fix edits a file owning its checklist items.
6. **Drop**: restore an abandoned edit with `git reset --hard <commit>` and run `xpd chart-review drop <fix-id> --reason "<why>"`.
7. **Status and finish**: run `xpd chart-review status` to resume across sessions, and `xpd chart-review finish` once every open fix, new finding, and checklist item is settled.
