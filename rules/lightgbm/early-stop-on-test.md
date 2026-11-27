Early stopping chooses the number of boosting rounds using an evaluation set.
Scoring that set again gives a score on data that already guided the fit.
Use an inner development validation set to choose the round and score the
held-out test set separately.
