Fit the portfolio optimizer on the training split, then predict or score on
the holdout. This rule follows a locally known skfolio optimizer that fits the
same bare input later split by `train_test_split` and consumes the holdout
before any refit. A separate deployment fit with no holdout consumer is valid.
