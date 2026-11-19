These calls rely on a default that warns on SciPy 1.18.1 because it will change. Pass the argument that the message names, with the value you mean.

```diff lang="python"
-x, info = gmres(A, b, callback=log)
-A = mmread("matrix.mtx")
-result = stats.anderson(sample)
+x, info = gmres(A, b, callback=log, callback_type="legacy")
+A = mmread("matrix.mtx", spmatrix=True)
+result = stats.anderson(sample, method="interpolate")
```

## What the rule reports

The rule reports calls that rely on a default that warns on SciPy 1.18.1. A form whose message
names an argument added in SciPy 1.17 or 1.18 is skipped on an older SciPy.

| Call | Default that warns | Fix |
| --- | --- | --- |
| `gmres` with `callback=` | no `callback_type=` | `callback_type='legacy'` |
| `stats.anderson` | no `method=` | none |
| `stats.anderson_ksamp` | no `variant=` | none |
| `io.hb_read`, `io.mmread` | no `spmatrix=` | `spmatrix=True` |
| `ndimage.zoom` with `grid_mode=True` | mode `'constant'`, the default, or `'wrap'` | `mode='grid-constant'` when the call passes no `mode=` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `callback_type='legacy'` for gmres with a callback, and `spmatrix=True` for hb_read and mmread. Each is what SciPy 1.18.1 does after it warns. Unsafe: `mode='grid-constant'` for zoom with `grid_mode=True` and no `mode=`, because the grid mode pads in another way and the values near the edges change. The other forms have no fix, because the result of anderson and anderson_ksamp changes type, and an explicit zoom `mode=` is outside the match.
