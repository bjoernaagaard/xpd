def historical_price(frame):
    return frame["price"].rolling(24).mean()


def current_forecast(frame):
    return frame.groupby("zone")["forecast"].transform(lambda values: values.rolling(24).mean())


def grouped_history(frame):
    return frame.groupby("zone")["price"].transform(lambda values: values.shift(1).rolling(24).mean())


def residual_load(load, wind):
    return load - wind


def residual_load_copy(load, wind):
    return load - wind


def total_supply(load, wind):
    return load + wind
