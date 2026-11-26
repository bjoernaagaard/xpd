Use `xpd optimize` before you simplify or speed up code, and after each round of it. It combines
[`xpd check`](../check/) and any project rule packs (`sgconfig.yml` / `xpd.yml`) with
whole-function and cross-place analysis, listing the places where code does work that a shorter or
faster program would not do. It reads the files and runs no code.

| Group | What the code does | Kinds |
| --- | --- | --- |
| Rulekit | A rule of `xpd check` or a project rule pack fires | `rule-issue` |
| Written twice | The same code stands in two places but for a few names | `near-copy` |
| Done again | A call, boolean mask, masked slice or loop body is computed more than once | `repeated-call`, `repeated-mask`, `loop-invariant-call` |
| Already guaranteed | An argument changes nothing, because the library does the same without it | `default-argument` |
| Larger than the answer | A pairwise matrix is reduced or contracted, a full diff/sort/cumsum is built for one element, or a loop scans a list or frame on every pass | `matrix-reduced`, `materialized-for-one`, `linear-scan-in-loop` |

`near-copy` is found in every programming language that ast-grep parses. The other kinds are read
from Python (plus any languages covered by the project's own rules). A finding is a place to read
and not yet a change, because the syntax does not show every difference, so read the code before
you change it. The `xpd-optimize` skill, which [`xpd skill --json`](../skill/) prints, holds the
steps from a finding to a change with proof that the result is the same.

## Examples

`xpd check` reports no issue in this file.

```python title="shop/prices.py"
import numpy as np
import pandas as pd


def retail_total(orders):
    """The total of the retail orders, with tax."""
    kept = orders[orders["kind"] == "retail"]
    amounts = kept["price"].to_numpy() * kept["count"].to_numpy()
    taxed = amounts * 1.25
    return float(np.round(taxed.sum(), 2))


def wholesale_total(orders):
    """The total of the wholesale orders, with tax."""
    kept = orders[orders["kind"] == "wholesale"]
    amounts = kept["price"].to_numpy() * kept["count"].to_numpy()
    taxed = amounts * 1.25
    return float(np.round(taxed.sum(), 2))


def report(orders, rates):
    rows = []
    for rate in rates:
        means = orders.groupby("kind")["price"].mean()
        rows.append(means * rate)
    spread = orders["price"].to_numpy().max() - orders["price"].to_numpy().min()
    return rows, spread, pd.concat([orders, orders], axis=0)


def nearest(prices, levels):
    gaps = np.abs(prices[:, None] - levels)
    return gaps.min(axis=1)
```

```console
$ xpd optimize shop --format text
xpd optimize: 5 findings in 1 file (32 lines), 1 of them Python

Rulekit: issues that `xpd check` and the project's own rules already report: 0

Written twice: code that is the same but for a few names, which one function with a parameter replaces: 1
  near-copy: 1
    d0d923d9  shop/prices.py:5-10, shop/prices.py:13-18  These 2 stretches of 6 lines are the same code but for `retail` and `wholesale`, `retail_total` and `wholesale_total`.

Done again: a value computed more than once, which one computation serves: 2
  repeated-call: 1
    e948b2af  shop/prices.py:26  report computes `orders["price"].to_numpy()` 2 times, on line 26, and changes nothing that the call reads in between.
  loop-invariant-call: 1
    066069f5  shop/prices.py:24  The loop on line 23 of report computes `orders.groupby("kind")["price"].mean()` on every pass, and changes nothing that the call reads.

Already guaranteed: an argument that changes nothing, because the library does the same without it: 1
  default-argument: 1
    5d90ce0e  shop/prices.py:27  `axis=0` passes the default of `pandas.concat`.

Larger than the answer: work or storage that is much larger than the result: 1
  matrix-reduced: 1
    607bb2c5  shop/prices.py:31, shop/prices.py:32  `prices[:, None] - levels` holds one value for each pair of elements of its two sides, and line 32 of nearest reduces it.
```

Each finding has its id, its places and what was measured. `--kind` keeps some kinds, and `--limit`
sets how many findings of each kind are listed, which is 20 without it and every one with 0.

```console
$ xpd optimize shop --kind near-copy,matrix-reduced --format json | jq -c '.findings[] | {kind, places: [.places[] | "\(.path):\(.line)"]}'
{"kind":"near-copy","places":["shop/prices.py:5","shop/prices.py:13"]}
{"kind":"matrix-reduced","places":["shop/prices.py:31","shop/prices.py:32"]}
```

The summary holds every count, also of the kinds that `--kind` leaves out of the list.

```console
$ xpd optimize shop --format json | jq -c '.summary.groups[] | {group, count}'
{"group":"rulekit","count":0}
{"group":"written-twice","count":1}
{"group":"done-again","count":2}
{"group":"already-guaranteed","count":1}
{"group":"larger-than-the-answer","count":1}
```

## What each kind measures

| Kind | What xpd reports |
| --- | --- |
| `rule-issue` | Any issue reported by `xpd check` and the project's own rule packs (`sgconfig.yml` / `xpd.yml`) |
| `near-copy` | Two or more definitions or blocks of at least 5 lines and 60 tokens with the same structure, differing in at most 4 pairs of names or values and at most a quarter of their tokens. Comments, docstrings and layout are not compared. Token-for-token copies are reported by [`xpd untangle`](../untangle/) |
| `repeated-call` | A call that one function writes out more than once, when no line between the calls assigns, deletes or calls a method on a name that the call reads, and the calls are not in different arms of one branch |
| `repeated-mask` | A boolean mask or masked subscript (such as `d[near]` or `total >= MIN_MW`) evaluated more than once in one function with nothing it reads written in between |
| `loop-invariant-call` | A call inside a `for` loop or comprehension whose value is used, when the loop assigns none of the names that the call reads. Calls under `return` or `raise` and calls in `while` loops are left out |
| `default-argument` | A keyword argument with a literal value that equals the default of the function it is passed to, as the index of [`xpd api`](../api/) holds it. A method of a value whose class the file does not name counts when every class with a method of that name, in the packages the file imports, has this default |
| `matrix-reduced` | An expression that indexes one side with `None` or `np.newaxis`, building one value for each pair of elements, and a reduction (`sum`, `mean`, `logsumexp`, etc.) or matrix contraction (`@`, `dot`, `matmul`, `einsum`) in the same function or in a caller of the function that returns it, tracking local variable bindings |
| `materialized-for-one` | Building a full `diff`, `sorted`/`sort`, `argsort`, `cumsum` or `sort_values` only to read one element (`[0]`, `[-1]`) or a small `.head(k)`/`.tail(k)` |
| `linear-scan-in-loop` | Testing membership (`in` / `not in`) against a list or filtering a whole frame (`df[df[...] == ...]`) on every pass of a loop |

The calls and scans of `Done again` and `Larger than the answer` are read from functions outside
test files, since a test repeats calls to say what it checks. A call is left out when its name
says that it has an effect or gives a new value each time, such as `append`, `print`, `open`,
`next`, `random` or a class that is called to make an object, and when it costs nothing worth
saving, such as `len` or `dict.get`.

xpd cannot see that a function has an effect its name does not show, or that a call changes an
object through another name. A default is the one of the library version in the index, which
[`xpd api`](../api/) prints, and it may differ in the version the project has installed.

## Measure a round of work

The count of each kind measures a round of simplification, and each finding has an `id` that it
keeps from commit to commit. `--since <commit>` reads the files of that commit too and compares.

| In the JSON | Meaning |
| --- | --- |
| `was` beside each `count` | The count at the commit |
| `"new": true` on a finding | The commit had no finding of this id, so the change added it |
| `fixed` | The findings of the commit that are gone today |
| `summary.new` and `summary.fixed` | How many of each |

```sh
xpd optimize --since main
```

xpd writes the files of the commit to a temporary folder, so the working tree and the index stay
as they are. An id is made from the kind and from what the finding is about, such as the file, the
function and the call, and not from a line, so it stays the same when code moves within a file. It
changes when the file is renamed. Outside a git repository `--since` fails.

## A recorded run

`xpd optimize start`, `plan`, `approve`, `gate`, `drop`, `status` and `finish` record a run of
changes in `.git/xpd-optimize.json`, tracking finding counts, total code lines, check results and
an optional benchmark workload recorded at [`start`](./start/) (1 warmup plus 5 timed runs by
default). A plan may include fixes that name static finding ids or judgment-based fixes with
`"findings": []` and `"goal"` set to `"shorter"`, `"faster"` or `"both"`. A gate keeps a change
when each check passes as it did at the start, the count of findings is not higher, at least one
named finding is gone (or its goal is met), and any recorded benchmark did not regress by more
than 15%.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | It printed the ledger, with or without findings, or the stage of a run was done |
| 1 | A gate did not keep its change, or `finish` did not end the run |
| 2 | It could not run, as when `--since` names no commit |
