Collect inside a loop can execute a separate query on each iteration. Build the queries first and use collect_all when they are independent.

```diff lang="python"
-lf.collect()
+pl.collect_all([lf] * 3)
```

## What the rule reports

Three collect calls produced the same three frames as collect_all in the probe.

## How to fix it

Edit the code using the replacement in the example.
