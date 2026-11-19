The encoder does not take this `handle_unknown` or `handle_missing` value, so fit raises ValueError on category_encoders 2.10 and later. Use one of the values the message names, such as `"value"`.

```diff lang="python"
-encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="ignore")
+encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="value")
```

## What the rule reports

The rule reports a `handle_unknown` or `handle_missing` string that the encoder doesn't take, such
as `handle_unknown="ignore"`, which belongs to the scikit-learn `OneHotEncoder`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `"value"`, the default, for each string the class doesn't take, and on `HashingEncoder` the keyword is dropped. Since 2.10 the call raised at fit, and before 2.10 the string did not act as the default, so the codes can change.

| Encoders | `handle_unknown` | `handle_missing` |
| --- | --- | --- |
| The ten supervised encoders, and `OrdinalEncoder` | `error`, `return_nan`, `value` | `error`, `return_nan`, `value` |
| `BackwardDifferenceEncoder`, `BaseNEncoder`, `BinaryEncoder`, `GrayEncoder`, `HelmertEncoder`, `PolynomialEncoder`, `SumEncoder` | the three, and `indicator` | the three, and `indicator` |
| `OneHotEncoder`, `MultiHotEncoder` | the three, and `indicator` | the three, `indicator` and `ignore` |
| `RankHotEncoder` | the three | the three, and `ignore` |
| `HashingEncoder` | not a parameter | not a parameter |
| `CountEncoder` | not checked | not checked |

The table was read from 2.11.2 by fitting every class with every string. Since 2.10 an unknown
string raises at fit, as in `ValueError: Unexpected handle_unknown value 'ignore' for
OneHotEncoder. Supported values: ['error', 'indicator', 'return_nan', 'value']`. Before 2.10 the
string was taken in silence and did not act as the default. On 2.9.0, `handle_unknown="ignore"`
coded an unseen category as NaN on `OneHotEncoder`, `OrdinalEncoder`, `BinaryEncoder`,
`TargetEncoder` and `WOEEncoder`. The form is wrong on both sides of 2.10, so the rule runs on
every version.

`"value"` on a category_encoders `OneHotEncoder` codes an unseen category as zeros, which is
what `"ignore"` does in scikit-learn. `HashingEncoder` keeps no list of categories, so it has
neither keyword and raised `TypeError: HashingEncoder.__init__() got an unexpected keyword
argument 'handle_unknown'`. A value that is not a string, such as a number or a function, is
allowed since 2.11 and is left alone. `CountEncoder` checks nothing and is left alone too.
