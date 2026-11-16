"""A module that chooses the look of the px figures alone.

px.defaults changes what the px functions draw. A go.Figure does not read it, so
`default-template` is silent in a file of px figures that imports this module, and it still
reports a file of go.Figure calls.
"""

import plotly.express as px

px.defaults.template = "simple_white"
