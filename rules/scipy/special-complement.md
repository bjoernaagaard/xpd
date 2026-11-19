`1 - special.f(x)` drops the digits of a small result and gives 0 once the function rounds to 1. Call the complement function that the message names.

```diff lang="python"
-cdf = 0.5 * (1 + special.erf(x / np.sqrt(2)))
+cdf = 0.5 * special.erfc(-x / np.sqrt(2))
```

## What the rule reports

The rule reports 1 plus or minus a `scipy.special` function, where SciPy has a function that
keeps the digits of a small result. The old form lost 3 or more digits in the following ranges:

| Old form | Digits lost for | Write instead |
| --- | --- | --- |
| `1 - special.erf(x)` | x from 2.75 to 26.5 | `special.erfc(x)` |
| `1 - special.erfc(x)` | x below 6e-5 | `special.erf(x)` |
| `1 - special.gammainc(a, x)`, `1 - special.gammaincc(a, x)` | the tails, where the result falls below about 1e-4 | `special.gammaincc(a, x)`, `special.gammainc(a, x)` |
| `1 - special.betainc(a, b, x)` | x from 0.32 to 0.95, with a = 2 and b = 30 | `special.betaincc(a, b, x)` |
| `1 + special.erf(x)` | x below -2.75 | `special.erfc(-x)` |
| `1 - special.ndtr(x)` | x above 4.25 | `special.ndtr(-x)` |
| `1 - special.expit(x)` | x above 8 | `special.expit(-x)` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `special.erfc(x)`, `special.erf(x)`, `special.gammaincc(a, x)`, `special.gammainc(a, x)`, `special.betaincc(a, b, x)`, `special.erfc(-x)`, `special.ndtr(-x)`, `special.expit(-x)`. `betaincc` needs SciPy 1.11. `erf`, `erfc`, `gammainc`, `gammaincc` and `betainc` imported by name are fixed only in a file that imports the new name from `scipy.special`.
