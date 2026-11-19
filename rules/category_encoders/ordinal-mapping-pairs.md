An `OrdinalEncoder` mapping written as a list of pairs matches no category on category_encoders 2.11, so every row is coded -1.0. Write the mapping as a dict.

```diff lang="python"
-mapping = [{"col": "size", "mapping": [("small", 1), ("medium", 2), ("large", 3)]}]
+mapping = [{"col": "size", "mapping": {"small": 1, "medium": 2, "large": 3}}]
 encoder = ce.OrdinalEncoder(cols=["size"], mapping=mapping)
```

## What the rule reports

The rule reports an `OrdinalEncoder` mapping whose `"mapping"` entry is a list of
`(category, code)` pairs. The list of pairs was the form before category_encoders 2.0, and old
answers and notebooks still show it.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes the list of pairs written as a dict.

category_encoders 2.11.2 takes the list of pairs without an error and finds no category in it.
With `[("a", 1), ("b", 2), ("c", 3)]` every row of a column that holds a, b and c was coded -1.0.
The fix changes every code of the column, from -1.0 to the code the pair names, so it is unsafe.
The rule reports the `"mapping"` entry of a dict that also has a `"col"` entry.
