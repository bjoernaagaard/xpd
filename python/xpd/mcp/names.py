"""Public names shared by registration and MCP workflow instructions."""

from enum import StrEnum, auto


class ToolName(StrEnum):
    DISCOVERY_GATE = auto()
    GATES = auto()
    CONTRACT = auto()
    ANIMATE_GUIDE = auto()
    ANIMATE_STATUS = auto()
    ANIMATE_RENDER = auto()
    ANIMATE_ARTIFACT = auto()
    ANIMATE_SOURCE = auto()
