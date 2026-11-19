Use `xpd profile` before you choose how to load or process a data file. It prints JSON with the
dtype, null count, distinct count and memory use of each column, and findings about how to load
the file. It reads `.csv`, `.parquet`, `.feather`, `.json` and `.xlsx` files with the project's
Python, which needs pandas.

## Examples

```console
$ xpd profile samples/data.csv
{"file": "samples/data.csv", "pandas": "3.1.1", "rows": 3, "memory_bytes": 183, "columns": [{"column": "a", "dtype": "float64", "nulls": 1, "unique": 2, "memory_bytes": 24}, {"column": "b", "dtype": "str", "nulls": 0, "unique": 2, "memory_bytes": 27}], "findings": []}
```

Each finding has `finding`, `columns` and a `message` with what was measured on the file.

```console
$ xpd profile samples/pipeline/penguins.csv | jq -r '.findings[].message'
species holds 3 distinct texts in 344 rows. As a category it took 390 bytes, where it takes 5,020 as str. Pass dtype={"species": "category"} to the reader, or call astype("category").
island holds 3 distinct texts in 344 rows. As a category it took 389 bytes, where it takes 4,848 as str. Pass dtype={"island": "category"} to the reader, or call astype("category").
sex holds 2 distinct texts in 344 rows. As a category it took 371 bytes, where it takes 4,457 as str. Pass dtype={"sex": "category"} to the reader, or call astype("category").
```

| Finding | When |
| --- | --- |
| `values-as-column-names` | Three or more column names are years, dates, or a stem with a number, as `sales_2022`, `sales_2023` and `sales_2024` are. The message names the `melt` call that gives one row per value |
| `category-candidate` | A text column has at most half as many distinct values as rows, and takes at most half the memory as a category |

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | It printed a profile |
| 2 | It could not run, as when the project's Python has no pandas or the file cannot be read |
