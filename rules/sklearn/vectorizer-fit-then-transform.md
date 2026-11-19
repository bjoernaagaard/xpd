`fit` already builds the matrix of the documents and drops it, so `transform` reads them a second time. Call `fit_transform`.

```diff lang="python"
 vectorizer = TfidfVectorizer()
-X = vectorizer.fit(docs).transform(docs)
+X = vectorizer.fit_transform(docs)
```

## What the rule reports

The rule reports `vectorizer.fit(docs).transform(docs)`, and `vectorizer.fit(docs)` followed by
`vectorizer.transform(docs)`, on a `CountVectorizer` or `TfidfVectorizer`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `vectorizer.fit_transform(docs)`. The form in two statements has no fix.

`fit` of the two vectorizers runs `fit_transform` and drops the matrix, so `fit_transform` was
faster:

| Vectorizer | At 2,000 documents | At 20,000 documents |
| --- | --- | --- |
| `CountVectorizer` | 1.63 times | 2.06 times |
| `TfidfVectorizer` | 1.82 times | 2.03 times |

The fix is unsafe, because `fit_transform` leaves the column indices of each row unsorted. The
fitted vectorizer is bit equal, but the `TfidfVectorizer` values differ by up to 1.1e-16. A
`MultinomialNB` fitted on the matrix gave probabilities that differ by up to 3e-15.

Reader B called 4 of the 12 held-back reports wrong advice, and reader A called none wrong. Recall
was 19 of 25. The 6 misses were a fitted vectorizer assigned back to its own name, as in
`cv = cv.fit(corpus)` followed by `cv.transform(corpus)`.
