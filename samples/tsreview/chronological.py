import pandas as pd
from sklearn.model_selection import train_test_split
from utilsforecast.losses import mae

observations = pd.DataFrame({"unique_id": ["a", "a"], "ds": [1, 2], "y": [4, 0]})
train, holdout = train_test_split(observations.sort_values("ds"), shuffle=False)
observations["lag"] = observations.sort_values(["unique_id", "ds"]).groupby("unique_id")["y"].shift(1)
score = mae(holdout, models=["prediction"])
