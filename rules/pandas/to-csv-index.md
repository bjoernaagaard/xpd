`to_csv` writes the index as a first column, which `read_csv` reads back as `Unnamed: 0`. Pass `index=False`, or `index=True` when the index holds data.

```diff lang="python"
-df.to_csv(path)
+df.to_csv(path, index=False)
```

## What the rule reports

The rule reports `df.to_csv(path)` without `index=`, which writes the index as an extra column without a name, or under its name when the index has one. The rule leaves alone a call with no argument and a call that forwards `**options`.

## How to fix it

`xpd check --fix --unsafe-fixes` adds `index=False` to `to_csv(...)` calls that omit `index=`. It is unsafe because you should pass `index=True` instead when the index holds meaningful data.
