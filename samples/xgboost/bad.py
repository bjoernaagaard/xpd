"""XGBoost calls that xpd should report."""

import xgboost as xgb
from xgboost import DMatrix, QuantileDMatrix, XGBRegressor, cv, train
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
