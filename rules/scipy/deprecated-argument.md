These arguments warn on SciPy 1.18.1, and SciPy will remove them. Delete the argument or use the replacement that the message names.

```diff lang="python"
-power = signal.lombscargle(t, y, freqs, precenter=True)
-res = optimize.linprog(c, A_ub=A, b_ub=b, method="interior-point")
+power = signal.lombscargle(t, y - y.mean(), freqs)
+res = optimize.linprog(c, A_ub=A, b_ub=b, method="highs")
```

## What the rule reports

The rule reports arguments that warn on SciPy 1.18.1 and that SciPy will remove. The forms whose
message names `variant=` are skipped on SciPy older than 1.17.

| Function | Argument that warns |
| --- | --- |
| `signal.lombscargle` | `precenter=`, and a fourth or fifth positional argument |
| `linalg.qr` | `lwork=`, by keyword or by position |
| `stats.anderson_ksamp` | `midrank=`, by keyword or by position |
| `optimize.linprog` | `method='simplex'`, `'interior-point'` or `'revised simplex'` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix deletes `precenter=False` from lombscargle, which is what the default does. The unsafe fix deletes `lwork=` from qr, because an `lwork` below the number of rows raised ValueError. The other forms have no fix, because `precenter=True` needs `y` centred too, the positional forms need the names of the arguments after them, and `variant=` and `method='highs'` change the result or its options.
