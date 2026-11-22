Polars DataFrame.equals returned True on equal values with Int32 and Int64 columns. Use assert_frame_equal to check dtypes and report the differences.

```diff lang="python"
-assert df.equals(other)
+pl.testing.assert_frame_equal(df, other)
```

## What the rule reports

equals returned True. assert_frame_equal raised AssertionError for Int32 against Int64.

## How to fix it

Edit the code using the replacement in the example.
