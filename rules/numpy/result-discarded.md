Functions such as `np.sort` and `np.append` return a new array and leave the input alone, so a call on its own line does nothing. Assign the result.

```diff lang="python"
-np.append(lengths, 1)
+lengths = np.append(lengths, 1)
```

## What the rule reports

The rule reports `np.sort(x)`, `np.append(x, 4)`, `np.clip(x, 0, 1)`, `x.astype(float)` and `m.reshape(3, 2)` standing alone as a statement. Each call returns a new array that the code drops, so the statement does nothing.

The rule leaves alone a call with `out=`, a call in a `try` block or in a `with pytest.raises` block, and a call in a benchmark method of asv (airspeed velocity). After the fix, other names bound to the old array keep its old values.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `x = np.sort(x)`.
