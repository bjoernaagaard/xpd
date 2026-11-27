---
title: Review model features
description: Document what drives a model feature and measure its contribution in development folds.
---

Use `xpd feature-review` when a model feature needs a defensible reason to stay.

1. Declare the target, prediction cutoff, development snapshot, baseline columns and candidates in `feature-review.json`. Give each feature or hypothesis a stable ID and an owner. Document raw inputs and intermediates.
2. Trace the mechanism from a primary source to the target. Check provider publication and revision times against the cutoff. Use `xpd graph` for dependency and consumer evidence. Use `xpd equiv` for rewiring that preserves the result.
3. Commit the sources and a development-only adapter. Run `xpd feature-review start --adapter adapter.py --check "git diff --check"`, then list with `xpd feature-review --format json`.
4. Read the paired fold scores alongside stability, correlations and unseen levels. Use `xpd feature-review plan plan.json` to record fix IDs, accepted findings and checklist answers with reasons. For a low-lift retention decision, cite the measured result and the domain reason. Record an unimplemented hypothesis with its mechanism and deferral.
5. Run `xpd feature-review approve <fix>` for an approved interface change. Commit a fix and run `xpd feature-review gate <fix>`. Restore an abandoned change before `xpd feature-review drop <fix> --reason "<reason>"`. Review reopened answers on the bare list.
6. Check `xpd feature-review status`, record current answers, then run `xpd feature-review finish`.

See the [command reference](../../commands/feature-review/) for the manifest and adapter contracts.
