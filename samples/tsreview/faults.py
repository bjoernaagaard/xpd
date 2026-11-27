import pandas as pd
from mlforecast import MLForecast
from sklearn.model_selection import train_test_split
from utilsforecast.losses import mape

observations = pd.DataFrame(
    {"unique_id": ["a", "a", "b"], "ds": [1, 2, 1], "y": [4, 0, 5], "promo": [1, 0, 0]}
).sort_values("ds")

train, holdout = train_test_split(observations, random_state=0)
observations["lag"] = observations["y"].shift(1)
model = MLForecast(models=[], freq=1, lags=[1])
model.fit(train, static_features=[])
forecast = model.predict(h=2)
score = mape(holdout, models=["prediction"])
