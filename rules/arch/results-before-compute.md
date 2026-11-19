These results of `SPA`, `StepM`, `MCS` and `RealityCheck` exist only after `compute`, so reading them first raises RuntimeError. Call `compute` first.

```diff lang="python"
 mcs = MCS(losses, size=0.1, seed=0)
+mcs.compute()
 kept = mcs.included
```

## What the rule reports

The rule reports `pvalues`, `critical_values`, `better_models`, `superior_models`, `included` and `excluded` read from an `SPA`, `StepM`, `MCS` or `RealityCheck` in a file that never calls `compute` on that object.

The rule leaves alone a variable that the file also reads without a dot after it, such as an
argument of a call, an item of a tuple or a list, or the right side of an assignment. Such a
variable can reach `compute` under another name, as in `for test in (spa, mcs): test.compute()`.
Both reports, on arch's own code, have wrong advice, because they are rows of a table in arch's
tests that pass a bad `pvalue` to `critical_values` to check the ValueError that comes first.

## How to fix it

xpd has no automatic fix. `compute` must run once before the first read, in a statement that the match does not reach. Make the change by hand.

The tests run their bootstrap in `compute` and not when they are created. On a new object each
read raised RuntimeError, on arch 9.0.0 and on 8.0.0.

| Class | Read | Message |
| --- | --- | --- |
| `SPA` | `pvalues`, `critical_values()`, `better_models()` | compute must be called before pvalues are available |
| `StepM` | `superior_models` | compute must be called before accessing superior_models |
| `MCS` | `pvalues`, `included`, `excluded` | Must call compute before accessing results |

The rule reports a result read straight from the constructor call. It also reports a result read
from a variable that the file assigns from one of the four classes and from nothing else, when no
line of the file calls `compute` on that variable. It does not follow the order of the lines, so
a read before a later `compute` is not reported.
