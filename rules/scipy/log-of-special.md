A `scipy.special` function such as `ndtr` underflows to 0 far in a tail, so its log is -inf there. Call the log form, such as `log_ndtr`.

```diff lang="python"
-loglike = np.log(special.ndtr(q * xb)).sum()
+loglike = special.log_ndtr(q * xb).sum()
```

## What the rule reports

The rule reports `np.log` of a `scipy.special` function that has a log form of its own, under
any import of `scipy.special`. The table shows where the old form overflowed or lost digits.

| Old form | Where the old form failed | Write instead |
| --- | --- | --- |
| `np.log(special.ndtr(x))` | -inf below -37.75 | `special.log_ndtr(x)` |
| `np.log(special.gamma(x))` | inf at 200 | `special.gammaln(x)` |
| `np.log(special.factorial(n))` | inf at 200 | `special.gammaln(n + 1)` |
| `np.log(special.beta(a, b))` | -inf at a = b = 1e6 | `special.betaln(a, b)` |
| `np.log(special.expit(x))` | 3 or more digits lost above 8 and below -710 | `special.log_expit(x)` |
| `np.log(special.softmax(x))` | -inf for an entry more than 745 below the largest | `special.log_softmax(x)` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `special.log_ndtr(x)`, `special.gammaln(x)`, `special.gammaln(n + 1)`, `special.betaln(a, b)`, `special.log_expit(x)`, `special.log_softmax(x)`. `log_expit` needs SciPy 1.8 and `log_softmax` SciPy 1.5. A function imported by name is fixed only in a file that imports the log form from `scipy.special`.
