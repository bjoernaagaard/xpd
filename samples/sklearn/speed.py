"""Forms that repeat work scikit-learn already did, and the forms that do it once.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from sklearn.feature_extraction.text import CountVectorizer, TfidfVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score, cross_validate
from sklearn.preprocessing import StandardScaler

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def documents(n=60, seed=0):
    r = np.random.default_rng(seed)
    words = np.array(["red", "green", "blue", "cat", "dog", "fish", "runs", "sleeps", "eats", "fast"])
    return [" ".join(r.choice(words, 8)) for _ in range(n)]


def table(n=200, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 4))
    y = (X[:, 0] + X[:, 1] + r.normal(scale=0.5, size=n) > 0).astype(int)
    return X, y


@case
def vectorizer_fit_then_transform():
    docs = documents()
    counts = CountVectorizer(min_df=2).fit(docs).transform(docs)  # xpd: vectorizer-fit-then-transform
    tfidf = TfidfVectorizer()
    weights = tfidf.fit(docs).transform(docs)  # xpd: vectorizer-fit-then-transform
    return counts, weights, tfidf


@case
def vectorizer_fit_then_transform_in_two_statements():
    docs, test_docs = documents(), documents(seed=1)
    vectorizer = CountVectorizer()
    vectorizer.fit(docs)  # xpd: vectorizer-fit-then-transform
    X_train = vectorizer.transform(docs)
    X_test = vectorizer.transform(test_docs)
    return vectorizer, X_train, X_test


@case
def vectorizer_fit_on_other_documents():
    docs, test_docs = documents(), documents(seed=1)
    vectorizer = TfidfVectorizer()
    vectorizer.fit(docs)
    X_test = vectorizer.transform(test_docs)
    refitted = CountVectorizer(ngram_range=(1, 2))
    refitted.fit(documents(seed=2))
    later = refitted.transform(docs)
    X, _ = table()
    scaled = StandardScaler().fit(X).transform(X)
    once = TfidfVectorizer().fit_transform(docs)
    return vectorizer, X_test, refitted, later, scaled, once


@case
def cross_val_score_per_metric():
    X, y = table()
    model = LogisticRegression()
    accuracy = cross_val_score(model, X, y, cv=5, scoring="accuracy").mean()  # xpd: cross-val-score-per-metric
    precision = cross_val_score(model, X, y, cv=5, scoring="precision").mean()
    default = cross_val_score(model, X, y)  # xpd: cross-val-score-per-metric
    recall = cross_val_score(model, X, y, scoring="recall")
    return accuracy, precision, default, recall


@case
def cross_validate_once():
    X, y = table()
    model = LogisticRegression()
    scores = cross_validate(model, X, y, cv=5, scoring=["accuracy", "precision"])
    five = cross_val_score(model, X, y, cv=5, scoring="f1")
    three = cross_val_score(model, X, y, cv=3, scoring="recall")
    again = cross_val_score(model, X, y, cv=5, scoring="f1")
    half = cross_val_score(model, X[:100], y[:100], cv=5, scoring="recall")
    return scores["test_accuracy"], five, three, again, half
