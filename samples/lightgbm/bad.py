"""LightGBM calls that should report diagnostics."""

import lightgbm as lgb
import pytest
from lightgbm import LGBMRegressor, cv
from numpy.typing import NDArray


def old_native(train_set: lgb.Dataset) -> None:
    lgb.train(  # xpd: removed-training-keyword
        {}, train_set, num_boost_round=20, early_stopping_rounds=5
    )
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
        features,
        labels,
        eval_set=[(valid_features, valid_labels)],  # xpd: deprecated-eval-set
    )


def bagging(train_set: lgb.Dataset) -> None:
    LGBMRegressor(subsample=0.8)  # xpd: bagging-without-frequency
    lgb.cv(  # xpd: bagging-without-frequency
        params={"bagging_fraction": 0.7, "bagging_freq": 0}, train_set=train_set
    )


def dart(train_set: lgb.Dataset, valid_set: lgb.Dataset) -> None:
    lgb.train(  # xpd: dart-early-stopping
        {"boosting": "dart"}, train_set, valid_sets=[valid_set], callbacks=[lgb.early_stopping(3)]
    )
    cv({"boosting_type": "dart", "early_stopping_round": 5}, train_set)  # xpd: dart-early-stopping


def rounds(train_set: lgb.Dataset) -> None:
    lgb.train(
        {"objective": "regression", "n_estimators": 50},
        train_set,
        num_boost_round=200,  # xpd: boost-round-overridden
    )
    cv(
        train_set=train_set,
        params={"num_iterations": 80},
        num_boost_round=200,  # xpd: boost-round-overridden
    )


def prediction(train_set: lgb.Dataset, features: NDArray) -> None:
    booster = lgb.train({}, train_set)
    booster.predict(features, predict_disable_shape_check=True)  # xpd: predict-disabled-shape-check
    model = lgb.LGBMClassifier()
    model.predict_proba(
        features,
        predict_disable_shape_check=True,  # xpd: predict-disabled-shape-check
    )


def unrelated_guards(
    train_set: lgb.Dataset, model: lgb.LGBMRegressor, data: NDArray, labels: NDArray
) -> None:
    try:
        lgb.train({}, train_set, verbose_eval=5)  # xpd: removed-training-keyword
    except OSError:
        return
    if hasattr(train_set, "num_data"):
        lgb.train({}, train_set, fobj=None)  # xpd: removed-training-keyword
        model.fit(data, labels, eval_set=[(data, labels)], eval_X=None)  # xpd: deprecated-eval-set
    model.fit(data, labels, verbose=False)  # xpd: removed-training-keyword


def shadowed_version_guard(
    lgb: object, model: LGBMRegressor, train_set: object, data: NDArray, labels: NDArray
) -> None:
    if lgb.__version__.startswith("3."):
        cv({}, train_set, fobj=None)  # xpd: removed-training-keyword
        model.fit(data, labels, eval_set=[(data, labels)])  # xpd: deprecated-eval-set


def newer_version_guard(train_set: lgb.Dataset) -> None:
    if lgb.__version__.startswith("4.6."):
        cv({}, train_set, feature_name=["sector"])  # xpd: removed-training-keyword


def unrelated_type_error(train_set: lgb.Dataset, model: LGBMRegressor, data: NDArray) -> None:
    with pytest.raises(TypeError, match="unrelated"):
        lgb.train({}, train_set, verbose_eval=False)  # xpd: removed-training-keyword
    with pytest.raises(TypeError, match="early_stopping_rounds"):
        model.fit(data, data, verbose=False)  # xpd: removed-training-keyword


def positional_validation(
    data: NDArray, labels: NDArray, valid: NDArray, valid_labels: NDArray
) -> None:
    lgb.LGBMRegressor().fit(  # xpd: deprecated-eval-set
        data, labels, None, None, [(valid, valid_labels)]
    )
    lgb.LGBMRanker().fit(  # xpd: deprecated-eval-set
        data, labels, None, None, [5, 5], [(valid, valid_labels)]
    )


def aliased_bagging(train_set: lgb.Dataset) -> None:
    LGBMRegressor(bagging_fraction=0.8)  # xpd: bagging-without-frequency
    lgb.train({"subsample": 0.8}, train_set)  # xpd: bagging-without-frequency
    LGBMRegressor(subsample=0.8, boosting_type="gbdt")  # xpd: bagging-without-frequency


def dart_with_callbacks(train_set: lgb.Dataset, data: NDArray, labels: NDArray) -> None:
    lgb.train(  # xpd: dart-early-stopping
        {"boosting": "dart", "boosting_type": "gbdt"}, train_set, callbacks=[lgb.early_stopping(3)]
    )
    lgb.LGBMRegressor(boosting_type="dart").fit(  # xpd: dart-early-stopping
        data, labels, callbacks=[lgb.early_stopping(3)]
    )


def later_positional(train_set: lgb.Dataset, valid_set: lgb.Dataset) -> None:
    lgb.train({"num_iterations": 50}, train_set, 200, [valid_set])  # xpd: boost-round-overridden
