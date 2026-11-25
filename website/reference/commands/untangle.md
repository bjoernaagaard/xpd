Use `xpd untangle` before a refactor of a codebase and after each round of it. It lists the places
where the code gets one question wrong, which is whether two pieces should change together. It
reads the files and the git history, and it runs no code.

| Group | What is wrong | Kinds |
| --- | --- | --- |
| Hidden link | Pieces must change together, and the code does not show it | `copy`, `changes-together`, `shared-state` |
| Needless link | Pieces are tied together and change for different reasons | `volatile-import`, `import-cycle`, `private-reach`, `vague-module` |
| Surplus | A piece has no reason of its own to change | `pass-through`, `unreferenced`, `constant-parameter` |
| Special case | Code exists for one situation, which the design did not account for | `one-value-branch`, `one-caller-parameter`, `swallowed-exception`, `sleep-or-retry`, `disabled-test` |

`copy` and `changes-together` are found in every programming language that ast-grep parses. The
other kinds are read from Python. A finding is evidence, so judge it against the code before you
change anything. The `xpd-untangle` skill, which [`xpd skill --json`](../skill/) prints, holds the
steps of a refactor that the counts measure, and `untangle.workflow.ts`, which runs those steps as a
durable workflow of T3 Code with a separate agent for each stage.

## Examples

```python title="shop/orders.py"
import time

from shop import pricing


def load(path, strict=False):
    with open(path) as handle:
        return [row for row in handle if row or not strict]


def summarize(rows, kind):
    if kind == "wholesale":
        rows = rows[:10]
    pricing.RATE = 0.2
    try:
        value = pricing.total(rows)
    except ValueError:
        pass
    time.sleep(1)
    return value


def first():
    return load("a.csv")


def second():
    return load("b.csv")
```

```python title="shop/pricing.py"
from shop import orders

RATE = 0.25


def total(rows):
    return sum(float(row) for row in rows) * RATE


def report():
    return total(orders.first()), orders.second(), orders.summarize([], "retail")
```

```console
$ touch shop/__init__.py
$ xpd untangle shop --format text
xpd untangle: 7 findings in 3 files, 3 of them Python, with 0 commits of history
No git history was read, so no finding rests on how the files changed.

Hidden links: pieces that must change together, where the code does not show it: 1
  shared-state: 1
    29ce6e4b  shop/orders.py:14  This writes RATE of the module pricing from outside it.

Needless links: pieces tied together that change for different reasons: 1
  import-cycle: 1
    8de3d24b  shop/orders.py:3, shop/pricing.py:1  These 2 modules import one another in a circle, so none of them can change alone.

Surplus: pieces with no reason of their own to change: 2
  unreferenced: 1
    c17ac665  shop/pricing.py:10  No Python file that was read mentions report, apart from this definition.
  constant-parameter: 1
    0bea3ced  shop/orders.py:6  None of the 2 calls of load passes strict, so it always has its default.

Special cases: code for one situation, which the design did not account for: 3
  one-value-branch: 1
    ccd51d7f  shop/orders.py:12  This branch runs for one value: kind == "wholesale".
  swallowed-exception: 1
    ebae19e6  shop/orders.py:17  This catches an exception and does nothing with it.
  sleep-or-retry: 1
    0c77beff  shop/orders.py:19  This waits a fixed time for something else to finish.
```

Each finding has its id, its places and what was measured. `--kind` keeps some kinds, and `--limit`
sets how many findings of each kind are listed, which is 20 without it and every one with 0.

```console
$ xpd untangle shop --kind import-cycle,shared-state --format json | jq -c '.findings[] | {kind, places: [.places[] | "\(.path):\(.line)"]}'
{"kind":"shared-state","places":["shop/orders.py:14"]}
{"kind":"import-cycle","places":["shop/orders.py:3","shop/pricing.py:1"]}
```

The summary holds every count, also of the kinds that `--kind` leaves out of the list.

```console
$ xpd untangle shop --format json | jq -c '.summary.groups[] | {group, count}'
{"group":"hidden-link","count":1}
{"group":"needless-link","count":1}
{"group":"surplus","count":2}
{"group":"special-case","count":3}
```

## What each kind measures

| Kind | What xpd reports |
| --- | --- |
| `copy` | A whole definition or block of at least 5 lines and 60 tokens that is written out token for token in two or more places. Comments and layout are not compared |
| `changes-together` | Two files that changed in the same commits at least 5 times, and in at least half of the commits of the one that changed less. A commit of more than 25 files is left out. A pair of Python files is listed only when neither imports the other |
| `shared-state` | A write to a name of another module, or a function that writes a module level name through `global` |
| `volatile-import` | A module that imports a module with at least twice as many commits and at least 8, counted from the first commit of the importer. A test and an `__init__.py` are left out |
| `import-cycle` | Modules that import one another in a circle |
| `private-reach` | An import or a read of a name of another module that starts with one underscore |
| `vague-module` | What [`python/vague-module-name`](../../rules/python/vague-module-name/) reports |
| `pass-through` | What [`python/pass-through-function`](../../rules/python/pass-through-function/) reports |
| `unreferenced` | A function or class at the top of a module that no Python file mentions apart from its definition. A decorated definition, a test and a name that a string holds are left out |
| `constant-parameter` | A parameter that none of two or more calls passes, or that each of them gives the same literal |
| `one-value-branch` | An `if` without `elif` that compares something with one string. Each value is one finding with every place that checks it, and a value that many places check comes first. Tests are left out |
| `one-caller-parameter` | A parameter with a default that one of three or more calls passes |
| `swallowed-exception` | An `except` that only passes or continues, or `contextlib.suppress` |
| `sleep-or-retry` | `time.sleep`, `asyncio.sleep` or a retry decorator, and in a test what [`python/test-sleep`](../../rules/python/test-sleep/) and [`python/test-rerun`](../../rules/python/test-rerun/) report |
| `disabled-test` | What [`python/test-skip`](../../rules/python/test-skip/) reports |

The parameters of a function are judged only when every mention of its name is a call whose
arguments xpd can match to them, so a function that is passed as a value is left alone.
`unreferenced` cannot see a caller in another language or outside the paths, so check before you
delete.

## Measure a refactor

The count of each kind is the measure of a refactor, and each finding has an `id` that it keeps
from commit to commit. `--since <commit>` reads the files of that commit too and compares.

| In the JSON | Meaning |
| --- | --- |
| `was` beside each `count` | The count at the commit |
| `"new": true` on a finding | The commit had no finding of this id, so the change added it |
| `fixed` | The findings of the commit that are gone today |
| `summary.new` and `summary.fixed` | How many of each |

```sh
xpd untangle --since main
```

xpd writes the files of the commit to a temporary folder, so the working tree and the index stay
as they are. An id is made from the kind and from what the finding is about, such as the file, the
name or the value, and not from a line, so it stays the same when code moves within a file. It
changes when the file is renamed. Of two findings that are alike in one file, the first in the
file has the first id. Without git, `changes-together` and `volatile-import` report nothing, and
`--since` fails.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | It printed the ledger, with or without findings |
| 2 | It could not run, as when `--since` names no commit |
