`np.exp` overflows to inf above 709 and underflows to 0 below -745, so a log of a sum of exponentials can be inf where the answer is finite. Use `np.logaddexp` or a similar function.

```diff lang="python"
-softplus = np.log(1 + np.exp(z))
+softplus = np.logaddexp(0, z)
```

## What the rule reports

The rule reports `np.log(np.exp(a) + np.exp(b))`, `np.log(1 + np.exp(x))`, `np.log1p(np.exp(x))`, `np.log(np.sum(np.exp(x)))` and `np.exp(x) / np.sum(np.exp(x))`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.logaddexp(a, b)`, `np.logaddexp(0, x)`, `np.max(x) + np.log(np.sum(np.exp(x - np.max(x))))` and `np.exp(x - np.max(x)) / np.sum(np.exp(x - np.max(x)))`. The last bits change, an array of only -inf gives NaN, and an empty array raises. A softmax summed along an axis, and an operand that calls a function, have no fix.
