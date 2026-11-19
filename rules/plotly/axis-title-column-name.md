Without `labels`, plotly puts the column name, such as `gdpPercap`, on the axis as its title. Pass `labels` with words the reader understands.

```diff lang="python"
-fig = px.scatter(df, x="gdpPercap", y="lifeExp")
+fig = px.scatter(df, x="gdpPercap", y="lifeExp", labels={"gdpPercap": "GDP per person", "lifeExp": "Life expectancy"})
```

## What the rule reports

The rule reports a px call whose `x` or `y` is a written column name that is code and not words, such as `"gdpPercap"` or `"total_bill"`, in a call without `labels`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The book lists as friendly "words are spelled out, mysterious and elaborate encoding avoided",
and as unfriendly "abbreviations abound, requiring the viewer to sort through text to decode
abbreviations" (page 183). px titles an axis with the column that `x` or `y` names.

| Call | Axis titles |
| --- | --- |
| `px.scatter(x="gdpPercap", y="lifeExp")` | `gdpPercap`, `lifeExp` |
| `px.histogram(x="total_bill")` | `total_bill` |
| with `labels=`, a later `update_xaxes(title_text=...)` or a later `update_layout(xaxis_title=...)` | the words given there |

The rule reads a name with an underscore, or with a capital inside a word, as in `lifeExp`. It
leaves alone a plain word such as `"day"`, a name given by a variable, and a list of columns,
which px titles `value`. The rule is silent in a file that sets an axis title or an axis
dictionary anywhere, such as `update_xaxes(...)`, `update_layout(xaxis_title=...)` or
`xaxis=dict(...)`, because the title may be replaced on the figure afterwards. It has no fix,
because only the author knows the words.
