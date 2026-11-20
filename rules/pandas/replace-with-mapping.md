`replace` with a mapping takes longer for every key in the mapping. Use `map(lambda v: mapping.get(v, v))`, which keeps the other values and the dtype.

```diff lang="python"
-names = df["name"].replace(mapping)
+names = df["name"].map(lambda value: mapping.get(value, value))
```

## What the rule reports

The rule reports `df["k"].replace(mapping)` with a mapping held in a name, and asks for `map` with `mapping.get(v, v)`. The `map` form keeps the values that the mapping lacks, and it keeps the dtype.

The file must show that the name holds a dict: the function or the module assigns it a dict, a dict comprehension or a call of `dict`, or annotates it as `dict` or `Mapping`. Any other name is left alone, because a `Path` and a `str` have a `replace` too, as in `source.replace(destination)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `s.map(lambda v: mapping.get(v, v))`. A key of NaN, text mapped to numbers, and a dict of dicts on a frame give another result.

The replacement assumes a plain dict of single values. `replace` can be faster with a short mapping, and `map` doesn't replace by regular expression.
