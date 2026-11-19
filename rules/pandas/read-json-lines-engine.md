`read_json(lines=True)` without an engine parses one line at a time. Pass `engine="pyarrow"`, which read the file 25 times faster.

```diff lang="python"
-df = pd.read_json(path, lines=True)
+df = pd.read_json(path, lines=True, engine="pyarrow")
```

## What the rule reports

The rule reports `pd.read_json(path, lines=True)` without an engine, and `pd.DataFrame([json.loads(line) for line in f])`. `read_json` with the pyarrow engine was 25 times faster than without an engine, and 48 times faster than the `json.loads` loop, measured with the reader's default threads on a 32-core machine. The rule leaves a call alone that reads from a `StringIO`, because the pyarrow engine raises on a text buffer.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `engine="pyarrow"`. The `json.loads` loop has no fix.

The fix is unsafe, because pyarrow can pick another dtype for a column.
