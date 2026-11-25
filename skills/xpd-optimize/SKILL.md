---
name: xpd-optimize
description: Make a script or a codebase shorter, simpler and faster with xpd optimize, which lists the work that no rule of xpd check reports. Use when the user asks to optimize, simplify, shorten or speed up code, to review code that the linter passes, or to find what could be done with less code.
---

# Make code shorter and faster with xpd

Use this guide when `xpd check` is clean and the code could still be shorter, simpler or faster.

A rule reads one call at a time, so it finds a slow or wrong use of a function. It cannot know
what the code is for. The work that is left shows only when two places of the code are read
together, and `xpd optimize` lists the part of it that the syntax shows. It reads the files and
runs no code.

| Group | What the code does | Where it comes from |
| --- | --- | --- |
| Written twice | The same code stands in two places but for a few names | `near-copy` of `xpd optimize`, and `copy` of `xpd untangle` |
| Done again | A value is computed more than once, and one computation serves | `repeated-call` and `loop-invariant-call` of `xpd optimize` |
| Already guaranteed | An argument, a check, a sort or a copy changes nothing, because an earlier step or the library already ensures it | `default-argument` of `xpd optimize`, and your reading |
| Larger than the answer | A matrix, a list or a join is much larger than the result it is reduced to | `matrix-reduced` of `xpd optimize`, and your reading |
| In the way | A wrapper only forwards, a parameter never varies, and nothing calls a function | `pass-through`, `constant-parameter` and `unreferenced` of `xpd untangle` |
| Already in a library | Code rebuilds what a dependency of the project provides | `xpd find "<task>"`, then `xpd api <name>` |

`near-copy` and `copy` are found in every programming language. The other kinds are read from
Python. In another language, find them by reading, with the table as the list.

The work is a loop of five stages, and no stage grades its own work. The commands find, you
judge, each change is proven by a comparison of the old code with the new, and the command
measures.

```text
record the commit the work starts from, and run the project's checks
repeat
  list     xpd optimize, xpd untangle and your own reading
  judge    read the code behind each finding, and drop what the code disproves
  change   one change at a time, with every caller
  prove    the old and the new code give the same result on the same input
  measure  xpd optimize --since <start>, the line count and the run time
until a round keeps no change
```

## Survey

Find the commands the project checks itself with, in its hook, task and CI files: the formatter,
the linters, `xpd check`, the type checker and the tests. Run each one, and fix what `xpd check`
reports before you go on, because a clean linter is the starting point and no finding below
repeats a rule.

Record the baseline: the commit, the line count of the scope, the result of each check, and the
run time of one representative workload when the code has one.

## List

Change nothing in this stage.

```sh
xpd optimize <paths> --format json
xpd untangle <paths> --kind copy,pass-through,constant-parameter,unreferenced --format json
```

Each finding has an `id`, its `places` and the `evidence` that was measured. The commands list
what the syntax shows, and the rest needs the purpose of the code. So for each function, class and
script in scope, write one sentence that names its inputs and its result and leaves out how it
gets there. Describe the shortest program that gives the same result with the libraries the
project already uses, and write down each difference between the code and that program, with its
file and line. Read the callers together with the code, because a guarantee often lives in one
place and its check in another.

| What you look for | Command that helps |
| --- | --- |
| Twins that differ in more than a few names | `xpd rules outline <paths>` lists every definition with its members |
| The callers of a function, and what each passes | `xpd rules run --pattern '<name>($$$ARGS)' <paths>` |
| What a library call returns, and its defaults | `xpd api <Class.method>` |
| A library function for a task the code does by hand | `xpd find "<task>"` |
| Which step of a pipeline reads what | `xpd graph <file>`, and `xpd graph impact` for one column |
| A move from pandas to Polars, or from eager to lazy | `xpd graph map` |

## Judge

Read the code behind each finding. A finding is a place to read and not yet a change.

| Finding | Check before you change it |
| --- | --- |
| `near-copy` | The texts that differ are all values that one parameter can carry, and the two stretches do not change for different reasons |
| `repeated-call` and `loop-invariant-call` | The call has no effect and gives the same value each time. xpd cannot see an effect that the name does not show, or a change through another name |
| `default-argument` | The default of the installed version is the one in the index. `xpd api <name>` prints it |
| `matrix-reduced` | The two sides are large enough to matter, and the result can be computed without the matrix, as with a sort, a search or one pass |

Look for the one change behind several findings before you fix them one by one. A data shape that
fits the work often removes a conversion at every step. Search for callers outside the scope
before you rename or remove a public name. Then sort each change.

| The change | Keep it when |
| --- | --- |
| Removes code and keeps the result | The result is proven equal |
| Makes the code faster and no longer | The result is proven equal. A speedup you did not measure is reported as a guess. |
| Makes the code faster and longer or harder to read | A measurement on the representative workload shows a gain the user would notice |
| Changes what a user can notice: a result beyond rounding error, a public name, a message or an error | The user agrees |

## Change and prove

Apply one change at a time, and update every caller with it. Then run the proof that fits.

| The code is | Proof |
| --- | --- |
| A data script that assigns `result` | `xpd equiv <script.py> --since HEAD` prints `{"equivalent": true}` |
| A function or a class | The old and the new version run on the same inputs, including an empty one and one at each boundary, and the outputs match |
| Covered by the project's tests | The tests pass as they did at the baseline |

Measure a claim about speed on the same input before and after, with the time in units, and with
a second input size when the claim is about scaling. Drop a change that its proof or its
measurement does not support, and say that you dropped it.

## Measure

```sh
xpd optimize <paths> --since <start>
xpd check <paths>
```

`--since` marks each finding that the start commit did not have as `new` and lists the ones that
are gone under `fixed`. A change is done when the finding it was for is under `fixed`, no finding
is `new`, and every check of the survey passes as it did at the baseline. A finding you judged and
left in place stays in the ledger, so name it in the report with the reason.

Run the loop again on the changed code, and stop when a round keeps no change.

## Report

Give the line count, the counts of `xpd optimize` and the measured times before and after, each
change with its kind and the proof you ran for it, and the findings you left as they are with the
reason.

## A recorded run

`xpd optimize start <paths> --check "<command>"` begins a run that xpd records in the git
folder of the repository, so it survives the end of a session. `xpd optimize plan <file>` takes
the changes in order, each with the ids of the findings it removes, and the accepted findings
with their reasons. After each commit, `xpd optimize gate <change>` runs the checks and
compares the findings, and exits 1 when the change is not kept. `xpd optimize status` prints
the stage that comes next, and `xpd optimize finish` ends the run. The `xpd-untangle` skill
describes each stage.
