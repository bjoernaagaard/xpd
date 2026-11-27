---
name: xpd-harden
description: Find and fix silent failures, fixed waits, retries, and flaky or disabled tests with xpd harden. Verify every fix against the reliability checklist and determinism gate.
---

# Harden Python reliability with xpd

Use `xpd harden` to make Python code and test suites fail loudly, deterministically, and honestly.

| Group | Finding kinds |
| --- | --- |
| Silent failure and reliability errors | `swallowed-exception`, `rule-issue` |
| Fixed waits and retries | `sleep-or-retry` |
| Disabled or unreliable tests | `disabled-test`, `flaky-test` |

Each module with a finding, including test files, gets six reliability checklist items. Run `xpd harden --questions` to read the catalog. Set `goal` to `design` for a checklist fix. At every gate, `xpd` runs the first `--check` command a second time and requires both exit status and stdout SHA-256 digest to match.

## Recorded run

1. **Start**: run `xpd harden start . --check "<project-check>"` from a clean commit.
2. **List**: run `xpd harden . --format json` to read finding IDs and checklist item IDs.
3. **Plan**: write `plan.json` with ordered `fixes` (naming finding IDs or `"goal": "design"` with `questions`), `accepted` findings with reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd harden plan plan.json`.
4. **Approve**: run `xpd harden approve <fix-id>` after user approval for a fix that changes public behavior or error types.
5. **Gate**: commit one fix and run `xpd harden gate <fix-id>`.
6. **Drop**: restore the run's current commit and run `xpd harden drop <fix-id> --reason "<why>"` if a fix is abandoned.
7. **Status and finish**: run `xpd harden status` to see the next stage, and `xpd harden finish` once every open fix, new finding, and checklist item is resolved.
