"""Public names shared by registration and MCP workflow instructions."""

from enum import StrEnum, auto


class ToolName(StrEnum):
    FIND = auto()
    API = auto()
    DISCOVERY_GATE = auto()
    EQUIV = auto()
    CHECK = auto()
    CHART_REVIEW = auto()
    FIX = auto()
    QUERY = auto()
    REWRITE = auto()
    OUTLINE = auto()
    NEW = auto()
    TEST = auto()
    RULE = auto()
    RULES = auto()
    GRAPH = auto()
    PROFILE = auto()
    GATES = auto()
    REFIT = auto()
    CONTRACT = auto()
    ANIMATE_GUIDE = auto()
    ANIMATE_STATUS = auto()
    ANIMATE_RENDER = auto()
    ANIMATE_ARTIFACT = auto()
    ANIMATE_SOURCE = auto()
