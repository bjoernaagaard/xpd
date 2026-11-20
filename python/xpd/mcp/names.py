"""Public names shared by registration and MCP workflow instructions."""

from enum import StrEnum, auto


class ToolName(StrEnum):
    FIND = auto()
    API = auto()
    DISCOVERY_GATE = auto()
    EQUIV = auto()
    CHECK = auto()
    FIX = auto()
    QUERY = auto()
    REWRITE = auto()
    OUTLINE = auto()
    NEW = auto()
    RULE = auto()
    RULES = auto()
    GRAPH = auto()
    PROFILE = auto()
    GATES = auto()
    REFIT = auto()
    CONTRACT = auto()
