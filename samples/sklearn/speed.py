"""Forms that repeat work scikit-learn already did, and the forms that do it once.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd
from sklearn.feature_extraction.text import CountVectorizer, TfidfVectorizer
from sklearn.linear_model import LinearRegression, LogisticRegression
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


@case
def predict_per_row():
    X, y = table()
    model = LogisticRegression().fit(X[:150], y[:150])
    X_test = X[150:]
    labels = [model.predict([row])[0] for row in X_test]  # xpd: predict-per-row
    scores = []
    for i in range(len(X_test)):
        scores.append(model.predict_proba(X_test[i].reshape(1, -1))[0, 1])  # xpd: predict-per-row
    frame = pd.DataFrame(X_test)
    margins = frame.apply(lambda row: model.decision_function([row])[0], axis=1)  # xpd: apply-axis-1, predict-per-row
    return model, np.array(labels), np.array(scores), margins.to_numpy()


@case
def predict_once_or_recursively():
    X, y = table()
    model = LogisticRegression().fit(X[:150], y[:150])
    labels = model.predict(X[150:])
    series = np.arange(40, dtype=float)
    window = series[:4].copy()
    forecaster = LinearRegression().fit(np.lib.stride_tricks.sliding_window_view(series[:-1], 4), series[4:])
    forecast = []
    for step in range(5):
        forecast.append(forecaster.predict([window])[0])
        window = np.append(window[1:], forecast[-1])
    rows = X[150:160].copy()
    for i in range(len(rows) - 1):
        rows[i + 1, 0] = model.predict_proba([rows[i]])[0, 1]
    return labels, forecast, rows
