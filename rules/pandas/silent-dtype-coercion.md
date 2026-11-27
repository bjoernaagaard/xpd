Inspect invalid and fractional values before converting a pandas column.

```diff lang="python"
-amount = pd.to_numeric(raw_amount, errors='coerce')
+amount = pd.to_numeric(raw_amount, errors='raise')
```
