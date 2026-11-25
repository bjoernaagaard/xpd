Use `untangle` before a refactor of a codebase and after each round of it, as
[`xpd untangle`](../../../commands/untangle/) does, and it returns the same JSON. It reads the
files and the git history, and it changes no file. `summary` counts each kind in four groups, and
`findings` holds the strongest of each kind with its places and the evidence.

## Example

```python title="shop/orders.py"
from shop import pricing


def summarize(rows):
    pricing.RATE = 0.2
    return pricing.total(rows)
```

```python title="shop/pricing.py"
from shop import orders

RATE = 0.25


def total(rows):
    return sum(rows) * RATE


def report():
    return orders.summarize([1.0])
```

```json
{"paths":["shop"],"kinds":["import-cycle","shared-state"]}
```

```json
{"read":{"files":2,"python":2,"commits":"..."},"summary":"...","findings":["..."]}
```

## Errors

A path outside the project root fails before xpd runs. A `since` that names no commit fails with
the binary's message, and so does a `since` outside a git repository.
