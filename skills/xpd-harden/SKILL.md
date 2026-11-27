---
name: xpd-harden
description: Use xpd harden to find swallowed exceptions, fixed waits and flaky, skipped or nondeterministic tests. Verify fixes with the reliability checklist and determinism gate.
---

# xpd harden

Run `xpd harden` to answer one question. Does the code fail loudly, deterministically and honestly?

| Group | Finding kinds |
| --- | --- |
| Silent failure | `swallowed-exception`, `rule-issue` |
| Fixed waits and retries | `sleep-or-retry` |
| Unreliable tests | `disabled-test`, `flaky-test` |

Each module with a finding gets six checklist items, including test files. Run `xpd harden --questions` to read the questions and the facts that inform them. Set `goal` to `design` for a checklist fix.

At each gate, xpd repeats the first check after the checks pass. The exit status and the SHA-256 digest of stdout must agree with the first execution.

## Recorded run

1. **Start**. Record the current commit, paths and project checks. Start from a clean tracked tree. Run `xpd harden start . --check "your-project-check"`.
2. **List**. Read the findings and checklist with the bare command. Use JSON to get stable finding IDs and checklist item IDs. Run `xpd harden . --format json`.
3. **Plan**. Record an ordered list of fixes and any findings or checklist answers that have a reason. Name a finding ID or set a supported goal for each fix. A design goal names checklist items in `questions`. Under `answered`, give each item a verdict of `pass`, `na` or `accepted` and a reason. Run `xpd harden plan plan.json`.
4. **Approve**. Record approval for a fix that changes what callers or users see. Run `xpd harden approve fix-id`.
5. **Gate**. Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item. Run `xpd harden gate fix-id`.
6. **Drop**. Return to the run’s current commit and record a reason for dropping the fix. Run `xpd harden drop fix-id --reason "reason"`.
7. **Status**. Show the current run, its recorded answers and the next stage. Run `xpd harden status`.
8. **Finish**. Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure. Run `xpd harden finish`.
