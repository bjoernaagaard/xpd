`TSNE(method="exact")` computes the force between every pair of rows, and the default method gave the same quality 4.5 to 16 times faster. Drop `method="exact"`.

```diff lang="python"
-embedding = TSNE(method="exact", random_state=0).fit_transform(X)
+embedding = TSNE(random_state=0).fit_transform(X)
```

## What the rule reports

The rule reports `TSNE(method="exact")`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `method="exact"`, which gives another embedding.

xpd applies the replacement with `--fix --unsafe-fixes`.

The rule is info, because the default Barnes-Hut method approximates the forces between distant
points. Its embedding therefore differs from the exact one, so no speed proof applies. Over 20 seeds on
`make_classification` data, with the default number of threads, Barnes-Hut gave a trustworthiness
with 5 neighbours within 0.001 of the exact method:

| Rows | Barnes-Hut was faster, as a median | Slowest seed |
| --- | --- | --- |
| 300 | 4.5 times | 4.1 times |
| 1,000 | 16.0 times | 11.4 times |

Recall was 21 of 30. The rule misses `TSNE` imported under another name and a method held in a
variable.
