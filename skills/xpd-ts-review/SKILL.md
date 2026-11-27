---
name: xpd-ts-review
description: Use xpd ts-review to inspect time series cadence, seasonal drivers, breaks, and pooling groups before forecasting.
---

# Review time series before forecasting

1. Run `xpd ts-review start <paths> --check "<project check>" --adapter series.py` from a clean commit. Run the bare list with `xpd ts-review <paths> --adapter series.py --format json`. Inspect the fourteen facts and twelve questions with `xpd ts-review --questions --format json`.
2. Read cadence and gaps before interpreting periods. Compare phase, shape, intermittency, trend, and break evidence. Check which metadata explains the seasonal profile and whether candidate groups have coherent phases. Record one strategy per series or group, and name what its drivers and regimes teach you.
3. Run `xpd ts-review plan plan.json` with ordered fixes and their finding IDs. Use `"goal": "design"` and `questions` for an edit that addresses checklist items. Use `answered` with an item ID, a verdict of `pass`, `na` or `accepted`, and a reason. For insufficient history, drop the unsupported period and use `na` with that reason.
4. Run `xpd ts-review approve <fix-id>` after approval for an interface change. Commit one fix and run `xpd ts-review gate <fix-id>`. Restore an abandoned edit before `xpd ts-review drop <fix-id> --reason "<reason>"`.
5. Run `xpd ts-review status` to see the next stage. Run `xpd ts-review finish` when fixes, new findings, and checklist items are settled.

A Python adapter defines `series()` and returns a pandas or Polars frame, a path, or a dictionary of columns containing `unique_id`, `ds`, and `y`. Include metadata columns to compare candidate groups. Use the [Nixtla forecasting playbook](../../rules/xpd/playbooks/nixtla-forecasting.md) after recording a strategy.
