These names and arguments warn on category_encoders and will be removed. Use the replacement that the message names, such as `get_feature_names_out()`.

```diff lang="python"
-names = encoder.get_feature_names()
-hasher = ce.HashingEncoder(cols=["city"], max_sample=1000, n_components=64)
+names = encoder.get_feature_names_out()
+hasher = ce.HashingEncoder(cols=["city"], n_components=64)
```

## What the rule reports

The rule reports `encoder.get_feature_names()` and `HashingEncoder(max_sample=...)`.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The severity is warning for `get_feature_names`, and info for `max_sample`. The fix is safe.

`get_feature_names` raised ``FutureWarning: `get_feature_names` is deprecated in all of
sklearn. Use `get_feature_names_out` instead.`` Both methods returned the same NumPy array, so
the fix is safe. The method `get_feature_names_out` was added in 2.6, so the document carries
`since: "2.6"`.

scikit-learn removed its own `get_feature_names`, and
[`removed-in-sklearn`](../sklearn/removed-in-sklearn.md) reports every call in a file that imports sklearn.
`deprecated-in-category-encoders` therefore runs only in a file that doesn't import sklearn. It
needs the receiver built from a class whose name ends in `Encoder` or `Wrapper`, in the same expression or in an
assignment of the same function.

The changelog of 2.6.4 says of `max_sample` that "it has no use anymore". `HashingEncoder` still
takes the keyword and raises no warning, so the document is info. With `max_sample=10` and
without it the encoded frame was equal, with one process and with two, so the fix that drops the
keyword is safe.
