from sklearn.model_selection import cross_val_score
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import TargetEncoder


def encoder_before_outer_cv(model, X, y):
    return cross_val_score(model, TargetEncoder().fit_transform(X, y), y)


def encoder_inside_outer_cv(model, X, y):
    return cross_val_score(make_pipeline(TargetEncoder(), model), X, y)
