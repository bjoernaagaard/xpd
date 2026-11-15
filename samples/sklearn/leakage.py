"""Steps fitted on rows they should not see, and the same steps fitted on the training rows.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd
from imblearn.over_sampling import SMOTE
from imblearn.pipeline import make_pipeline as make_imb_pipeline
from sklearn.feature_selection import SelectKBest, f_classif
from sklearn.linear_model import LogisticRegression, SGDClassifier
from sklearn.model_selection import GridSearchCV, cross_val_score, cross_validate, train_test_split
from sklearn.neighbors import KNeighborsClassifier
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import LabelEncoder, OneHotEncoder, OrdinalEncoder, StandardScaler, TargetEncoder
from sklearn.utils import resample

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=300, seed=0):
    r = np.random.default_rng(seed)
    city = r.choice(["berlin", "paris", "rome", "oslo"], n, p=[0.4, 0.3, 0.25, 0.05])
    x1 = r.normal(size=n)
    x2 = r.normal(size=n)
    y = ((city == "berlin") * 1.5 + x1 + r.normal(scale=0.5, size=n) > 0.8).astype(int)
    return pd.DataFrame({"city": city, "x1": x1, "x2": x2, "y": y})


def split(df):
    X = df[["x1", "x2"]]
    return train_test_split(X, df["y"], test_size=0.3, random_state=0)


@case
def fit_transform_on_test():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.fit_transform(X_test)  # xpd: fit-on-test
    X_val = X_test
    X_val_scaled = StandardScaler().fit_transform(X_val)  # xpd: fit-on-test
    test = df.tail(30)
    codes = OrdinalEncoder().fit_transform(test[["city"]])  # xpd: fit-on-test
    picked = SelectKBest(f_classif, k=1).fit_transform(X_test, y_test)  # xpd: fit-on-test
    return X_train_scaled, X_test_scaled, X_val_scaled, codes, picked


@case
def refits_that_are_meant():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    cont_val = ["x1", "x2"]
    scaled = StandardScaler().fit_transform(df[cont_val])
    from sklearn.preprocessing import PolynomialFeatures

    poly = PolynomialFeatures(2)
    squares = poly.fit_transform(X_test)
    every_city = LabelEncoder().fit(pd.concat([X_train.index.to_series(), X_test.index.to_series()]))
    return scaled, squares, every_city.classes_


@case
def fit_on_test():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    model = LogisticRegression().fit(X_test, y_test)  # xpd: fit-on-test
    scaler = StandardScaler()
    scaler.fit(X_test)  # xpd: fit-on-test
    return model.score(X_test, y_test), scaler.mean_


@case
def partial_fit_on_test():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    model = SGDClassifier(random_state=0)
    model.partial_fit(X_train, y_train, classes=[0, 1])
    model.partial_fit(X_test, y_test)  # xpd: fit-on-test
    return model.score(X_test, y_test)


@case
def fit_on_train():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    X_val = X_test
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)
    model = LogisticRegression().fit(X_train_scaled, y_train)
    calibrated = LogisticRegression().fit(X_val, y_test)
    latest = scaler.fit_transform(X_train)
    X_cv_and_test = pd.concat([X_train, X_test])
    final = LogisticRegression().fit(X_cv_and_test, pd.concat([y_train, y_test]))
    return model.score(X_test_scaled, y_test), calibrated.coef_, latest, final.coef_


@case
def fit_before_split():
    df = frame()
    X = df[["x1", "x2"]]
    y = df["y"]
    X_picked = SelectKBest(f_classif, k=1).fit_transform(X, y)  # xpd: fit-before-split
    X_train, X_test, y_train, y_test = train_test_split(X_picked, y, test_size=0.3, random_state=0)
    return LogisticRegression().fit(X_train, y_train).score(X_test, y_test)


@case
def scale_before_split():
    df = frame()
    X = df[["x1", "x2"]]
    X_scaled = StandardScaler().fit_transform(X)
    X_both = StandardScaler().fit_transform(X, df["y"])
    X_train, X_test, y_train, y_test = train_test_split(X_both, df["y"], test_size=0.3, random_state=0)
    return LogisticRegression().fit(X_train, y_train).score(X_test, y_test), X_scaled


@case
def fit_columns_before_split():
    df = frame()
    numeric = ["x1", "x2"]
    df[numeric] = SelectKBest(f_classif, k=2).fit_transform(df[numeric], df["y"])  # xpd: fit-before-split
    train, test = train_test_split(df, random_state=0)
    return train.shape, test.shape


@case
def encode_before_split():
    df = frame()
    encoder = TargetEncoder()
    codes = encoder.fit(df[["city"]], df["y"]).transform(df[["city"]])  # xpd: fit-before-split, target-encoder-fit-transform
    cross_fitted = encoder.fit_transform(df[["city"]], df["y"])
    return train_test_split(codes, df["y"], random_state=0), cross_fitted


@case
def target_mean_before_split():
    df = frame()
    df["city_mean"] = df.groupby("city")["y"].transform("mean")  # xpd: fit-before-split
    df["city_x1"] = df.groupby("city")["x1"].transform("mean")
    y = df["y"]
    return train_test_split(df[["city_mean", "city_x1"]], y, random_state=0)


@case
def correlation_before_split():
    df = frame()
    numeric = ["x1", "x2", "y"]
    num = df[numeric]
    print(num.corr()["y"].sort_values())
    picked = num.corr()["y"].drop("y").abs().nlargest(1).index  # xpd: fit-before-split
    y = num["y"]
    X = num[picked]
    return train_test_split(X, y, random_state=0)


@case
def resample_before_split():
    df = frame()
    X_more, y_more = SMOTE(random_state=0).fit_resample(df[["x1", "x2"]], df["y"])  # xpd: fit-before-split
    return train_test_split(X_more, y_more, random_state=0)


@case
def upsample_before_split():
    df = frame()
    minority = df[df["y"] == 1]
    majority = df[df["y"] == 0]
    more = resample(minority, replace=True, n_samples=len(majority), random_state=0)  # xpd: fit-before-split
    fewer = resample(majority, replace=False, n_samples=len(minority), random_state=0)
    both = pd.concat([majority, more])
    return train_test_split(both, random_state=0), fewer.shape


@case
def split_before_fit():
    df = frame()
    y = LabelEncoder().fit_transform(df["y"])
    X_train, X_test, y_train, y_test = train_test_split(df[["x1", "x2"]], y, test_size=0.3, random_state=0)
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_all = StandardScaler().fit_transform(df[["x1", "x2"]])
    X_more, y_more = SMOTE(random_state=0).fit_resample(X_train, y_train)
    return LogisticRegression().fit(X_more, y_more).score(scaler.transform(X_test), y_test), X_all.shape, X_train_scaled


@case
def cross_val_on_fitted_data():
    df = frame()
    X = df[["x1", "x2"]]
    y = df["y"]
    model = LogisticRegression()
    X_picked = SelectKBest(f_classif, k=1).fit_transform(X, y)  # xpd: cross-val-on-fitted-data
    scores = cross_val_score(model, X_picked, y, cv=3)
    inline = cross_val_score(model, SelectKBest(f_classif, k=1).fit_transform(X, y), y, cv=3)  # xpd: cross-val-on-fitted-data
    scaled = cross_validate(model, StandardScaler().fit_transform(X), y, cv=3)["test_score"]
    return scores, inline, scaled


@case
def cross_val_on_resampled_data():
    df = frame()
    X_more, y_more = SMOTE(random_state=0).fit_resample(df[["x1", "x2"]], df["y"])  # xpd: cross-val-on-fitted-data
    return cross_val_score(KNeighborsClassifier(), X_more, y_more, cv=3)


@case
def search_on_fitted_data():
    df = frame()
    X = df[["x1", "x2"]]
    y = df["y"]
    grid = GridSearchCV(LogisticRegression(), {"C": [0.1, 1.0]}, cv=3)
    X_picked = SelectKBest(f_classif, k=1).fit_transform(X, y)  # xpd: cross-val-on-fitted-data
    grid.fit(X_picked, y)
    search = GridSearchCV(KNeighborsClassifier(), {"n_neighbors": [3, 5]}, cv=3)
    X_more, y_more = SMOTE(random_state=0).fit_resample(X, y)  # xpd: cross-val-on-fitted-data
    search.fit(X_more, y_more)
    return grid.best_score_, search.best_score_


@case
def cross_val_on_a_pipeline():
    df = frame()
    X = df[["x1", "x2"]]
    picked = cross_val_score(make_pipeline(SelectKBest(f_classif, k=1), LogisticRegression()), X, df["y"], cv=3)
    resampled = cross_val_score(make_imb_pipeline(SMOTE(random_state=0), KNeighborsClassifier()), X, df["y"], cv=3)
    return picked, resampled


@case
def get_dummies_on_test():
    df = frame()
    X_train, X_test = train_test_split(df[["city", "x1"]], test_size=0.1, random_state=0)
    train_columns = pd.get_dummies(X_train).columns
    X_test_dummies = pd.get_dummies(X_test)  # xpd: get-dummies-on-test
    test_df = pd.get_dummies(X_test, columns=["city"])  # xpd: get-dummies-on-test
    return list(train_columns), list(X_test_dummies.columns), list(test_df.columns)


@case
def one_hot_fitted_on_train():
    df = frame()
    X_train, X_test = train_test_split(df[["city", "x1"]], test_size=0.1, random_state=0)
    encoder = OneHotEncoder(handle_unknown="ignore").fit(X_train[["city"]])
    aligned = pd.get_dummies(X_test).reindex(columns=pd.get_dummies(X_train).columns, fill_value=0)
    return encoder.transform(X_test[["city"]]).shape, aligned.shape, pd.get_dummies(df).shape


@case
def early_stopping_on_test():
    from xgboost import XGBClassifier

    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    X_fit, X_val, y_fit, y_val = train_test_split(X_train, y_train, test_size=0.25, random_state=0)
    model = XGBClassifier(n_estimators=50, early_stopping_rounds=5, n_jobs=1, random_state=0)
    model.fit(X_train, y_train, eval_set=[(X_test, y_test)], verbose=False)  # xpd: early-stopping-on-test
    held = XGBClassifier(n_estimators=50, early_stopping_rounds=5, n_jobs=1, random_state=0)
    held.fit(X_fit, y_fit, eval_set=[(X_val, y_val)], verbose=False)
    return model.score(X_test, y_test), held.score(X_test, y_test)


@case
def tune_on_test():
    df = frame()
    X_train, X_test, y_train, y_test = split(df)
    scores = []
    for k in range(1, 10):  # xpd: tune-on-test
        model = KNeighborsClassifier(n_neighbors=k)
        model.fit(X_train, y_train)
        scores.append(model.score(X_test, y_test))
    for k in (3, 9):
        model = KNeighborsClassifier(n_neighbors=k).fit(X_train, y_train)
        print(k, model.score(X_test, y_test))
    search = GridSearchCV(KNeighborsClassifier(), {"n_neighbors": list(range(1, 10))}, cv=3).fit(X_train, y_train)
    return max(scores), search.score(X_test, y_test)


@case
def fit_on_all_rows():
    df = frame()
    X = df[["x1", "x2"]]
    y = df["y"]
    X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, random_state=0)
    model = LogisticRegression()
    model.fit(X, y)  # xpd: fit-on-all-rows
    leaked = model.score(X_test, y_test)
    honest = LogisticRegression().fit(X_train, y_train)
    X_train, X_val, y_train, y_val = train_test_split(X_train, y_train, random_state=0)
    honest.fit(X_train, y_train)
    return leaked, honest.score(X_val, y_val)
