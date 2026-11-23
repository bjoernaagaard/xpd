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


def raw_overridden_option(data):
    return xgb.train({r"tree_method": r"gpu_hist", "tree_method": "hist"}, data)


def concatenated_overridden_option(data):
    return xgb.train({"tree_method": "gpu_hist", "tree_" "method": "hist"}, data)


def parentheses_around_default(features):
    return QuantileDMatrix(features, max_quantile_batches=(None))


def checked_gpu_error(data):
    try:
        return xgb.train({"tree_method": "gpu_hist"}, data)
    except xgb.core.XGBoostError:
        return xgb.train({"tree_method": "hist", "device": "cuda"}, data)


def checked_quantile_error(data, features):
    try:
        return xgb.train({}, data, evals=[(QuantileDMatrix(features), "valid")])
    except (ValueError, TypeError):
        return xgb.train({}, data, evals=[(QuantileDMatrix(features, ref=data), "valid")])


def equivalent_matrix_reference(features, labels):
    train = DMatrix(features, labels)
    alias = train
    return xgb.train({}, train, evals=[(QuantileDMatrix(features, ref=alias), "valid")])


def linear_predictions(train, batches):
    model = xgb.train(params={"booster": "gblinear"}, dtrain=train)
    for batch in batches:
        yield model.predict(DMatrix(batch))


def linear_predictions_with_params(data, batches):
    params = {"booster": "gblinear"}
    model = xgb.train(params, data)
    for batch in batches:
        yield model.predict(DMatrix(batch))


def last_batch_prediction(train, batches):
    model = xgb.train({}, train)
    for batch in batches:
        if len(batch) == 0:
            break
    else:
        return model.predict(DMatrix(batch))


def loop_binds_callback(train, evals, stops):
    stop = EarlyStopping(rounds=3)
    for stop in stops:
        xgb.train({}, train, evals=evals, callbacks=[stop])


def loop_else_trains_once(train, evals, grid):
    stop = EarlyStopping(rounds=3)
    for params in grid:
        if params:
            break
    else:
        xgb.train({}, train, evals=evals, callbacks=[stop])


def legacy_positional_range(train, valid):
    model = xgb.train({}, train, evals=[(valid, "valid")], early_stopping_rounds=3)
    return model.predict(valid, True, False, False, False, False, True, False, (0, 1))


def old_batch_limit(features):
    if xgb.__version__ < "3.3":
        return QuantileDMatrix(features, max_quantile_batches=2)


def rebound_to_same_matrix(features, labels):
    train = DMatrix(features, labels)
    alias = DMatrix(features, labels)
    alias = train
    return xgb.train({}, train, evals=[(QuantileDMatrix(features, ref=alias), "valid")])
