The linearmodels panel models read the first index level as the entity and the second as the time, and here the time comes first, so the effects are on the wrong level. Put the entity first in `set_index`.

```diff lang="python"
-data = wage.set_index(["year", "nr"])
+data = wage.set_index(["nr", "year"])
 model = PanelOLS(data.lwage, data[["hours", "married"]], entity_effects=True)
```

## What the rule reports

The rule reports `set_index([time, entity])` when the frame reaches a panel model. The first name
must be a usual name for a time, such as `"year"` or `"date"`, and the second must not be. linearmodels
doesn't check which level is which.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes the two names in the other order.

With the year first, `PanelOLS` read the wage panel as 8 entities over 545 periods, with no error
and no warning:

| Index | Option | Coefficient of `married` |
| --- | --- | --- |
| `["year", "nr"]` | `entity_effects=True` | 0.1622 |
| `["nr", "year"]` | `time_effects=True` | 0.1622 |
| `["nr", "year"]` | `entity_effects=True` | 0.2483 |

The rule knows the order only by the names. A two level index is also made for other uses, so the
frame must reach a panel model. It reaches one when the `set_index` call is an argument of the
model, or when its result is assigned to a name that a panel model in the same function reads. The first form of the rule read
every `set_index` in a file that imports linearmodels, and 2 of its 4 reports on the public files were
a table of portfolios by month that no panel model reads.

The rule also stays silent when:

- another statement in the function can change the index of the same name, such as `swaplevel`, a
  second `set_index`, a merge or an assignment to `.index`
- the same chain goes on with such a call, as in `set_index(["year", "nr"]).swaplevel()`
- a later statement assigns the name again

A list with a comment inside is reported without the fix, which would delete the comment.
