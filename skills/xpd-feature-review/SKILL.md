---
name: xpd-feature-review
description: Research and review how each model feature relates to its target, what is known at prediction time, and whether the feature adds value in development folds. Use for feature engineering, lags, rolling windows, target encoding, Hamilton lineage, and feature ablation.
---

# Review model features

1. Use `xpd profile` to inspect inputs, `xpd find` to locate relevant APIs, and `xpd api xpd/recipe/feature-module` for the project style. Record the target, prediction cutoff, development snapshot and holdout boundary in `feature-review.json`. Give every implemented feature and deferred hypothesis a stable ID and owning file. Record raw baseline inputs and graph intermediates.
2. Search the owner's library with `library_search` and open a result with `library_read` before citing it. If the library cannot supply evidence, read a primary source. Record the expected direction, units, conditions and path to the target. Use provider publication and revision metadata to establish the actual cutoff.
3. Build a development-only adapter. Keep the final holdout out of the process. Run `xpd feature-review --questions` for the catalog, then `xpd feature-review start --adapter adapter.py --check "git diff --check"` on the committed fixture. Use `xpd feature-review --format json` to list findings, fold evidence and questions with their owners.
4. Explain how every input reaches the model at the cutoff. Verify the inner fold policy and target encoding. Use `xpd graph` to trace each node and `xpd equiv old.py new.py` to verify rewiring with nulls and multiple entities. Run `xpd check` for broader lint.
5. Compare paired lift in each fold, numeric stability, correlation clusters and unseen categories. Explain where the mechanism could fail and where support is thin. Retain low-lift features only with a source-backed domain reason tied to measured lift. Document deferrals with a mechanism, proposed availability and regimes.
6. Use `xpd feature-review plan plan.json` to record fixes, accepted findings and answers with evidence. For interface changes obtain approval and use `xpd feature-review approve <fix>`. Commit one fix and run `xpd feature-review gate <fix>`. Restore an abandoned change before `xpd feature-review drop <fix> --reason "<reason>"`. A kept gate leaves design judgments open and reopens answers when their evidence changes.
7. Use `xpd feature-review status` to find the next stage. Record remaining answers with `xpd feature-review plan -`, then close the ledger with `xpd feature-review finish`.

Use `xpd refit` and the MCP gates for a project with a scikit-learn raw-row refit or a saved model to compare. Use the paired feature variants for feature ablation.

## Example mechanism

Day-ahead residual load approximates demand that other suppliers must meet. Higher residual load can move the market up the supply curve. Compare load and wind forecasts in the same units and with pre-auction vintages. See `mazzi2018pricetaker` p. 2177 and `reguant2014online` p. 5, § B. Available capacity and the marginal fuel can change the relationship. See `aid2015electricity` pp. 60–62. Treat findings from other markets as hypotheses and check local operating periods and publication metadata.
