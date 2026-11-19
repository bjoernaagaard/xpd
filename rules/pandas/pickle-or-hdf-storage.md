`read_pickle` runs whatever code the file tells it to, so a pickle from someone else can run any code. Store frames as Parquet, which holds data only and kept every dtype in a smaller file.

```diff lang="python"
-df.to_pickle("clean.pkl")
-again = pd.read_pickle("clean.pkl")
+df.to_parquet("clean.parquet")
+again = pd.read_parquet("clean.parquet")
```

## What the rule reports

The rule reports `to_pickle`, `pd.read_pickle`, `to_hdf`, `pd.read_hdf` and `pd.HDFStore`. `read_pickle` runs the function that the file names. For example, a pickle that named `print` made `pd.read_pickle` print a line. HDF needs the PyTables package, and without it `to_hdf` raised ImportError. Parquet holds data only, and it needs pyarrow.

## How to fix it

xpd has no automatic fix, so make the change by hand.
