"""Supported LightGBM calls and nearby patterns that should stay quiet."""

import re

import lightgbm as lgb
import pytest
from lightgbm import LGBMRegressor, cv
from numpy.typing import NDArray
from pandas import DataFrame


def native(train_set: lgb.Dataset, valid_set: lgb.Dataset) -> None:
    history = {}
    callbacks = [lgb.early_stopping(5), lgb.log_evaluation(10), lgb.record_evaluation(history)]
    lgb.train({}, train_set, valid_sets=[valid_set], callbacks=callbacks)
    cv({"early_stopping_rounds": 5}, train_set)
    train_set.set_categorical_feature(["sector"])


def fit(features: NDArray, labels: NDArray, valid_features: NDArray, valid_labels: NDArray) -> None:
    model = LGBMRegressor()
    model.fit(features, labels, eval_X=(valid_features,), eval_y=(valid_labels,))
    model.fit(features, labels, eval_set=None)


def bagging(train_set: lgb.Dataset) -> None:
    LGBMRegressor(subsample=0.8, subsample_freq=1)
    LGBMRegressor(subsample=1.0)
    LGBMRegressor(subsample=0.8, bagging_freq=1)
    lgb.cv({"bagging_fraction": 0.7, "subsample_freq": 1}, train_set)
    lgb.train({"bagging_fraction": 0.7, "bagging_freq": 2}, train_set)


def dart(train_set: lgb.Dataset, stop_factory: object) -> None:
    lgb.cv({"boosting": "dart", "early_stopping_round": 0}, train_set)
    lgb.cv({"boosting": "gbdt"}, train_set, callbacks=[lgb.early_stopping(3)])
    lgb.cv({"boosting": "dart"}, train_set, callbacks=[stop_factory.early_stopping(3)])


def rounds(train_set: lgb.Dataset, settings: dict[str, int]) -> None:
    lgb.train({}, train_set, num_boost_round=50)
    lgb.train({"n_estimators": 50}, train_set, num_boost_round=50)
    cv({"num_iterations": 50, "n_estimators": 50}, train_set, num_boost_round=50)
    lgb.train({"n_estimators": 50, **settings}, train_set, num_boost_round=50)


def prediction(model: lgb.Booster, features: DataFrame) -> None:
    model.predict(features, predict_disable_shape_check=False)
    model.predict(features, validate_features=True)


def other_library(lgb: object, model: object, features: NDArray, labels: NDArray) -> None:
    lgb.train({"n_estimators": 50}, features, early_stopping_rounds=5, num_boost_round=200)
    model.fit(features, labels, eval_set=[(features, labels)])
    model.predict(features, predict_disable_shape_check=True)


def compatibility(train_set: lgb.Dataset, features: NDArray, labels: NDArray) -> None:
    if lgb.__version__.startswith("3."):
        lgb.train({}, train_set, verbose_eval=10)
    else:
        lgb.train({}, train_set, callbacks=[lgb.log_evaluation(10)])
    model = LGBMRegressor()
    if lgb.__version__.startswith("4.6."):
        model.fit(features, labels, eval_set=[(features, labels)])


def rejected_keywords(train_set: lgb.Dataset, model: LGBMRegressor, data: NDArray) -> None:
    with pytest.raises(TypeError):
        lgb.train({}, train_set, evals_result={})
    with pytest.raises(TypeError, match="unexpected keyword argument"):
        cv({}, train_set, categorical_feature=["sector"])
    with pytest.raises(TypeError, match=r"unexpected keyword argument.*'fobj'"):
        cv({}, train_set, fobj=None)
    with pytest.raises(TypeError, match=re.compile("verbose_eval")):
        cv({}, train_set, verbose_eval=5)
    with pytest.raises(TypeError, match="verbose"):
        model.fit(data, data, verbose=False)


def manual_lookalikes(train_set: lgb.Dataset, data: NDArray, labels: NDArray) -> None:
    LGBMRegressor(subsample=0.8, boosting_type="goss")
    LGBMRegressor(subsample=0.8, bagging_fraction=1.0)
    cv({"subsample": 0.8, "bagging_freq": 1}, train_set)
    cv({"subsample": 0.8, "data_sample_strategy": "goss"}, train_set)
    cv({"boosting": "dart", "boosting": "gbdt"}, train_set, callbacks=[lgb.early_stopping(3)])
    lgb.LGBMRegressor(boosting_type="gbdt").fit(data, labels, callbacks=[lgb.early_stopping(3)])
    lgb.LGBMRegressor().fit(data, labels, None, None, None)
    lgb.LGBMRanker().fit(data, labels, None, None, [5, 5])
    lgb.train({"num_iterations": 50, "n_estimators": 200}, train_set, 50)
