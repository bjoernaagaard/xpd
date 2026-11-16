"""A package that chooses the look of its figures once, for all of its modules.

Python runs this file before any module of the package, so no module has to name a template.
xpd reads it for each of them, and `default-template` is silent in charts.py and tables.py.
"""

import plotly.io as pio

pio.templates.default = "simple_white"
