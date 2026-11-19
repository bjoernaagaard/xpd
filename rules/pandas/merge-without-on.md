Without `on=`, `merge` joins on every column name the two frames share, so a new shared column silently changes the join. Name the keys with `on=`.

```diff lang="python"
-joined = left.merge(right, validate="many_to_one")
+joined = left.merge(right, on="key", validate="many_to_one")
```

## What the rule reports

The rule reports `merge` without `on=`, which joins on every shared column name.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Name the key columns that you mean to join on.
