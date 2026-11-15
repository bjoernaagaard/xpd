"""Time series, and code that looks similar but stays.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

from datetime import datetime, timedelta

import numpy as np
import pandas as pd
from dateutil import parser

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=2000, seed=0):
    r = np.random.default_rng(seed)
    b = r.normal(size=n).round(4)
    b[::11] = np.nan
    return pd.DataFrame({
        "a": r.integers(0, 100, n),
        "b": b,
        "w": r.random(n) + 0.1,
        "k": r.choice(["g1", "g2", "g3", "g4", "g5"], n),
        "s": r.choice(["Alpha_one", " beta_two", "Gamma_three ", "delta_four"], n),
        "raw": r.choice(["2024-01-05", "2024-02-11", "2023-12-31"], n),
        "d": pd.Timestamp("2022-01-01") + pd.to_timedelta(r.integers(0, 400 * 86400, n), unit="s"),
    })


def spread(window):
    return window.max() - window.min()


@case
def rolling_apply(df):
    return (
        df["b"].rolling(20).apply(lambda w: w.mean()),  # xpd: rolling-apply
        df["b"].expanding().apply(lambda w: w.max()),  # xpd: rolling-apply
    )


@case
def rolling_apply_without_raw(df):
    return df["b"].rolling(20).apply(spread)  # xpd: rolling-apply


@case
def rolling_that_stays(df):
    return df["b"].rolling(20).apply(spread, raw=True), df["b"].rolling(20).apply(lambda w: w.count()), df["b"].rolling(20).mean()


@case
def text_time_key(df):
    return df.groupby(df["d"].dt.strftime("%Y-%m"))["b"].sum()  # xpd: text-time-key


@case
def period_time_key(df):
    return df.groupby(df["d"].dt.to_period("M"))["b"].sum(), df["d"].dt.strftime("%Y-%m")


@case
def object_dates(df):
    return df["d"].dt.date  # xpd: object-dates


@case
def datetime_dates(df):
    return df["d"].dt.normalize(), df["d"].dt.floor("D"), df["d"].dt.day


@case
def apply_on_dates(df):
    return (
        df["d"].apply(lambda t: t + pd.Timedelta(days=7)),  # xpd: apply-on-dates
        df["d"].apply(lambda t: t.strftime("%Y-%m")),  # xpd: apply-on-dates
    )


@case
def apply_month_start(df):
    return df["d"].map(lambda t: t.replace(day=1))  # xpd: apply-on-dates


@case
def dates_on_whole_column(df):
    return df["d"] + pd.Timedelta(days=7), df["d"] + pd.to_timedelta(df["d"].dt.day, unit="D")


@case
def index_year_mask(df):
    ts = df.set_index("d").sort_index()
    return ts[ts.index.year == 2023]  # xpd: index-year-mask


@case
def index_year_label(df):
    ts = df.set_index("d").sort_index()
    return ts.loc["2023"], ts[ts.index.month == 3]


@case
def apply_date_method(df):
    gaps = df["d"] - pd.Timestamp("2022-01-01")
    return (
        df["d"].apply(lambda t: t.floor("h")),  # xpd: apply-date-method
        df["d"].map(lambda t: t.tz_localize("UTC")),  # xpd: apply-date-method
        df["d"].apply(lambda t: t.day_name()),  # xpd: apply-date-method
        gaps.apply(lambda v: v.total_seconds()),  # xpd: apply-date-method
        df["d"].apply(lambda t: t.weekday()),  # xpd: apply-date-method
    )


@case
def date_method_that_stays(df):
    return df["d"].dt.floor("h"), df["d"].apply(lambda t: t.round("h")), df["d"].apply(lambda t: t.isoformat())


@case
def apply_parse_date(df):
    return (
        df["raw"].apply(lambda v: datetime.strptime(v, "%Y-%m-%d")),  # xpd: apply-parse-date
        df["raw"].apply(lambda v: pd.to_datetime(v)),  # xpd: apply-parse-date
        df["raw"].map(lambda v: pd.to_datetime(v, format="%Y-%m-%d")),  # xpd: apply-parse-date
        df["raw"].map(pd.Timestamp),  # xpd: apply-parse-date
        df["raw"].apply(lambda v: pd.Timestamp(v)),  # xpd: apply-parse-date
        df["raw"].apply(parser.parse),  # xpd: apply-parse-date
    )


@case
def parse_date_on_whole_column(df):
    return pd.to_datetime(df["raw"], format="%Y-%m-%d"), df["raw"].apply(lambda v: pd.to_datetime(v, dayfirst=v.startswith("3")))


@case
def timedelta_per_value(df):
    return (
        df["a"].apply(lambda v: pd.Timedelta(days=v)),  # xpd: timedelta-per-value
        df["a"].map(lambda v: timedelta(hours=int(v))),
        df["d"] + df["a"].apply(lambda v: timedelta(hours=v)),  # xpd: timedelta-per-value
        df["a"].apply(lambda v: pd.Timedelta(v, unit="D")),  # xpd: timedelta-per-value
        pd.to_timedelta(df["a"].astype(str) + " days"),  # xpd: timedelta-per-value
    )


@case
def timedelta_on_whole_column(df):
    return pd.to_timedelta(df["a"], unit="D"), df["a"].head(5).apply(lambda v: pd.DateOffset(months=v))


@case
def strftime_date_part(df):
    return (
        df["d"].dt.strftime("%Y").astype(int),  # xpd: strftime-date-part
        df["d"].dt.strftime("%m").astype("int64"),  # xpd: strftime-date-part
        df["d"].dt.strftime("%H").astype(int),  # xpd: strftime-date-part
        df["d"].dt.strftime("%j").astype(int),  # xpd: strftime-date-part
        df["d"].dt.strftime("%A"),  # xpd: strftime-date-part
        df["d"].dt.strftime("%B"),  # xpd: strftime-date-part
    )


@case
def date_part_from_dt(df):
    return df["d"].dt.year, df["d"].dt.day_name(), df["d"].dt.strftime("%Y-%m"), df["d"].dt.strftime("%Y").astype(str)


@case
def apply_date_expression(df):
    return (
        df["d"].apply(lambda t: t.weekday() >= 5),  # xpd: apply-date-expression
        df["d"].apply(lambda t: t.month in (12, 1, 2)),  # xpd: apply-date-expression
        df["d"].map(lambda t: t.year == 2023),  # xpd: apply-date-expression
        df["d"].apply(lambda t: t.hour * 60 + t.minute),  # xpd: apply-date-expression
        df["d"].apply(lambda t: t + pd.Timedelta(days=t.day)),  # xpd: apply-date-expression
    )


@case
def date_expression_on_whole_column(df):
    limit = pd.Timestamp("2022-06-01")
    return df["d"].dt.weekday >= 5, df["d"].dt.hour * 60 + df["d"].dt.minute, df["a"].apply(lambda v: limit.year + v)


@case
def apply_date_method_without_dt_twin(df):
    return (
        df["d"].apply(lambda t: t.date()),  # xpd: apply-date-method
        df["d"].apply(lambda t: t.isocalendar()[1]),  # xpd: apply-date-method
        df["d"].map(lambda t: t.isocalendar().week),  # xpd: apply-date-method
        df["d"].apply(lambda t: t.timestamp()),  # xpd: apply-date-method
        df["d"].map(lambda t: t.toordinal()),  # xpd: apply-date-method
    )


@case
def epoch_on_whole_column(df):
    return (df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s"), df["d"].dt.isocalendar().week, df["d"].dt.date  # xpd: object-dates


@case
def strftime_round_trip(df):
    return (
        pd.to_datetime(df["d"].dt.strftime("%Y-%m-%d")),  # xpd: strftime-round-trip
        pd.to_datetime(df["d"].dt.strftime("%Y-%m-%d %H")),  # xpd: strftime-round-trip
        df[df["d"].dt.strftime("%Y-%m-%d") == "2022-06-01"],  # xpd: strftime-round-trip
    )


@case
def dates_cut_without_text(df):
    return df["d"].dt.normalize(), df["d"].dt.floor("h"), df[df["d"].dt.normalize() == pd.Timestamp("2022-06-01")], pd.to_datetime(df["raw"])


@case
def rolling_numpy_function(df):
    return (
        df["b"].rolling(20).agg(np.max),  # xpd: rolling-apply
        df["b"].rolling(20).apply(np.mean, raw=True),  # xpd: rolling-apply
    )


@case
def resample_lambda(df):
    ts = df.set_index("d").sort_index()
    return (
        ts[["b"]].resample("D").apply(lambda v: v.sum()),  # xpd: groupby-lambda
        ts.resample("D")["b"].agg(np.max),  # xpd: groupby-lambda
    )


@case
def resample_named_method(df):
    ts = df.set_index("d").sort_index()
    return ts[["b"]].resample("D").sum(), ts.resample("D")["b"].max(), ts.resample("D")["b"].first()


@case
def epoch_from_index_values(df):
    index = pd.DatetimeIndex(df["d"])
    return (
        index.asi8 // 10**9,  # xpd: epoch-from-int-cast
        df["d"].values.astype("float64") / 1e9,  # xpd: epoch-from-int-cast
    )


@case
def removed_time_series_forms(df):
    ts = df.set_index("d").sort_index()
    return (
        ts.tshift(1),  # xpd: removed-in-pandas-3
        ts["b"].pad(),  # xpd: removed-in-pandas-3
        ts.first("3D"),  # xpd: removed-in-pandas-3
        ts.resample("D", loffset="1h").sum(),  # xpd: removed-in-pandas-3
        pd.date_range("2024-01-01", periods=3, closed="left"),  # xpd: removed-in-pandas-3
        pd.to_datetime(df["raw"], infer_datetime_format=True),  # xpd: removed-in-pandas-3
        pd.to_datetime(df["raw"], errors="ignore"),  # xpd: removed-in-pandas-3
        df["b"].pct_change(fill_method="pad"),  # xpd: removed-in-pandas-3
        pd.Timedelta("5T"),  # xpd: removed-in-pandas-3
        df["d"].diff().astype("timedelta64[D]"),  # xpd: removed-in-pandas-3
    )


@case
def time_series_forms_that_stay(df):
    ts = df.set_index("d").sort_index()
    return ts.shift(1), ts["b"].ffill(), ts.resample("D", closed="left").sum(), pd.to_datetime(df["raw"], errors="coerce"), np.pad(df["a"].to_numpy(), 1), df["s"].str.pad(12), pd.Timedelta("5min")


@case
def day_frequency_with_time_zone(df):
    ts = df.set_index("d").sort_index()[["b"]].tz_localize("Europe/Copenhagen", ambiguous="NaT", nonexistent="NaT")
    return (
        ts.shift(freq="D"),  # xpd: day-frequency-with-time-zone
        ts.shift(1, "1D"),  # xpd: day-frequency-with-time-zone
        ts.shift(freq="2D"),  # xpd: day-frequency-with-time-zone
        ts["b"].pct_change(freq="D"),  # xpd: day-frequency-with-time-zone
        ts.index + pd.offsets.Day(1),  # xpd: day-frequency-with-time-zone
        ts.index - pd.tseries.offsets.Day(),  # xpd: day-frequency-with-time-zone
        ts.resample("D", offset="6h").sum(),  # xpd: day-frequency-with-time-zone
        ts.resample("1D", origin="start").sum(),  # xpd: day-frequency-with-time-zone
        ts.groupby(pd.Grouper(freq="D", offset="6h")).sum(),  # xpd: day-frequency-with-time-zone
    )


@case
def day_frequency_that_stays(df):
    ts = df.set_index("d").sort_index()[["b"]].tz_localize("UTC")
    return (
        ts.shift(freq="24h"),
        ts.shift(1),
        ts.index + pd.Timedelta(days=1),
        ts.resample("D").sum(),
        ts.resample("24h", offset="6h").sum(),
        ts.resample("D", origin="start_day").sum(),
        ts.resample("h", offset="15min").sum(),
    )


@case
def removed_frequency_in_more_calls(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        ts.pct_change(freq="H"),  # xpd: removed-in-pandas-3
        ts.pct_change(freq="M"),  # xpd: removed-in-pandas-3
        ts.shift(1, "15T"),  # xpd: removed-in-pandas-3
        df["d"].dt.to_period("H"),  # xpd: removed-in-pandas-3
        df["d"].dt.to_period("A-DEC"),  # xpd: removed-in-pandas-3
        pd.tseries.frequencies.to_offset("H"),  # xpd: removed-in-pandas-3
        pd.tseries.frequencies.to_offset("M"),  # xpd: removed-in-pandas-3
        pd.period_range("2024", periods=3, freq="H"),  # xpd: removed-in-pandas-3
        pd.Period("2024", "A"),  # xpd: removed-in-pandas-3
        pd.PeriodIndex(["2024-01-01"], freq="15T"),  # xpd: removed-in-pandas-3
        pd.timedelta_range("0h", periods=3, freq="H"),  # xpd: removed-in-pandas-3
        pd.interval_range(pd.Timestamp("2024"), periods=3, freq="H"),  # xpd: removed-in-pandas-3
        ts.index.snap("H"),  # xpd: removed-in-pandas-3
    )


@case
def frequency_in_more_calls_that_stays(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return ts.pct_change(freq="h"), ts.shift(1, "15min"), df["d"].dt.to_period("M"), pd.period_range("2024", periods=3, freq="Q"), pd.Period("2024", "Y")


@case
def removed_timedelta_units(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        pd.Timedelta(1, unit="T"),  # xpd: removed-in-pandas-3
        pd.Timedelta(1, "L"),  # xpd: removed-in-pandas-3
        pd.to_timedelta(df["a"], unit="N"),  # xpd: removed-in-pandas-3
        pd.Timedelta("5 L"),  # xpd: removed-in-pandas-3
        pd.to_timedelta("15T"),  # xpd: removed-in-pandas-3
        ts.resample("D", offset="30T").sum(),  # xpd: removed-in-pandas-3, day-frequency-with-time-zone
    )


@case
def deprecated_timedelta_units(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        pd.Timedelta("1H"),  # xpd: deprecated-in-pandas-3
        pd.to_timedelta("30S"),  # xpd: deprecated-in-pandas-3
        pd.to_timedelta(df["a"], unit="H"),  # xpd: deprecated-in-pandas-3
        pd.Timedelta(1, "S"),  # xpd: deprecated-in-pandas-3
        ts.resample("h", offset="30S").sum(),  # xpd: deprecated-in-pandas-3
        pd.Timestamp.utcfromtimestamp(0),  # xpd: deprecated-in-pandas-3
    )


@case
def timedelta_units_that_stay(df):
    return pd.Timedelta(1, unit="min"), pd.Timedelta("1h"), pd.Timedelta(hours=1), pd.to_timedelta(df["a"], unit="s"), pd.Timestamp.fromtimestamp(0, "UTC")


@case
def removed_time_arguments(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        ts.between_time("08:00", "16:00", include_start=False),  # xpd: removed-in-pandas-3
        ts.between_time("08:00", "16:00", include_end=False),  # xpd: removed-in-pandas-3
        pd.Timestamp("2024-01-01", freq="h"),  # xpd: removed-in-pandas-3
    )


@case
def old_alias_comparison(df):
    index = pd.date_range("2024-01-01", periods=8, freq="h")
    return (
        pd.infer_freq(index) == "H",  # xpd: removed-in-pandas-3
        pd.infer_freq(index) != "M",  # xpd: removed-in-pandas-3
        index.freqstr == "15T",  # xpd: removed-in-pandas-3
    )


@case
def alias_comparison_that_stays(df):
    index = pd.date_range("2024-01-01", periods=8, freq="h")
    return pd.infer_freq(index) == "h", index.freqstr == "15min", index.to_period("M").freqstr == "M"


@case
def mixed_offsets_without_utc(df):
    local = pd.Series(["2024-01-01 00:00+0100", "2024-07-01 00:00+0200"])
    return pd.to_datetime(local, format="%Y-%m-%d %H:%M%z")  # xpd: mixed-offsets-without-utc


@case
def offsets_read_as_utc(df):
    local = pd.Series(["2024-01-01 00:00+0100", "2024-07-01 00:00+0200"])
    return pd.to_datetime(local, format="%Y-%m-%d %H:%M%z", utc=True).dt.tz_convert("Europe/Copenhagen"), pd.to_datetime(df["raw"], format="%Y-%m-%d")


@case
def pytz_error_handler(df):
    import pytz
    from pytz.exceptions import AmbiguousTimeError

    try:
        first = df["d"].dt.tz_localize("Europe/Copenhagen")  # xpd: tz-localize-without-dst
    except pytz.exceptions.NonExistentTimeError:  # xpd: removed-in-pandas-3
        first = None
    try:
        second = df["d"].dt.tz_localize("Europe/Copenhagen")  # xpd: tz-localize-without-dst
    except (pytz.AmbiguousTimeError, KeyError):  # xpd: removed-in-pandas-3
        second = None
    try:
        third = df["d"].dt.floor("h")
    except AmbiguousTimeError:  # xpd: removed-in-pandas-3
        third = None
    return first, second, third


@case
def error_handler_that_stays(df):
    import pytz

    zone = pytz.timezone("Europe/Copenhagen")
    try:
        first = df["d"].dt.tz_localize("Europe/Copenhagen")  # xpd: tz-localize-without-dst
    except (pytz.exceptions.NonExistentTimeError, ValueError):
        first = None
    try:
        second = zone.localize(df["d"].iloc[0].to_pydatetime(), is_dst=None)
    except pytz.exceptions.AmbiguousTimeError:
        second = None
    try:
        third = df["d"].dt.tz_localize("Europe/Copenhagen")  # xpd: tz-localize-without-dst
    except ValueError as error:
        raise pytz.exceptions.InvalidTimeError(str(error)) from error
    return first, second, third


@case
def tz_localize_without_dst(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        df["d"].dt.tz_localize("Europe/Copenhagen"),  # xpd: tz-localize-without-dst
        ts.tz_localize("Europe/Copenhagen", ambiguous="infer"),  # xpd: tz-localize-without-dst
        ts.tz_localize(tz="US/Eastern", nonexistent="shift_forward"),  # xpd: tz-localize-without-dst
    )


@case
def tz_localize_that_stays(df):
    zone = "Europe/Copenhagen"
    return (
        df["d"].dt.tz_localize("UTC"),
        df["d"].dt.tz_localize("Etc/GMT-1"),
        df["d"].dt.tz_localize("+01:00"),
        df["d"].dt.tz_localize("Europe/Copenhagen", ambiguous="NaT", nonexistent="shift_forward"),
        df["d"].dt.tz_localize(zone, ambiguous="NaT", nonexistent="NaT").dt.tz_localize(None),
        pd.Timestamp("2024-01-01").tz_localize("Europe/Copenhagen"),
    )


@case
def date_range_then_localize(df):
    zone = "Europe/Copenhagen"
    return (
        pd.date_range("2024-05-01", "2024-05-03", freq="15min").tz_localize("Europe/Copenhagen"),  # xpd: date-range-then-localize
        pd.date_range("2024-05-01", periods=96, freq="15min").tz_localize(zone),  # xpd: date-range-then-localize
    )


@case
def date_range_with_zone(df):
    return pd.date_range("2024-05-01", "2024-05-03", freq="15min", tz="Europe/Copenhagen"), pd.date_range("2024-05-01", periods=4).tz_localize(None)


@case
def apply_time_zone_method(df):
    utc = df["d"].dt.tz_localize("UTC")
    return (
        utc.apply(lambda t: t.replace(tzinfo=None)),  # xpd: apply-date-method
        utc.map(lambda t: t.astimezone("Europe/Copenhagen")),  # xpd: apply-date-method
    )


@case
def time_zone_method_that_stays(df):
    utc = df["d"].dt.tz_localize("UTC")
    zones = {2022: "Europe/Copenhagen", 2023: "Europe/London"}
    return utc.dt.tz_localize(None), utc.head(5).apply(lambda t: t.astimezone(zones[t.year])), utc.dt.tz_convert("Europe/Copenhagen")


@case
def to_datetime_then_localize(df):
    return (
        pd.to_datetime(df["raw"]).dt.tz_localize("UTC"),  # xpd: to-datetime-then-localize
        pd.to_datetime(df["raw"].tolist(), format="%Y-%m-%d").tz_localize("utc"),  # xpd: to-datetime-then-localize
    )


@case
def to_datetime_with_utc(df):
    return pd.to_datetime(df["raw"], utc=True), pd.to_datetime(df["raw"]).dt.tz_localize("Europe/Copenhagen", ambiguous="NaT", nonexistent="NaT")


@case
def convert_then_drop_zone(df):
    local = df["d"].dt.tz_localize("UTC").dt.tz_convert("Europe/Copenhagen")
    ts = df.set_index(local).sort_index()[["b"]]
    return (
        local.dt.tz_convert("UTC").dt.tz_localize(None),  # xpd: convert-then-drop-zone
        ts.tz_convert("UTC").tz_localize(None),  # xpd: convert-then-drop-zone
        ts.index.tz_convert("utc").tz_localize(None),  # xpd: convert-then-drop-zone
    )


@case
def zone_dropped_another_way(df):
    local = df["d"].dt.tz_localize("UTC").dt.tz_convert("Europe/Copenhagen")
    return local.dt.tz_convert(None), local.dt.tz_localize(None), local.dt.tz_convert("Europe/London").dt.tz_localize(None)


@case
def index_plus_offset(df):
    ts = df.set_index("d").sort_index()[["b"]]
    ts.index = ts.index + pd.Timedelta(hours=6)  # xpd: index-plus-offset
    ts.index += timedelta(days=1)  # xpd: index-plus-offset
    return ts


@case
def index_moved_by_shift(df):
    ts = df.set_index("d").sort_index()[["b"]]
    other = ts.copy()
    other.index = ts.index + pd.Timedelta(hours=6)
    return ts.shift(freq=pd.Timedelta(hours=6)), other


@case
def date_range_slice(df):
    return pd.date_range("2024-03-30", "2024-04-01", freq="15min")[:-1]  # xpd: date-range-slice


@case
def date_range_without_end_point(df):
    return pd.date_range("2024-03-30", "2024-04-01", freq="15min", inclusive="left"), pd.date_range("2024-03-30", periods=10, freq="15min")[:-1]


@case
def index_time_mask(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        ts[ts.index.time == datetime(2024, 1, 1, 12).time()],  # xpd: index-time-mask
        ts.loc[ts.index.time == datetime(2024, 1, 1, 12, 30).time()],  # xpd: index-time-mask
        ts[(ts.index.hour >= 8) & (ts.index.hour < 16)],  # xpd: index-time-mask
    )


@case
def time_of_day_that_stays(df):
    ts = df.set_index("d").sort_index()[["b"]]
    start, end = 8, 16
    return ts.at_time("12:00"), ts.between_time("08:00", "16:00", inclusive="left"), ts[(ts.index.hour >= start) & (ts.index.hour < end)], ts[(ts.index.hour >= 8) & (ts.index.hour < 24)]


@case
def reindex_to_date_range(df):
    ts = df.set_index("d").sort_index()[["b"]].resample("h").mean().dropna()
    return ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="h"))  # xpd: reindex-to-date-range


@case
def reindex_that_stays(df):
    ts = df.set_index("d").sort_index()[["b"]].resample("h").mean().dropna()
    return ts.asfreq("h"), ts.reindex(pd.date_range("2022-01-01", "2022-02-01", freq="h"))


@case
def divide_by_shift(df):
    n = 4
    return (
        df["w"] / df["w"].shift() - 1,  # xpd: divide-by-shift
        (df["w"] - df["w"].shift(96)) / df["w"].shift(96),  # xpd: divide-by-shift
        df["w"] / df["w"].shift(n) - 1,  # xpd: divide-by-shift
    )


@case
def ratio_that_stays(df):
    return df["w"].pct_change(96), df["w"] / df["a"].shift() - 1, df["w"] / df["w"].shift(fill_value=1) - 1


@case
def date_part_key(df):
    ts = df.set_index("d").sort_index()[["b"]]
    return (
        df.groupby([df["d"].dt.year, df["d"].dt.month])["b"].sum(),  # xpd: date-part-key
        ts.groupby([ts.index.year, ts.index.month], sort=False).sum(),  # xpd: date-part-key
    )


@case
def month_key_that_stays(df):
    return df.groupby(pd.Grouper(key="d", freq="MS"))["b"].sum(), df.groupby([df["d"].dt.year, df["k"]])["b"].sum()
