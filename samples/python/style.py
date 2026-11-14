"""Style that xpd reports in any Python file, and the same code without it.

A line that xpd reports ends with a comment that names the rule.
"""

import logging
import os
from dataclasses import dataclass
from functools import cache
from typing import Any, TypeVar, cast
from unittest.mock import patch  # xpd: mock-import

__all__ = ["rows", "rows_with_loop"]

_LIMIT = 10  # xpd: module-private-name
LIMIT = 10
_ = LIMIT
_T = TypeVar("_T")

logging.basicConfig(level=logging.INFO)  # xpd: import-side-effect
REGION = os.environ.get("REGION", "eu")  # xpd: import-side-effect
NAMES = ["id"]
NAMES.append("name")
log = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(level=os.environ.get("LOG_LEVEL", "INFO"))


def _prices(item: dict) -> dict:  # xpd: module-private-name
    return {"price": item["price"]}


@cache
def _cached() -> int:  # xpd: module-private-name
    return LIMIT


class _Row:  # xpd: module-private-name
    def _total(self) -> int:
        return 0


class Row:
    _count = 0

    def _total(self) -> int:
        _local = 1
        return _local


def prices(item: dict) -> dict:
    def _inner() -> dict:
        return {"price": item["price"]}

    return _inner()


def rows(items: list[dict]) -> dict:
    return {  # xpd: comprehension-merges-dicts
        item["id"]: {"name": item["name"], **prices(item)} for item in items
    }


def flat(item: dict) -> dict:
    return {
        "id": item["id"],
        # xpd reports the next line as unpacked-comprehension.
        **{f"tag_{name}": value for name, value in item["tags"].items()},
    }


def merged(item: dict, extra: dict) -> dict:
    return {"id": item["id"], **extra}


def names(items: list[dict]) -> list[str]:
    return [item["name"] for item in items if item["name"]]


def rows_with_loop(items: list[dict]) -> dict:
    found = {}
    for item in items:
        row = {"name": item["name"]}
        row.update(prices(item))
        for name, value in item["tags"].items():
            row[f"tag_{name}"] = value
        found[item["id"]] = row
    return found


def flatten(lists: list[list[int]]) -> list[int]:
    return [x for xs in lists for x in xs]  # xpd: comprehension-two-fors


def flatten_with_loop(lists: list[list[int]]) -> list[int]:
    found = []
    for xs in lists:
        found.extend(xs)
    return found


def price(payload: dict[str, Any]) -> float:  # xpd: dict-str-any
    return cast(float, payload["price"])  # xpd: typing-cast


@dataclass(frozen=True, slots=True, kw_only=True)
class Item:
    price: float


def decode(payload: object) -> Item:
    if not isinstance(payload, dict) or not isinstance(payload.get("price"), float):
        raise ValueError(f"not an item: {payload!r}")
    return Item(price=payload["price"])


def first(values: list[_T]) -> _T:
    with patch("os.getcwd"):
        return values[0]
