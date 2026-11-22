Concatenating into the same frame on every iteration repeatedly grows the result. Collect the pieces and concatenate once.

```diff lang="python"
-output = pl.concat([output, piece], rechunk=True)
+output = pl.concat(pieces, rechunk=True)
```

## What the rule reports

16.23x and 269.11x on 50,000 and 500,000 total rows across 25 batches with rechunk=True.

## How to fix it

Edit the code using the replacement in the example.
