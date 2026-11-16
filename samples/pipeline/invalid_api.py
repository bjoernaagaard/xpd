"""Show the discovery gate refusing a made up class and keyword before execution."""

from sklego.meta import GroupedPredictor, ImaginaryPredictor

model = GroupedPredictor(group_by="group")
missing = ImaginaryPredictor()
