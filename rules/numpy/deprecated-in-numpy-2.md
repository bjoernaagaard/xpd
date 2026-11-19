These names and modules warn on NumPy 2.5 and will stop working in a later release. Use the replacement that the example and the table show.

```diff lang="python"
-from numpy.core.numeric import normalize_axis_tuple
-whole = np.fix(values)
-np.maximum(values, 0.0, out_buffer)
+from numpy.lib.array_utils import normalize_axis_tuple
+whole = np.trunc(values)
+np.maximum(values, 0.0, out=out_buffer)
 grid = np.arange(6.0)
-grid.shape = (2, 3)
+grid = grid.reshape((2, 3))
```

## What the rule reports

The rule reports the forms in the table, and each of them warns on NumPy 2.5.

| Kind | Reported form |
| --- | --- |
| Modules and names | `numpy.core`, `np.typename`, `np.char.array`, `np.char.asarray` and `np.char.chararray` |
| Testing helpers | `np.testing.assert_warns` and `suppress_warnings` |
| Dates and times | a timedelta64 without a unit, and a time zone in a datetime64 string |
| Calls and attributes | `fftn` and its kin with `s=` and no `axes=`, and setting `shape`, `dtype` or `strides` on a name bound from an `np.` call in the same function |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `np.trunc` for `np.fix`, `np.ma.round` for `np.ma.round_`, `out=` for an output passed by position to `np.maximum` and `np.minimum`, and a datetime64 string without its `Z`. The unsafe fix writes `np.rec` and `np.char` for `np.core.records` and `np.core.defchararray`, which export fewer names, `np.<name>` for `np.core.<name>` and for `np.core.numeric.<name>` and the other public submodules, `from numpy import` for a `from numpy.core` import of public names, `randint(low, high + 1)` for `random_integers(low, high)`, with or without a size and with brackets around a `high` such as `n * 2`, `randint(1, high + 1)` for `random_integers(high)`, `axes=range(-len(s), 0)` added where `s=` has no `axes=`, which reads `s` twice, `x = x.reshape(shape)` for `x.shape = shape`, and `x = x.view(dtype)` for `x.dtype = dtype`. The other forms have no fix, and `x.strides = v` has none because `as_strided` does not check the bounds that the assignment checked.
