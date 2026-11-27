---
name: xpd-data-prep
description: Use xpd data-prep to audit raw data, joins, cleaning, imputation, splits and scaling of features and targets in a recorded ledger.
---

# Prepare features and targets

1. Run `xpd data-prep start <paths> --check "<project check>" --adapter prep.py` from a clean commit. Repeat `--check` for independent checks. Add `--target <column>` when the name is ambiguous.
2. List findings and column questions with `xpd data-prep <paths> --adapter prep.py --format json`. Match the options used at start. The adapter can define `columns()` returning a pandas or Polars frame, a file path, or a dictionary of lists. It can declare `target` and `model_family`. A command adapter can emit JSON with measured `columns`, `target` and `model_family`.
3. Read the graph before changing steps. Confirm which missing values, sentinel codes, outliers and duplicate keys have business meaning. Confirm when features were available relative to the target. Treat the scaler recommendation as evidence and fit preprocessing on training rows or inside each fold.
4. Run `xpd data-prep plan plan.json`. Cite finding IDs in `fixes[].findings` and record deliberate exceptions in `accepted` with a reason. Answer column items in `answered` with `item`, a `verdict` of `pass`, `na` or `accepted`, and a `reason`. For a code fix to a judgment item, use `goal: "design"` and put its item ID in `questions`. When exact rows repeat, cite deduplication or a group-aware split before marking duplicate handling as passed.
5. Run `xpd data-prep approve <fix-id>` after the user approves an interface change. Keep pandas steps in the project's `pipe` and `assign` style. Use `xpd equiv old.py new.py` for a preparation-only rewrite intended to preserve results and keep its JSON result in the plan reason. Commit one fix and run `xpd data-prep gate <fix-id>`.
6. Use `xpd data-prep drop <fix-id> --reason "<why>"` after restoring an abandoned edit. Run `xpd data-prep status` to resume the run. Use `xpd data-prep finish` once fixes, new findings and checklist items are resolved.

Run `xpd data-prep --questions --format json` for the fact and question catalog.
