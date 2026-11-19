Use `profile` before choosing methods for a data file, as
[`xpd profile`](../../../commands/profile/) does. It reads the file with pandas in the project's Python and
returns each column's dtype, nulls, distinct values and memory, with findings about how to load
it.

## Example

```json
{"file":"samples/data.csv"}
```

```json
{"file":"samples/data.csv","pandas":"3.1.1","rows":3,"memory_bytes":183,"columns":[{"column":"a","dtype":"float64","nulls":1,"unique":2,"memory_bytes":24},{"column":"b","dtype":"str","nulls":0,"unique":2,"memory_bytes":27}],"findings":[]}
```

## Errors

A file that cannot be read, or a project Python without pandas, fails with the binary's message.

```text
xpd profile -- samples/missing.csv exited with 2: xpd profile: cannot read samples/missing.csv. [Errno 2] No such file or directory: 'samples/missing.csv'
```
