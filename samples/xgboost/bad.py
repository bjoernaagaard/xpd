"""XGBoost calls that xpd should report."""

import xgboost as xgb
from xgboost import DMatrix, ExtMemQuantileDMatrix, QuantileDMatrix, XGBRegressor, cv, train
from xgboost.callback import EarlyStopping


def removed_fit_options(X, y, valid):
    model = XGBRegressor()
    model.fit(X, y, eval_metric="rmse")  # xpd: removed-fit-keywords
    xgb.XGBClassifier().fit(X, y, callbacks=[])  # xpd: removed-fit-keywords
    model.fit(X, y, eval_set=valid, early_stopping_rounds=3)  # xpd: removed-fit-keywords


def removed_tree_method(train):
    xgb.XGBClassifier(tree_method="gpu_hist")  # xpd: removed-gpu-hist
    xgb.train({"tree_method": "gpu_hist"}, train)  # xpd: removed-gpu-hist
    cv(dtrain=train, params={"tree_method": "gpu_hist"})  # xpd: removed-gpu-hist


def quantile_reference(train, X, y):
    valid = QuantileDMatrix(X, label=y, ref=None)
    xgb.train(
        {},
        train,
        evals=[
            (train, "train"),
            (valid, "valid"),  # xpd: quantile-eval-without-train-ref
        ],
    )
    xgb.train(
        params={},
        dtrain=train,
        evals=(
            (QuantileDMatrix(X, label=y), "valid"),  # xpd: quantile-eval-without-train-ref
        ),
    )


def quantile_batch_count(X):
    QuantileDMatrix(X, max_quantile_batches=2)  # xpd: ineffective-max-quantile-batches


def native_predictions(train, valid, X):
    model = xgb.train({}, train, evals=[(valid, "valid")], early_stopping_rounds=3)
    model.predict(valid)  # xpd: native-predict-after-early-stopping
    return model.inplace_predict(X)  # xpd: native-predict-after-early-stopping


def callback_reuse(data, valid, parameter_grid):
    evals = [(valid, "valid")]
    stop = EarlyStopping(rounds=3)
    for params in parameter_grid:
        train(params, data, evals=evals, callbacks=[stop])  # xpd: reused-early-stopping-callback


def batch_predictions(train, batches):
    model = xgb.train({}, train)
    for batch in batches:
        yield model.predict(DMatrix(batch))  # xpd: dmatrix-rebuilt-for-predict


def unrelated_fit_guard(X, y):
    try:
        XGBRegressor().fit(X, y, eval_metric="rmse")  # xpd: removed-fit-keywords
    except OSError:
        return None


def effective_tree_method(data):
    xgb.train({r"tree_method": r"gpu_hist"}, data)  # xpd: removed-gpu-hist
    try:
        cv({"tree_method": "hist", r"tree_method": r"gpu_hist"}, data)  # xpd: removed-gpu-hist
    except OSError:
        return None


def inline_training_matrix(X, y):
    valid = QuantileDMatrix(X)
    xgb.train({}, DMatrix(X, y), evals=[(valid, "valid")])  # xpd: quantile-eval-without-train-ref


def wrong_reference(X, y):
    data = DMatrix(X, y)
    other = DMatrix(X, y)
    xgb.train(
        {},
        data,
        evals=[
            (QuantileDMatrix(X, ref=other), "valid"),  # xpd: quantile-eval-without-train-ref
        ],
    )


def wrong_adjacent_reference(X, y):
    data = DMatrix(X, y)
    other = DMatrix(X, y)
    valid = QuantileDMatrix(X, ref=other)
    return xgb.train({}, data, evals=[(valid, "valid")])  # xpd: quantile-eval-without-train-ref


def external_memory_watchlist(data, batches):
    try:
        xgb.train(
            {},
            data,
            evals=[
                (ExtMemQuantileDMatrix(batches), "v"),  # xpd: quantile-eval-without-train-ref
            ],
        )
    except OSError:
        return None


def unrelated_attribute_guard(X, settings, limit):
    if hasattr(settings, "cache_dir"):
        QuantileDMatrix(X, max_quantile_batches=limit)  # xpd: ineffective-max-quantile-batches


def hex_rounds_and_empty_callbacks(data, valid):
    model = xgb.train(
        {},
        data,
        evals=[(valid, "valid")],
        early_stopping_rounds=0x3,
        callbacks=[],
    )
    return model.predict(valid, True)  # xpd: native-predict-after-early-stopping


def callback_list(data, grid, watch, cb):
    stop = EarlyStopping(rounds=3)
    for cfg in grid:
        train(cfg, data, evals=watch, callbacks=[cb, stop])  # xpd: reused-early-stopping-callback
