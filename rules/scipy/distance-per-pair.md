Calling a distance function once for every pair of points runs Python for each pair. Stack the points into arrays and call `cdist` or `pdist` once, which was about 1,000 times faster.

```diff lang="python"
-D = np.array([[distance.euclidean(a, b) for b in Y] for a in X])
+D = distance.cdist(X, Y, "euclidean")
```

## What the rule reports

The rule reports a metric function of `scipy.spatial.distance`, such as `euclidean`, `cityblock`
or `cosine`, called inside a comprehension, a generator or a loop body. On 1,000 by 1,000 points,
`cdist` ran 960 to 2,100 times faster.

## How to fix it

xpd has no automatic fix, so make the change by hand.
