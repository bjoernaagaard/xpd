---
name: xpd-optimize
description: Reduce measured CPU, memory, and I/O waste with xpd optimize. Find repeated computation, linear scans, and oversized intermediates, and gate every change on workload timing and profiling.
---

# Optimize measured cost with xpd

Use `xpd optimize` to eliminate CPU, memory, and I/O work that the result does not need, proving every change against a real workload.

| Group | Finding kinds |
| --- | --- |
| Performance rules | `rule-issue` |
| Repeated work | `repeated-call`, `repeated-mask`, `loop-invariant-call` |
| Oversized intermediates and scans | `matrix-reduced`, `materialized-for-one`, `linear-scan-in-loop` |

Each function with a finding or at least 5 percent of a workload's CPU time gets six performance checklist items. Run `xpd optimize --questions` to read the catalog. Supported fix goals are `faster`, `both`, and `design`.

## Workloads and adapters

- **Command benchmark (`--bench "<workload>"`)**: time an end-to-end workload at `start` and each `gate`.
- **Python profiler adapter (`--adapter perf.py`)**: define `workloads()` returning a dict of workload names to zero-argument callables.
- **Command adapter**: print JSON with a `workloads` array where each item has `name`, `median_ms`, `peak_bytes`, and `functions` (`file`, `name`, `line`, `cpu_ms`, `cumulative_cpu_ms`, `cpu_share`, `calls` using project-relative paths and fractional shares).

## Recorded run

1. **Start**: run `xpd optimize start . --check "<project-check>" [--bench "<workload>"] [--adapter perf.py]` from a clean commit.
2. **List**: run `xpd optimize . [--adapter perf.py] --format json` to read finding IDs, hotspot measurements, and checklist items.
3. **Plan**: write `plan.json` with ordered `fixes` (naming finding IDs or a `faster`, `both`, or `design` goal), `accepted` findings with reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd optimize plan plan.json`.
4. **Approve**: run `xpd optimize approve <fix-id>` after user approval for a fix that changes public interfaces.
5. **Gate**: commit one fix and run `xpd optimize gate <fix-id>`.
6. **Drop**: restore the run's current commit and run `xpd optimize drop <fix-id> --reason "<why>"` if a fix is abandoned.
7. **Status and finish**: run `xpd optimize status` to see the next stage, and `xpd optimize finish` once every open fix, new finding, and checklist item is resolved.
