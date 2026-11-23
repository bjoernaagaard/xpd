"""LightGBM calls that should report diagnostics."""

import lightgbm as lgb
from lightgbm import LGBMRegressor, cv
from numpy.typing import NDArray


def old_native(train_set: lgb.Dataset) -> None:
    lgb.train(
        {}, train_set, num_boost_round=20, early_stopping_rounds=5
    )  # xpd: removed-training-keyword
    cv({}, train_set, fobj=None)  # xpd: removed-training-keyword
    cv({}, train_set, verbose_eval=10)  # xpd: removed-training-keyword
    lgb.train({}, train_set, evals_result={})  # xpd: removed-training-keyword
    lgb.train({}, train_set, feature_name=["sector"])  # xpd: removed-training-keyword
    cv({}, train_set, categorical_feature=["sector"])  # xpd: removed-training-keyword


def old_fit(
    features: NDArray, labels: NDArray, valid_features: NDArray, valid_labels: NDArray
) -> None:
    model = LGBMRegressor()
    model.fit(features, labels, early_stopping_rounds=5)  # xpd: removed-training-keyword
    model.fit(
        features, labels, eval_set=[(valid_features, valid_labels)]
    )  # xpd: deprecated-eval-set


def bagging(train_set: lgb.Dataset) -> None:
    LGBMRegressor(subsample=0.8)  # xpd: bagging-without-frequency
    lgb.cv(
        params={"bagging_fraction": 0.7, "bagging_freq": 0}, train_set=train_set
    )  # xpd: bagging-without-frequency


def dart(train_set: lgb.Dataset, valid_set: lgb.Dataset) -> None:
    lgb.train(
        {"boosting": "dart"}, train_set, valid_sets=[valid_set], callbacks=[lgb.early_stopping(3)]
    )  # xpd: dart-early-stopping
    cv({"boosting_type": "dart", "early_stopping_round": 5}, train_set)  # xpd: dart-early-stopping


def rounds(train_set: lgb.Dataset) -> None:
    lgb.train(
        {"objective": "regression", "n_estimators": 50}, train_set, num_boost_round=200
    )  # xpd: boost-round-overridden
    cv(
        train_set=train_set, params={"num_iterations": 80}, num_boost_round=200
    )  # xpd: boost-round-overridden


def prediction(train_set: lgb.Dataset, features: NDArray) -> None:
    booster = lgb.train({}, train_set)
    booster.predict(features, predict_disable_shape_check=True)  # xpd: predict-disabled-shape-check
    model = lgb.LGBMClassifier()
    model.predict_proba(
        features, predict_disable_shape_check=True
    )  # xpd: predict-disabled-shape-check
