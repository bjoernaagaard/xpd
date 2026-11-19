Since pandas 2.0, `str.replace` reads its pattern as plain text, so a regular expression replaces nothing. Pass `regex=True`.

```diff lang="python"
-digits = df["phone"].str.replace(r"\D", "")
+digits = df["phone"].str.replace(r"\D", "", regex=True)
```

## What the rule reports

The rule reports `df["s"].str.replace(r"\d+", "")`, which replaces nothing, because `str.replace` reads its pattern as plain text since pandas 2.0. The rule reports only a pattern that can't be meant as plain text, as the table shows.

| Pattern | Reported |
| --- | --- |
| a class such as `\d`, a set in brackets, `.*` or `.+`, a `^` at the start, or two choices joined by a pipe | yes |
| `"."`, `"$"` and `"("` | no |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `regex=True`.

The fix is unsafe, because the code can mean a set in brackets as plain text.
