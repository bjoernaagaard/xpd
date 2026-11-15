"""Pipeline, ColumnTransformer, set_output and metadata routing forms on scikit-learn 1.9.1.

A line that xpd reports ends with a comment that names the rule. A form that raises on
scikit-learn 1.9.1 sits in a plain function, which the checker never calls. Each case returns
what the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd
from sklearn.compose import ColumnTransformer, TransformedTargetRegressor, make_column_transformer
from sklearn.linear_model import LogisticRegression, Ridge
from sklearn.model_selection import (
    GridSearchCV,
    GroupKFold,
    KFold,
    StratifiedKFold,
    RandomizedSearchCV,
    cross_val_predict,
    cross_val_score,
    cross_validate,
    learning_curve,
)
from sklearn.pipeline import FeatureUnion, Pipeline, make_pipeline, make_union
from sklearn.preprocessing import OneHotEncoder, StandardScaler

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def grouped(seed=0, n_groups=12, per_group=5):
    r = np.random.default_rng(seed)
    g = np.repeat(np.arange(n_groups), per_group)
    X = r.normal(size=(n_groups, 3))[g] + r.normal(scale=0.1, size=(len(g), 3))
    y = np.tile([0, 1], n_groups // 2)[g]
    return X, y, g


@case
def groups_cross_validation():
    X, y, g = grouped()
    model = LogisticRegression()
    ignored = cross_val_score(model, X, y, groups=g)  # xpd: groups-ignored
    five = cross_val_score(model, X, y, groups=g, cv=5)  # xpd: groups-ignored
    kfold = cross_validate(model, X, y, groups=g, cv=KFold(5, shuffle=True, random_state=0))  # xpd: groups-ignored
    labels = cross_val_predict(model, X, y, groups=g, cv=StratifiedKFold(3))  # xpd: groups-ignored
    grouped_scores = cross_val_score(model, X, y, groups=g, cv=GroupKFold(3))
    splitter = KFold(3)
    held = cross_val_score(model, X, y, groups=g, cv=splitter)
    plain = cross_val_score(model, X, y, cv=5)
    return ignored, five, kfold["test_score"], labels, grouped_scores, held, plain


@case
def groups_search():
    X, y, g = grouped()
    search = GridSearchCV(make_pipeline(StandardScaler(), LogisticRegression()), {"logisticregression__C": [0.1, 1.0]}, cv=3)
    search.fit(X, y, groups=g)  # xpd: groups-ignored
    inline = GridSearchCV(LogisticRegression(), {"C": [1.0]}).fit(X, y, groups=g)  # xpd: groups-ignored
    by_group = GridSearchCV(LogisticRegression(), {"C": [1.0]}, cv=GroupKFold(3))
    by_group.fit(X, y, groups=g)
    plain = GridSearchCV(LogisticRegression(), {"C": [1.0]}, cv=3)
    plain.fit(X, y)
    return search.best_score_, inline.best_score_, by_group.best_score_, plain.best_score_


def group_splitter_without_groups():
    X, y, g = grouped()
    cross_val_score(LogisticRegression(), X, y, cv=GroupKFold(3))  # xpd: groups-ignored
    cross_val_score(LogisticRegression(), X, y, cv=GroupKFold(3), groups=g)


def frame(seed=0, n=60):
    r = np.random.default_rng(seed)
    X = pd.DataFrame(r.normal(size=(n, 3)), columns=["a", "b", "c"])
    X["city"] = r.choice(["oslo", "rome", "paris"], n)
    return X, np.tile([0, 1], n // 2), r.uniform(0.5, 2.0, n)


def routing_is_off():
    X, y, w = frame()
    model = LogisticRegression().set_fit_request(sample_weight=True)  # xpd: metadata-routing
    cross_validate(model, X[["a", "b"]], y, params={"sample_weight": w})
    Pipeline([("scale", StandardScaler()), ("model", LogisticRegression())], transform_input=["X_val"]).fit(X[["a"]], y)  # xpd: metadata-routing


@case
def routing_off_forms_that_run():
    X, y, w = frame()
    plain = Pipeline([("scale", StandardScaler()), ("model", LogisticRegression())], transform_input=None).fit(X[["a"]], y)
    scores = cross_validate(LogisticRegression(), X[["a", "b"]], y, params={"sample_weight": w})["test_score"]
    return plain.predict(X[["a"]]), scores


def removed_fit_params():
    X, y, w = frame()
    cross_val_score(LogisticRegression(), X[["a"]], y, fit_params={"sample_weight": w})  # xpd: fit-params-keyword
    cross_validate(LogisticRegression(), X[["a"]], y, cv=3, fit_params={"sample_weight": w})  # xpd: fit-params-keyword
    learning_curve(LogisticRegression(), X[["a"]], y, fit_params={"sample_weight": w})  # xpd: fit-params-keyword
    GridSearchCV(LogisticRegression(), {"C": [1.0]}, fit_params=None).fit(X[["a"]], y)  # xpd: fit-params-keyword
    RandomizedSearchCV(LogisticRegression(), {"C": [1.0]}, n_iter=1, fit_params={"sample_weight": w}, random_state=0)  # xpd: fit-params-keyword


@case
def params_keyword():
    X, y, w = frame()
    scores = cross_val_score(LogisticRegression(), X[["a"]], y, params={"sample_weight": w})
    search = GridSearchCV(LogisticRegression(), {"C": [1.0]}).fit(X[["a"]], y, sample_weight=w)
    return scores, search.best_estimator_.coef_


def pipeline_plain_keyword():
    X, y, w = frame()
    pipe = make_pipeline(StandardScaler(), LogisticRegression())
    pipe.fit(X[["a"]], y, sample_weight=w)  # xpd: pipeline-fit-params
    Pipeline([("scale", StandardScaler()), ("model", LogisticRegression())]).fit(X[["a"]], y, sample_weight=w)  # xpd: pipeline-fit-params


@case
def pipeline_step_keyword():
    X, y, w = frame()
    pipe = make_pipeline(StandardScaler(), LogisticRegression())
    pipe.fit(X[["a"]], y, logisticregression__sample_weight=w)
    pipe.fit(X=X[["a"]], y=y)
    model = LogisticRegression()
    model.fit(X[["a"]], y, sample_weight=w)
    return pipe.predict(X[["a"]]), model.coef_


def step_forms_that_raise():
    X, y, w = frame()
    Pipeline([("scale__a", StandardScaler()), ("model", LogisticRegression())]).fit(X[["a"]], y)  # xpd: pipeline-step-form
    ColumnTransformer([("num__scale", StandardScaler(), ["a"])]).fit(X)  # xpd: pipeline-step-form
    FeatureUnion(transformer_list=[("a__b", StandardScaler())]).fit(X[["a"]])  # xpd: pipeline-step-form
    make_pipeline([StandardScaler(), LogisticRegression()]).fit(X[["a"]], y)  # xpd: pipeline-step-form
    make_union([StandardScaler()]).fit(X[["a"]])  # xpd: pipeline-step-form
    Pipeline([StandardScaler(), LogisticRegression()]).fit(X[["a"]], y)  # xpd: pipeline-step-form


@case
def step_forms_that_run():
    X, y, w = frame()
    pipe = Pipeline([("scale_a", StandardScaler()), ("model", LogisticRegression())]).fit(X[["a"]], y)
    grid = GridSearchCV(pipe, {"model__C": [0.5, 1.0]}, cv=3).fit(X[["a"]], y)
    union = make_union(StandardScaler(), StandardScaler()).fit_transform(X[["a"]])
    named = make_pipeline(StandardScaler(), LogisticRegression()).fit(X[["a"]], y)
    return pipe.predict(X[["a"]]), grid.best_params_, union, named.predict(X[["a"]])


def encoder_with_pandas_output():
    X, y, w = frame()
    OneHotEncoder(handle_unknown="ignore").set_output(transform="pandas").fit_transform(X[["city"]])  # xpd: pandas-output-sparse
    encoder = OneHotEncoder()
    encoder.set_output(transform="pandas")  # xpd: pandas-output-sparse
    pre = make_column_transformer((OneHotEncoder(), ["city"]), remainder="passthrough")
    pre.set_output(transform="pandas")  # xpd: pandas-output-sparse
    make_pipeline(make_column_transformer((OneHotEncoder(), ["city"])), LogisticRegression()).set_output(transform="pandas").fit(X, y)  # xpd: pandas-output-sparse


@case
def dense_encoder_with_pandas_output():
    X, y, w = frame()
    dense = OneHotEncoder(sparse_output=False).set_output(transform="pandas").fit_transform(X[["city"]])
    encoder = OneHotEncoder(sparse_output=False)
    encoder.set_output(transform="pandas")
    scaler = StandardScaler()
    scaler.set_output(transform="pandas")
    sparse = OneHotEncoder().fit_transform(X[["city"]])
    return dense, encoder.fit_transform(X[["city"]]), scaler.fit_transform(X[["a"]]), sparse


def removed_remainder_keyword():
    X, y, w = frame()
    ColumnTransformer([("scale", StandardScaler(), ["a"])], remainder="passthrough", force_int_remainder_cols=False).fit(X)  # xpd: column-transformer-removed
    make_column_transformer((StandardScaler(), ["a"]), force_int_remainder_cols=True)  # xpd: column-transformer-removed


@case
def remainder_columns():
    X, y, w = frame()
    ct = ColumnTransformer([("scale", StandardScaler(), ["a"])], remainder="passthrough").fit(X[["a", "b"]])
    return ct.transform(X[["a", "b"]]), ct.transformers_[-1][2]


def target_function_pairs():
    X, y, w = frame()
    target = np.exp(X["a"].to_numpy())
    TransformedTargetRegressor(regressor=Ridge(), func=np.log).fit(X[["b"]], target)  # xpd: transformed-target-pair
    TransformedTargetRegressor(regressor=Ridge(), inverse_func=np.exp).fit(X[["b"]], target)  # xpd: transformed-target-pair
    TransformedTargetRegressor(Ridge(), transformer=StandardScaler(), func=np.log, inverse_func=np.exp)  # xpd: transformed-target-pair


@case
def target_function_pair_that_runs():
    X, y, w = frame()
    target = np.exp(X["a"].to_numpy())
    log = TransformedTargetRegressor(regressor=Ridge(), func=np.log, inverse_func=np.exp).fit(X[["b"]], target)
    scaled = TransformedTargetRegressor(regressor=Ridge(), transformer=StandardScaler()).fit(X[["b"]], target)
    return log.predict(X[["b"]]), scaled.predict(X[["b"]])


def sampler_in_scikit_learn_pipeline():
    from imblearn.over_sampling import SMOTE
    from imblearn.under_sampling import RandomUnderSampler

    X, y, w = frame()
    Pipeline([("smote", SMOTE(random_state=0)), ("model", LogisticRegression())]).fit(X[["a"]], y)  # xpd: pipeline-step-form
    make_pipeline(StandardScaler(), RandomUnderSampler(random_state=0), LogisticRegression()).fit(X[["a"]], y)  # xpd: pipeline-step-form


@case
def sampler_outside_the_pipeline():
    from imblearn.over_sampling import SMOTE

    X, y, w = frame()
    X_more, y_more = SMOTE(random_state=0).fit_resample(X[["a", "b"]], y)
    model = make_pipeline(StandardScaler(), LogisticRegression()).fit(X_more, y_more)
    return model.predict(X[["a", "b"]])


@case
def frame_joined_back():
    from sklearn.model_selection import train_test_split

    X, y, w = frame(n=80)
    data = X[X["a"] > -1].assign(y=np.resize(y, len(X))[X["a"] > -1])
    X_train, X_test, y_train, y_test = train_test_split(data[["a", "b"]], data["y"], random_state=0)
    scaler = StandardScaler()
    scaled = pd.DataFrame(scaler.fit_transform(X_train), columns=["a", "b"])  # xpd: frame-drops-index
    joined = pd.concat([scaled, y_train], axis=1)
    kept = pd.DataFrame(scaler.transform(X_test), columns=["a", "b"], index=X_test.index)
    joined_test = pd.concat([kept, y_test], axis=1)
    numbered = pd.DataFrame(scaler.transform(X_test), columns=["a", "b"])
    numbered.index = X_test.index
    joined_numbered = pd.concat([numbered, y_test], axis=1)
    alone = pd.DataFrame(scaler.transform(X_test), columns=["a", "b"])
    rows = pd.concat([alone, alone])
    return joined, joined_test, joined_numbered, rows


@case
def function_transformers():
    from sklearn.preprocessing import FunctionTransformer

    X, y, w = frame()
    by_batch = FunctionTransformer(lambda Z: (Z - Z.mean(axis=0)) / Z.std(axis=0))  # xpd: stateful-function-transformer
    by_range = FunctionTransformer(func=lambda Z: Z / np.max(Z))  # xpd: stateful-function-transformer
    per_row = FunctionTransformer(lambda Z: Z / np.asarray(Z).sum(axis=1, keepdims=True))
    log = FunctionTransformer(np.log1p)
    pipe = make_pipeline(by_batch, LogisticRegression()).fit(X[["a", "b"]], y)
    return (pipe.predict(X[["a", "b"]]), by_range.fit_transform(X[["a"]].abs()), per_row.fit_transform(X[["a", "b"]].abs().to_numpy()),
            log.fit_transform(X[["a"]].abs()))


@case
def sampler_as_the_last_step():
    from imblearn.over_sampling import RandomOverSampler

    X, y, w = frame()
    pipe = make_pipeline(StandardScaler(), RandomOverSampler(random_state=0)).fit(X[["a"]], y)
    return pipe[0].mean_
