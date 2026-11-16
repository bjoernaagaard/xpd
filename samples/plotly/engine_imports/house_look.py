"""The module of a project that chooses the look of every figure.

A module that imports this one draws with "simple_white", so `default-template` is silent
there.
"""

import plotly.io as pio

pio.templates.default = "simple_white"

PALETTE = ["#4c78a8", "#f58518", "#54a24b"]
