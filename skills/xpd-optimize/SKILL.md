---
name: xpd-optimize
description: Make a script or a codebase shorter, simpler and faster with xpd optimize, combining rulekit checks, structural and performance findings, line-count tracking and benchmark gating. Use when the user asks to optimize, simplify, shorten or speed up code, to review code that the linter passes, or to find what could be done with less code.
---

# Make code shorter and faster with xpd

Use `xpd optimize` to review and improve a script or a whole codebase. It combines the project's
rule packs (`xpd check` and any `sgconfig.yml` / `xpd.yml` rules) with whole-function and
cross-place analysis, tracks code lines, and times a real workload before and after each change.
It runs in any agent or terminal with no workflow engine.

| Group | What the code does | Kinds in `xpd optimize` |
| --- | --- | --- |
| Rulekit | A rule of `xpd check` or a project rule pack fires | `rule-issue` |
| Written twice | The same code stands in two places but for a few names | `near-copy` (and `copy` of `xpd untangle`) |
| Done again | A call, boolean mask, masked slice or loop body is computed more than once | `repeated-call`, `repeated-mask`, `loop-invariant-call` |
| Already guaranteed | A keyword argument passes the library's own default | `default-argument`, and your reading |
| Larger than the answer | A pairwise matrix is reduced or contracted, a full diff/sort/cumsum is built for one element, or a loop scans a list or frame on every pass | `matrix-reduced`, `materialized-for-one`, `linear-scan-in-loop` |
| In the way | A wrapper only forwards, a parameter never varies, or nothing calls a function | `pass-through`, `constant-parameter`, `unreferenced` of `xpd untangle` |
| Already in a library | Code rebuilds what a dependency of the project provides | `xpd find "<task>"`, then `xpd api <name>` |

`near-copy` is found in every programming language that ast-grep parses. The other kinds are read
from Python (plus any languages covered by the project's own `sgconfig.yml` rules). In another
language, find them by reading with the table as the list.

The work is a recorded loop of five stages, and no stage grades its own work. `xpd` finds and
records the baseline, you judge and edit, and `xpd optimize gate` proves every change by running
the checks, counting lines, timing `--bench`, and comparing finding ids.

```text
xpd optimize start <paths> --check "<tests>" [--bench "<workload>"]
repeat
  list     xpd optimize <paths> --format json (plus reading for algorithmic and library wins)
  plan     xpd optimize plan <plan.json> (each fix names finding ids or a goal: shorter/faster/both)
  change   one fix at a time, with every caller, committed cleanly
  gate     xpd optimize gate <fix-id> (checks pass, findings do not rise, lines/bench proven)
until no fix is left
xpd optimize finish
```

## Survey and start

Find the commands the project checks itself with (formatter, linters, type checker, tests) and a
representative workload command when speed matters. Start the recorded run:

```sh
xpd optimize start <paths> --check "<test-command>" --bench "<workload-command>"
```

`start` records the commit, total code lines, baseline findings, each check's status, and the
median/min/max time of `--bench` (1 warmup + 5 timed runs by default, tunable with
`--bench-runs`). The state lives in `.git/xpd-optimize.json`, so it survives across sessions.

## List

Change nothing in this stage.

```sh
xpd optimize <paths> --format json
xpd untangle <paths> --kind copy,pass-through,constant-parameter,unreferenced --format json
```

Each finding has an `id`, its `places` and the `evidence` that was measured. The commands list
what the rules and syntax show, and the rest needs the purpose of the code. For each function,
class and script in scope, write one sentence that names its inputs and its result and leaves out
how it gets there. Describe the shortest program that gives the same result with the libraries the
project already uses, and note each difference with its file and line.

| What you look for | Command that helps |
| --- | --- |
| Twins that differ in more than a few names | `xpd rules outline <paths>` lists every definition with its members |
| The callers of a function, and what each passes | `xpd rules run --pattern '<name>($$$ARGS)' <paths>` |
| What a library call returns, and its defaults | `xpd api <Class.method>` |
| A library function for a task the code does by hand | `xpd find "<task>"` |
| Which step of a pipeline reads what | `xpd graph <file>`, and `xpd graph impact` for one column |
| A move from pandas to Polars, or from eager to lazy | `xpd graph map` |

## Judge and plan

Read the code behind each finding. A finding is a place to read and not yet a change.

| Finding | Check before you change it |
| --- | --- |
| `rule-issue` | Read the rule with `xpd rules --rule <name>` and fix the cause |
| `near-copy` | The differing tokens fit cleanly into parameters and the two stretches change for the same reason |
| `repeated-call`, `repeated-mask`, `loop-invariant-call` | The expression has no hidden side effect and nothing mutates its inputs in between |
| `default-argument` | The installed library version matches the default in `xpd api <name>` |
| `matrix-reduced` | The result can be computed without allocating the full pairwise matrix, such as by sorting, searching, chunking, or one pass |
| `materialized-for-one` | Indexing directly or using `min`/`max`/`argmin`/`argmax`/`sum`/`nsmallest` preserves edge cases (empty inputs, ties, NaNs) |
| `linear-scan-in-loop` | Converting the list to a `set` or grouping the frame once preserves order and semantics |

Group findings that share one root cause under a single fix. For an algorithmic or library
simplification that no static finding flagged, leave `"findings": []` and set `"goal"` to
`"shorter"`, `"faster"`, or `"both"`. Set `"changes_interface": true` whenever a fix changes a
public name, output beyond rounding error, message or error, and get the user's approval with
`xpd optimize approve <fix-id>` before gating it.

```sh
xpd optimize plan plan.json
```

`plan.json` lists `"fixes"` in order and `"accepted"` findings with one-sentence reasons.

## Change, prove and gate

Apply one fix at a time, update every caller, and prove equivalence:

| The code is | Proof |
| --- | --- |
| A data script that assigns `result` | `xpd equiv <script.py> --since HEAD` prints `{"equivalent": true}` |
| A function or a class | Run old and new code on the same inputs, including empty and boundary cases |
| Covered by tests | The project's test suite passes as at baseline |

Commit the fix and run the gate:

```sh
xpd optimize gate <fix-id>
```

The gate keeps the fix only when:
- every check that passed at `start` still passes,
- the total count of findings did not rise,
- at least one named finding is gone (or the fix's `"goal"` is met: fewer lines for `"shorter"`, at least 2% lower median `--bench` time for `"faster"`, or both for `"both"`), and
- if `--bench` is set, median time did not regress by more than 15%.

A fix that fails its gate exits 1 and records the reason; repair and re-gate once, or restore
with `git reset --hard <current>` and run `xpd optimize drop <fix-id> --reason "..."`.
`xpd optimize status` always prints the next stage.

## Finish and report

When no planned fix remains and a fresh `xpd optimize` scan yields no further worthwhile change:

```sh
xpd optimize finish
```

Report the before/after finding counts, line counts, measured benchmark times with units and
speedup, each kept fix with its proof, any dropped fixes, and each accepted finding with its
reason.
