"""Supported XGBoost calls and nearby cases the pack leaves alone."""

import xgboost as xgb
from xgboost import DMatrix, QuantileDMatrix, XGBRegressor
from xgboost.callback import EarlyStopping


def estimator_fit(features, labels, validation):
    model = XGBRegressor(eval_metric="rmse", early_stopping_rounds=3, callbacks=[])
    model.fit(features, labels, eval_set=validation)
    return model.predict(features)


def current_tree_method(train):
    xgb.XGBClassifier(tree_method="hist", device="cuda")
    xgb.train({"tree_method": "hist", "device": "cuda"}, train)
    xgb.cv({"tree_method": "hist"}, train)
    xgb.train({"booster": "gblinear", "tree_method": "gpu_hist"}, train)


def quantile_reference(features, labels, valid_features, valid_labels):
    train = QuantileDMatrix(features, label=labels)
    xgb.train({}, train, evals=[(train, "train")])
    valid = QuantileDMatrix(valid_features, label=valid_labels, ref=train)
    xgb.train({}, train, evals=[(train, "train"), (valid, "valid")])
    xgb.train({}, train, evals=[(DMatrix(valid_features, label=valid_labels), "valid")])
    alias = train
    xgb.train({}, train, evals=[(alias, "train")])
    return QuantileDMatrix(features, label=labels, max_quantile_batches=None)


def native_predictions(train, validation, features, replacement):
    model = xgb.train({}, train, evals=[(validation, "valid")], early_stopping_rounds=3)
    model.predict(validation, iteration_range=(0, model.best_iteration + 1))
    model.inplace_predict(features, iteration_range=(0, 0))
    model.inplace_predict(features, (0, model.best_iteration + 1))
    model = replacement
    return model.predict(features)


def fresh_callbacks(train, validation, parameter_grid):
    for params in parameter_grid:
        stop = EarlyStopping(rounds=3)
        xgb.train(params, train, evals=[(validation, "valid")], callbacks=[stop])


def rebound_callback(train, validation, parameter_grid):
    stop = EarlyStopping(rounds=10)
    for params in parameter_grid:
        stop = EarlyStopping(rounds=3)
        xgb.train(params, train, evals=[(validation, "valid")], callbacks=[stop])


def batch_predictions(train, batches):
    model = xgb.train({}, train)
    for batch in batches:
        yield model.inplace_predict(batch)
        yield model.predict(DMatrix(batch, missing=-1))
        yield model.predict(DMatrix(batch), pred_leaf=True)


def other_library(xgb, XGBRegressor, features, labels):
    XGBRegressor(tree_method="gpu_hist").fit(features, labels, eval_metric="rmse")
    xgb.QuantileDMatrix(features, max_quantile_batches=2)
    xgb.train({"tree_method": "gpu_hist"}, features)


def legacy_fit(features, labels):
    try:
        return XGBRegressor().fit(features, labels, eval_metric="rmse")
    except TypeError:
        return XGBRegressor(eval_metric="rmse").fit(features, labels)
