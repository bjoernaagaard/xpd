`factorial` overflows to inf above 170, so a binomial coefficient built from factorials gives inf or nan for larger numbers. Call `special.comb`.

```diff lang="python"
-ways = special.factorial(n) / (special.factorial(k) * special.factorial(n - k))
+ways = special.comb(n, k)
```

## What the rule reports

The rule reports `special.factorial(n) / (special.factorial(k) * special.factorial(n - k))`, and
the same with the factors in another order or divided one at a time. `factorial` overflows to inf
above 170, so at n = 200 and k = 3 the ratio gave nan. `comb` lost no digits at the same n and k.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `special.comb(n, k)`. `factorial` imported by name is fixed only in a file that imports `comb` from `scipy.special`.
