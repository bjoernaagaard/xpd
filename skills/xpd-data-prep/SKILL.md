---
name: xpd-data-prep
description: Audit raw data loading, joins, cleaning, imputation, train-test splits, and feature or target scaling in a recorded xpd data-prep ledger.
---

# Audit data preparation with xpd

Use `xpd data-prep` to verify Python preparation code from raw ingestion through joins, imputation, splitting, and feature or target scaling.

| Group | Example finding kinds |
| --- | --- |
| Leakage | `fit-before-split`, `fit-on-test`, `cross-val-on-fitted-data`, `scaler-fit-before-split`, `prefit-before-cross-validation` |
| Joins and alignment | `merge-without-validate`, `polars-join-without-validate`, `merge-asof-unsorted`, `concat-columns-unchecked`, `join-key-unvalidated` |
| Target transforms | `transformed-target-pair`, `scaled-target-without-inverse` |
| Dtypes and missing values | `compare-to-missing`, `polars-null-comparison`, `nan-to-int`, `silent-dtype-coercion` |
| Pipeline shape | `stateful-function-transformer`, `read-parquet-all-columns`, `frame-column-write`, `no-inplace` |

Run `xpd data-prep --questions --format json` for the full 14-fact and 13-question catalog.

## Adapter

Pass `--adapter prep.py` where `columns()` returns a pandas or Polars frame, a data file path, or a dict of column lists, and optionally declares `target` and `model_family`. A command adapter prints JSON with measured `columns`, `target`, and `model_family`. Pass `--target <column>` when the target name is ambiguous.

## Recorded run

1. **Start**: run `xpd data-prep start <paths> --check "<project-check>" --adapter prep.py` from a clean commit.
2. **List**: run `xpd data-prep <paths> --adapter prep.py --format json` using the same options as `start`.
3. **Judge**: inspect the pipeline graph and measured column facts. Confirm business meaning for missing values, sentinel codes, outliers, and duplicate rows, verify feature availability before target time, and fit every scaler or imputer on training rows only.
4. **Plan**: write `plan.json` with ordered `fixes` (mechanical `findings` or `"goal": "design"` with `questions`), `accepted` findings, and `answered` column verdicts (`pass`, `na`, or `accepted`) with reasons. When exact rows repeat, cite deduplication or a group-aware split before marking duplicate handling as `pass`. Run `xpd data-prep plan plan.json`.
5. **Approve and gate**: run `xpd data-prep approve <fix-id>` for approved interface changes. Prove preparation-only rewrites with `xpd equiv old.py new.py`, commit one fix, and run `xpd data-prep gate <fix-id>`.
6. **Drop, status, and finish**: restore an abandoned edit before `xpd data-prep drop <fix-id> --reason "<why>"`, check `xpd data-prep status` to resume, and close the run with `xpd data-prep finish`.
