"""A module that xpd reports on line 1, because its name does not say what it holds.

xpd: vague-module-name
"""

__all__ = ["clamp"]


def clamp(value: int, low: int, high: int) -> int:
    return max(low, min(value, high))
