A comprehension with more than one `for` is hard to read. Write a nested loop, or flatten with `[*xs for xs in lists]` on Python 3.15.

```diff lang="python"
 def checks(cases):
-    return [check for case in cases for check in case.checks]
+    found = []
+    for case in cases:
+        found.extend(case.checks)
+    return found
```

## What the rule reports

The rule reports a comprehension or generator expression with more than one `for`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The clauses run in the order of a nested loop, but the result stands before both of them. A
reader has to reorder the expression to follow it. On Python 3.15,
`[*case.checks for case in cases]` flattens the lists with one `for`.
