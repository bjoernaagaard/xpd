# Embedded in the xpd binary (include_str!) and run with the project's Python.
# Usage: python -c <this> PATH  → prints JSON column profile to stdout.
import json
import re
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import NoReturn


def fail(reason: str) -> NoReturn:
    """Exits with 2, the code of an xpd command that cannot run."""
    print(reason, file=sys.stderr)
    sys.exit(2)


try:
    import pandas as pd
except ImportError as e:
    fail(f"xpd profile needs pandas in the project's Python. {e}")


def unique_count(s: pd.Series) -> int | None:
    try:
        return int(s.nunique(dropna=True))
    except TypeError:  # cells hold unhashable values such as lists or dicts
        return None


# How many columns must share a pattern before their names count as values, which is also how
# many names a message lists.
SHARED = 3


def listed(names: list[str]) -> str:
    """The first names, then how many more there are."""
    if len(names) <= SHARED:
        return ", ".join(names[:-1]) + " and " + names[-1]
    return ", ".join(names[:SHARED]) + f" and {len(names) - SHARED} more"


# What a column name must look like for the name to be a value: a year, a month or a day, or a
# shared stem that ends in a number, as in sales_2019.
YEAR = re.compile(r"(19|20)\d{2}")
DATE = re.compile(r"\d{4}-\d{2}(-\d{2})?")
STEM = re.compile(r"(.*[^\d_ .-])[_ .-]?\d+")


def wide_findings(names: list[str]) -> Iterator[dict[str, object]]:
    """Sets of three or more columns whose names are values of one variable.

    A frame with such columns is wide: the years stand in the header where they could stand in
    a column of their own.
    """
    groups: dict[str, list[str]] = {}
    for name in names:
        if YEAR.fullmatch(name):
            groups.setdefault("a year", []).append(name)
        elif DATE.fullmatch(name):
            groups.setdefault("a date", []).append(name)
        elif stem := STEM.fullmatch(name):
            groups.setdefault(stem.group(1), []).append(name)
    for what, columns in groups.items():
        if len(columns) < SHARED:
            continue
        others = [name for name in names if name not in columns]
        if what in ("a year", "a date"):
            held = f"are values of one variable, {what}"
        else:
            held = f"share the stem {what} and differ in a number, which is a value"
        yield {
            "finding": "values-as-column-names",
            "columns": columns,
            "message": (
                f"The column names {listed(columns)} {held}, so the file is wide. "
                f"df.melt(id_vars={json.dumps(others)}, var_name=..., value_name=...) gives one "
                "row per value, with the names in a column of their own."
            ),
        }


def category_finding(s: pd.Series, unique: int | None, used: int) -> dict[str, object] | None:
    """A text column that takes at most half the memory as a category, with both sizes."""
    if unique is None or not pd.api.types.is_string_dtype(s.dtype) or unique * 2 > len(s):
        return None
    as_category = int(s.astype("category").memory_usage(deep=True, index=False))
    if as_category * 2 > used:
        return None
    return {
        "finding": "category-candidate",
        "columns": [str(s.name)],
        "message": (
            f"{s.name} holds {unique:,} distinct texts in {len(s):,} rows. As a category it took "
            f"{as_category:,} bytes, where it takes {used:,} as {s.dtype}. Pass "
            f'dtype={{"{s.name}": "category"}} to the reader, or call astype("category").'
        ),
    }


def main() -> None:
    path = Path(sys.argv[1])
    readers = {
        ".csv": pd.read_csv,
        ".parquet": pd.read_parquet,
        ".feather": pd.read_feather,
        ".json": pd.read_json,
        ".xlsx": pd.read_excel,
    }
    reader = readers.get(path.suffix.lower())
    if reader is None:
        fail(f"xpd profile: unsupported file type {path.suffix!r}")
    try:
        df = reader(path)
    except ImportError as e:  # pandas names the optional package it needs, such as openpyxl
        fail(f"xpd profile: reading {path.suffix} files needs an extra package. {e}")
    except (OSError, ValueError) as e:  # pandas raises ValueError subclasses on a malformed file
        fail(f"xpd profile: cannot read {path}. {e}")

    mem = df.memory_usage(deep=True)
    cols = []
    findings = list(wide_findings([str(c) for c in df.columns]))
    for c in df.columns:
        s = df[c]
        unique = unique_count(s)
        cols.append(
            {
                "column": str(c),
                "dtype": str(s.dtype),
                "nulls": int(s.isna().sum()),
                "unique": unique,
                "memory_bytes": int(mem[c]),
            }
        )
        if finding := category_finding(s, unique, int(mem[c])):
            findings.append(finding)
    print(
        json.dumps(
            {
                "file": str(path),
                "pandas": pd.__version__,
                "rows": len(df),
                "memory_bytes": int(mem.sum()),
                "columns": cols,
                "findings": findings,
            }
        )
    )


if __name__ == "__main__":
    main()
