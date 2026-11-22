"""Style that xpd reports in any Python file, and the same code without it.

A line that xpd reports ends with a comment that names the rule.
"""

import logging
import os
import json
import time
import tomllib
from collections import Counter, defaultdict, deque
from dataclasses import dataclass
from datetime import date, datetime
from functools import cache
from typing import Any, NamedTuple, TypeVar, cast
from unittest.mock import patch  # xpd: mock-import

import toml
import tomli  # xpd: toml-without-tomllib

import numpy as np
import orjson

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
    return {  # xpd: comprehension-merges-dicts, comprehension-multi-line
        item["id"]: {"name": item["name"], **prices(item)} for item in items
    }


def flat(item: dict) -> dict:
    return {
        "id": item["id"],
        **{f"tag_{name}": value for name, value in item["tags"].items()},  # xpd: unpacked-comprehension
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
class Record:  # xpd: record-without-namedtuple
    price: float


class Item(NamedTuple):
    price: float


def load(body: bytes) -> float:
    return json.loads(body)["price"]  # xpd: json-without-type


def decode(body: bytes) -> Item:
    return Item(**json.loads(body))


def by_length(words: list[str]) -> dict[int, list[str]]:
    found: dict[int, list[str]] = {}
    for word in words:
        found.setdefault(len(word), []).append(word)  # xpd: setdefault-without-defaultdict
    return found


def by_length_with_defaultdict(words: list[str]) -> dict[int, list[str]]:
    found: defaultdict[int, list[str]] = defaultdict(list)
    for word in words:
        found[len(word)].append(word)
    return found


def add_word(found: dict[int, list[str]], word: str) -> None:
    found.setdefault(len(word), []).append(word)


def word_counts(words: list[str]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for word in words:  # xpd: counting-loop-without-counter
        counts[word] = counts.get(word, 0) + 1
    return counts


def most_common_word(words: list[str]) -> str:
    return Counter(words).most_common(1)[0][0]


def length_counts(words: list[str]) -> dict[int, int]:
    counts: dict[int, int] = {}
    for word in words:
        counts[len(word)] = counts.get(len(word), 0) + 1
    return counts


def trading_day(text: str) -> date:
    return datetime.strptime(text, "%Y-%m-%d").date()  # xpd: strptime-without-fromisoformat


def trading_day_from_iso(text: str) -> date:
    return date.fromisoformat(text.strip())


def danish_day(text: str) -> date:
    return datetime.strptime(text, "%d-%m-%Y").date()


def timed(words: list[str]) -> float:
    start = time.time()
    word_counts(words)
    return time.time() - start  # xpd: elapsed-without-perf-counter


def timed_with_perf_counter(words: list[str]) -> float:
    start = time.perf_counter()
    word_counts(words)
    return time.perf_counter() - start


def age(modified: float) -> float:
    return time.time() - modified


def region(path: str) -> str:
    return toml.load(path)["region"]  # xpd: toml-without-tomllib


def region_with_tomllib(text: str) -> str:
    return tomllib.loads(text)["region"]


def region_as_toml(name: str) -> str:
    return toml.dumps({"region": name}) + tomli.__name__


def newest_first(events: list[str]) -> list[str]:
    found: list[str] = []
    for event in events:
        found.insert(0, event)  # xpd: front-insert-in-loop
    return found


def newest_first_with_reverse(events: list[str]) -> list[str]:
    found: list[str] = []
    for event in events:
        found.append(event)
    found.reverse()
    return found


def newest_first_with_deque(events: list[str]) -> deque[str]:
    found: deque[str] = deque()
    for event in events:
        found.appendleft(event)
    return found


def with_header(events: list[str], found: list[str]) -> list[str]:
    for event in events:
        found.insert(0, event)
    found.insert(0, "header")
    return found


def after_header(events: list[str]) -> list[str]:
    found = ["header"]
    for event in events:
        found.insert(0, event)
        found.insert(1, event)
    return found


def read_item(body: bytes) -> Item:
    return Item(**orjson.loads(body.decode("utf-8")))  # xpd: orjson-loads-decoded


def read_item_from_bytes(body: bytes) -> Item:
    return Item(**orjson.loads(body))


def read_latin_item(body: bytes) -> Item:
    return Item(**orjson.loads(body.decode("latin-1")))


def read_lenient_item(body: bytes) -> Item:
    return Item(**orjson.loads(body.decode("utf-8", errors="replace")))


def read_item_with_json(body: bytes) -> Item:
    return Item(**json.loads(body.decode()))


class Quote:
    __slots__ = ("symbol", "price")

    def __init__(self, symbol: str, price: float) -> None:
        self.symbol = symbol
        self.price = price


class SizedQuote(Quote):  # xpd: slotted-base-without-slots
    def __init__(self, symbol: str, price: float, size: int) -> None:
        super().__init__(symbol, price)
        self.size = size


class SizedQuoteWithSlots(Quote):
    __slots__ = ("size",)

    def __init__(self, symbol: str, price: float, size: int) -> None:
        super().__init__(symbol, price)
        self.size = size


class LabeledQuote(Quote):
    __slots__ = ()

    def label(self) -> str:
        return f"{self.symbol} {self.price}"


@dataclass(slots=True)
class TradedQuote(Quote):
    volume: int


class QuoteError(ValueError):
    pass


def spread(values: np.ndarray) -> float:
    return float(values.std())  # xpd: same-type-conversion


def spread_without_float(values: np.ndarray) -> float:
    return values.std()


def by_column(values: np.ndarray) -> list[float]:
    return list(values.mean(axis=0))


def first_value(values: np.ndarray) -> float:
    return float(values[0])  # xpd: float-around-number


def parsed(fields: list[str]) -> float:
    return float(fields[0])


def shape(values: np.ndarray, name: str, day: object) -> tuple[int, str, bool, float]:
    return (
        int(values.size),  # xpd: same-type-conversion
        str(f"{name}:{day}"),  # xpd: same-type-conversion
        bool(name.startswith("a")),  # xpd: same-type-conversion
        float(np.sqrt(values[0])),  # xpd: same-type-conversion
    )


def counts(values: np.ndarray) -> tuple[int, bool]:
    return int(values.argmax()), bool(values[0] < 1)


def decoded(body: bytes) -> Item:  # xpd: pass-through-function
    return decode(body)


def total(items: list[Item]) -> float:
    return sum(item.price for item in items)


def long_names(items: list[dict]) -> list[str]:
    return [  # xpd: comprehension-multi-line
        item["name"].strip().lower()
        for item in items
        if item["name"] and len(item["name"]) > LIMIT
    ]


def joined(items: list[dict], sep: str) -> str:
    return print([item["name"] for item in items], sep=sep)  # xpd: comprehension-among-arguments


def first(values: list[_T]) -> _T:
    with patch("os.getcwd"):
        return values[0]


def nested(items: list[dict]) -> list[str]:
    found = []
    found.append(
        "{name}:{price}".format(  # xpd: multi-line-call-argument
            name=items[0]["name"],
            price=items[0]["price"],
        )
    )
    return found


def flat_calls(items: list[dict]) -> list[str]:
    label = "{name}:{price}".format(name=items[0]["name"], price=items[0]["price"])
    return [label]
