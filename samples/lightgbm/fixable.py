"""LightGBM diagnostics that can all be cleared by applying their fixes."""

import lightgbm as lgb
from numpy.typing import NDArray


def removed_keyword(train_set: lgb.Dataset) -> None:
    lgb.train(fobj=None, params={}, train_set=train_set)  # xpd: removed-training-keyword
    lgb.cv({}, train_set, verbose_eval=False, stratified=False)  # xpd: removed-training-keyword


def rounds(train_set: lgb.Dataset) -> None:
    lgb.train(
        # xpd: boost-round-overridden
        {"num_iterations": 50, "n_estimators": 50},
        train_set,
        num_boost_round=200,
    )
    lgb.train({"num_iterations": 70}, train_set, 200)  # xpd: boost-round-overridden
    lgb.cv(
        num_boost_round=90, params={"n_estimators": 40}, train_set=train_set
    )  # xpd: boost-round-overridden
    lgb.train({"num_iterations": 60}, train_set, 200, valid_sets=[])  # xpd: boost-round-overridden


def validation(
    model: lgb.LGBMRegressor, data: NDArray, labels: NDArray, valid: NDArray, valid_labels: NDArray
) -> None:
    model.fit(data, labels, verbose=False)  # xpd: removed-training-keyword
    model.fit(data, labels, eval_set=[(valid, valid_labels)])  # xpd: deprecated-eval-set


def prediction(
    model: lgb.LGBMRegressor, folds: lgb.CVBooster, train_set: lgb.Dataset, data: NDArray
) -> None:
    model.booster_.predict(
        data, predict_disable_shape_check=(True)
    )  # xpd: predict-disabled-shape-check
    folds.predict(data, predict_disable_shape_check="true")  # xpd: predict-disabled-shape-check
    result = lgb.cv({}, train_set, return_cvbooster=True)
    result["cvbooster"].predict(
        data, predict_disable_shape_check=True
    )  # xpd: predict-disabled-shape-check
