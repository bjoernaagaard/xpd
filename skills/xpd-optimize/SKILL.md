---
name: xpd-optimize
description: Use xpd optimize for measured performance work. Find repeated computation, expensive scans and oversized allocations, and measure workload time, CPU and memory.
---

# xpd optimize

Run `xpd optimize` to answer one question. Does the program spend CPU, memory or I/O it does not need?

| Group | Finding kinds |
| --- | --- |
| Performance rules | `rule-issue` |
| Repeated work | `repeated-call`, `repeated-mask`, `loop-invariant-call` |
| Oversized intermediates and scans | `matrix-reduced`, `materialized-for-one`, `linear-scan-in-loop` |

A function with a finding or at least 5 percent of a workload’s CPU time gets six checklist items. Run `xpd optimize --questions` to read the questions and the facts that inform them. The supported fix goals are `faster`, `both` or `design`.

Pass a representative workload with `--bench` to time the whole command. Use `--adapter perf.py` on a list or start to measure individual workloads. The adapter defines `workloads()`, which returns a mapping from names to zero-argument callables. A shell adapter emits a JSON object with a `workloads` array. Each workload has `name`, `median_ms`, `peak_bytes` and `functions`. Each function has `file`, `name`, `line`, `cpu_ms`, `cumulative_cpu_ms`, `cpu_share` and `calls`. Use project-relative file paths and express shares as fractions.

## Recorded run

1. **Start**. Record the current commit, paths and project checks. Start from a clean tracked tree. Run `xpd optimize start . --check "your-project-check"`.
2. **List**. Read the findings and checklist with the bare command. Use JSON to get stable finding IDs and checklist item IDs. Run `xpd optimize . --format json`.
3. **Plan**. Record an ordered list of fixes and any findings or checklist answers that have a reason. Name a finding ID or set a supported goal for each fix. A design goal names checklist items in `questions`. Under `answered`, give each item a verdict of `pass`, `na` or `accepted` and a reason. Run `xpd optimize plan plan.json`.
4. **Approve**. Record approval for a fix that changes what callers or users see. Run `xpd optimize approve fix-id`.
5. **Gate**. Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item. Run `xpd optimize gate fix-id`.
6. **Drop**. Return to the run’s current commit and record a reason for dropping the fix. Run `xpd optimize drop fix-id --reason "reason"`.
7. **Status**. Show the current run, its recorded answers and the next stage. Run `xpd optimize status`.
8. **Finish**. Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure. Run `xpd optimize finish`.
