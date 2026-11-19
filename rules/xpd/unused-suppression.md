An `ast-grep-ignore` comment that hides no issue only adds noise, and it can hide a real issue later when the code below it changes. Delete the comment.

```diff lang="python"
 df = pd.DataFrame({"a": [1]})
-# ast-grep-ignore: pandas/no-inplace
+
 df = df.dropna()
```

## What the rule reports

The rule reports a comment such as `# ast-grep-ignore: pandas/no-inplace` when no rule that it names matches the line it covers. The severity is hint, so the report never changes the exit status of `xpd check`.

A comment counts as used when the shape of a rule matches under it, also when one of xpd's own checks would have dropped that match. So xpd may stay silent about a comment that hides nothing, and it never reports a comment that hides an issue.

A run with `--rule` or `--severity` skips this check, because a rule that the run leaves out could use the comment.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fix of this rule. Check the result, because an unsafe fix can change what the code does.

The fix deletes the text of the comment and leaves an empty line in its place.
