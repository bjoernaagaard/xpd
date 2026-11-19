Calling `concat` inside a loop copies every earlier row on each pass. Collect the frames in a list and call `concat` once after the loop.

```diff lang="python"
-out = pd.DataFrame()
-for path in paths:
-    out = pd.concat([out, pd.read_csv(path)])
+frames = [pd.read_csv(path) for path in paths]
+out = pd.concat(frames)
```

## What the rule reports

The rule reports a frame that grows inside a loop, as in `out = pd.concat([out, part])`, because each pass copies every earlier row. The rule leaves a concat with `axis=1` alone, because on pandas 3 it shares the columns it already has.

The rule leaves alone a `pd.concat` inside a loop that builds a frame from other objects, such as `panel = pd.concat(parts_of_this_month)`, because it copies each row once.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The replacement assumes that `paths` is not empty and that the empty starting frame set no columns or dtypes. One `concat` at the end copies each row once.
