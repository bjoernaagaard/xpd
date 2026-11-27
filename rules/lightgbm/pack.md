# LightGBM

The LightGBM pack checks native training, estimator validation and prediction.
Its API checks follow LightGBM 4.7.0. xpd uses the installed version when it
can find one.

## Training and validation

- removed-training-keyword
- deprecated-eval-set
- bagging-without-frequency
- dart-early-stopping
- early-stop-on-test
- boost-round-overridden

## Prediction

- predict-disabled-shape-check

The rules follow LightGBM imports and local bindings. The configuration checks
look for literal settings. Module aliases work, as do direct imports that
keep the API's name.

[Python API reference](https://lightgbm.readthedocs.io/en/v4.7.0/Python-API.html)

## Runtime evidence

The examples used LightGBM 4.7.0 and generated data with 240 rows and two
features.

| Rule | Observed behavior |
| --- | --- |
| removed-training-keyword | All obsolete keywords covered by the rule raised TypeError on the corresponding training or fit method. |
| deprecated-eval-set | All four estimator classes warned for the old form, produced equal evaluation results with the replacement, and rejected a mix of the two forms. |
| bagging-without-frequency | Native and estimator predictions were identical with the default and with a row fraction of 0.5 at frequency zero. At frequency one the predictions differed. |
| dart-early-stopping | Training ran all eight rounds and left best_iteration at zero with both a callback and a positive stopping parameter. |
| boost-round-overridden | Both train and cv used four rounds from params with eight passed as the argument. |
| predict-disabled-shape-check | Booster prediction and classifier prediction on a pandas frame rejected an extra column by default and accepted it with the flag. |
