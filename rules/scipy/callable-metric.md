Passing a metric function such as `euclidean` to `cdist` or `pdist` calls it once for every pair in Python. Pass the metric's name as a string, which runs it in C and was over 1,000 times faster.

```diff lang="python"
-D = cdist(X, Y, distance.euclidean)
+D = cdist(X, Y, "euclidean")
```

## What the rule reports

The rule reports a metric function of `scipy.spatial.distance` passed to `cdist` or `pdist` as
the metric, by position or as `metric=`. The metric's name as a string runs the same metric in C, and
on 1,000 by 1,000 points it ran 1,100 to 2,100 times faster.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes the metric's name as a string. `cosine` and `correlation` differ in the last bits, and complex input is cast to float.
