Three functions called one inside the other read from the inside out, so the first step is written last. Chain them with `pipe`, which lists the steps in the order they run.

```diff lang="python"
 df = pd.read_csv("sales.csv")
-result = add_shares(add_totals(clean(df)))
+result = df.pipe(clean).pipe(add_totals).pipe(add_shares)
```

## What the rule reports

The rule reports three functions called one inside the other on a frame, as in `add_shares(add_totals(clean(df)))`. `DataFrame.pipe(f)` calls `f(frame)`, and the two forms gave equal frames. The three functions must be plain names and not Python builtins. xpd must also know that the innermost argument is a frame, which means that the argument is one of the following:

- a call such as `pd.read_csv(...)` or `pd.DataFrame(...)`
- a variable whose nearest earlier assignment is such a call
- a parameter annotated `pd.DataFrame`

The fix is unsafe, because the chain raises AttributeError when the first or the second function returns something without `pipe`, such as a dict. [`xpd graph`](https://bjoernaagaard.github.io/xpd/commands/graph/) reads the pipe form as a pipeline of steps.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.pipe(clean).pipe(add_totals).pipe(add_shares)`.
