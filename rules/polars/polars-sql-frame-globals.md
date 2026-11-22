Polars sql resolves table names from Python globals. Use SQLContext with explicit table bindings when the query should depend on named inputs.

```diff lang="python"
-pl.sql("SELECT * FROM df")
+pl.SQLContext(df=df).execute("SELECT * FROM df")
```

## What the rule reports

Implicit and explicit binding each returned 3 rows.

## How to fix it

Edit the code using the replacement in the example.
