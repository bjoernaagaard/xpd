"""Forms that repeat work scikit-learn already did, and the forms that do it once.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import time

import numpy as np
import pandas as pd
from sklearn.feature_extraction.text import CountVectorizer, TfidfVectorizer
from sklearn.ensemble import ExtraTreesClassifier, GradientBoostingClassifier, RandomForestClassifier
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.manifold import TSNE
from sklearn.metrics import accuracy_score, f1_score
from sklearn.model_selection import cross_val_score, cross_validate
from sklearn.neighbors import KNeighborsRegressor
from sklearn.preprocessing import StandardScaler
from sklearn.svm import SVC
from sklearn.tree import DecisionTreeClassifier

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


@case
def score_after_predict():
    X, y = table()
    forest = RandomForestClassifier(n_estimators=20, random_state=0).fit(X[:150], y[:150])
    X_test, y_test = X[150:], y[150:]
    y_pred = forest.predict(X_test)
    accuracy = forest.score(X_test, y_test)  # xpd: score-after-predict
    f1 = f1_score(y_test, forest.predict(X_test))  # xpd: score-after-predict
    knn = KNeighborsRegressor()
    fitted = knn.fit(X[:150], X[:150, 0]).predict(X_test)
    r2 = knn.score(X_test, X_test[:, 0])  # xpd: score-after-predict
    return forest, y_pred, accuracy, f1, fitted, r2


@case
def score_without_a_second_predict():
    X, y = table()
    X_test, y_test = X[150:], y[150:]
    forest = RandomForestClassifier(n_estimators=20, random_state=0).fit(X[:150], y[:150])
    y_pred = forest.predict(X_test)
    accuracy = accuracy_score(y_test, y_pred)
    train_accuracy = forest.score(X[:150], y[:150])
    linear = LogisticRegression().fit(X[:150], y[:150])
    linear_pred = linear.predict(X_test)
    linear_accuracy = linear.score(X_test, y_test)
    tree = DecisionTreeClassifier(random_state=0).fit(X[:150], y[:150])
    tree_pred = tree.predict(X_test)
    tree_accuracy = tree.score(X_test, y_test)
    return accuracy, train_accuracy, linear_accuracy, linear_pred, tree_accuracy, tree_pred


@case
def predict_and_predict_proba():
    X, y = table()
    forest = RandomForestClassifier(n_estimators=20, random_state=0).fit(X[:150], y[:150])
    X_test = X[150:]
    labels = forest.predict(X_test)  # xpd: predict-and-predict-proba
    scores = forest.predict_proba(X_test)[:, 1]
    trees = ExtraTreesClassifier(n_estimators=20, random_state=0)
    trees.fit(X[:150], y[:150])
    proba = trees.predict_proba(X_test)
    extra_labels = trees.predict(X_test)  # xpd: predict-and-predict-proba
    return forest, labels, scores, proba, extra_labels


@case
def labels_from_one_predict_proba():
    X, y = table()
    X_test = X[150:]
    forest = RandomForestClassifier(n_estimators=20, random_state=0).fit(X[:150], y[:150])
    proba = forest.predict_proba(X_test)
    labels = forest.classes_[proba.argmax(axis=1)]
    train_labels = forest.predict(X[:150])
    linear = LogisticRegression().fit(X[:150], y[:150])
    linear_labels = linear.predict(X_test)
    linear_scores = linear.predict_proba(X_test)[:, 1]
    return proba, labels, train_labels, linear_labels, linear_scores


@case
def models_with_a_faster_relative():
    X, y = table()
    embedding = TSNE(method="exact", perplexity=10, random_state=0).fit_transform(X[:60])  # xpd: tsne-exact
    return embedding


@case
def the_faster_relatives():
    X, y = table()
    calibrated = SVC(kernel="linear", probability=True, random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    embedding = TSNE(perplexity=10, random_state=0).fit_transform(X[:60])
    return calibrated, embedding


@case
def refit_per_n_estimators():
    X, y = table()
    scores = []
    for n in [5, 10, 20]:
        forest = RandomForestClassifier(n_estimators=n, random_state=0)  # xpd: refit-per-n-estimators
        forest.fit(X[:150], y[:150])
        scores.append(forest.score(X[150:], y[150:]))
    boosted = []
    for n in [5, 10, 20]:
        model = GradientBoostingClassifier(n_estimators=n, random_state=0).fit(X, y)  # xpd: refit-per-n-estimators
        boosted.append(model.predict_proba(X[:10]))
    timed = []
    for n in [5, 10, 20]:
        start = time.perf_counter()
        forest = RandomForestClassifier(n_estimators=n, random_state=0).fit(X[:150], y[:150])
        print(n, time.perf_counter() - start)
        timed.append(forest.score(X[150:], y[150:]))
    return scores, boosted, timed


@case
def grow_one_forest():
    X, y = table()
    forest = RandomForestClassifier(warm_start=True, random_state=0)
    scores = []
    for n in [5, 10, 20]:
        forest.set_params(n_estimators=n)
        forest.fit(X[:150], y[:150])
        scores.append(forest.score(X[150:], y[150:]))
    cross_validated = []
    for n in [5, 10]:
        candidate = RandomForestClassifier(n_estimators=n, random_state=0)
        cross_validated.append(cross_val_score(candidate, X, y, cv=3))
    deeper = []
    for depth in [2, 4]:
        tree_model = RandomForestClassifier(n_estimators=10, max_depth=depth, random_state=0).fit(X, y)
        deeper.append(tree_model.predict_proba(X[:10]))
    return scores, cross_validated, deeper
