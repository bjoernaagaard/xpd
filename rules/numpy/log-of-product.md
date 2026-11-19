A product can overflow to inf or underflow to 0 before the log is taken. Sum the logs instead.

```diff lang="python"
-log_likelihood = np.log(np.prod(probabilities))
+log_likelihood = np.sum(np.log(probabilities))
```

## What the rule reports

The rule reports `np.log(np.prod(x))`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.sum(np.log(x))`.
