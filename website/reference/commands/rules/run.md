Use `xpd rules run` to find code by its syntax, in any language that ast-grep parses, and to preview
or apply a rewrite of each match. It reads the code without running it. A pattern such as
`$X.iterrows()` matches the code it spells out, where `$X` stands for any one syntax node and
`$$$ARGS` for any number of them. `xpd rules query` is another name for the same command.

## Examples

```console
$ xpd rules run --lang python --pattern '$X.iterrows()' samples/bad.py --format text
samples/bad.py:5:for i, row in df.iterrows():
```

A rewrite only prints a diff until you pass `-U` to write every rewrite, or `-i` to choose each
one. xpd does not check a rewrite or mark it safe the way it does a rule fix, so read the diff
before you write it.

```console
$ xpd rules query -p 'pd.merge($$$A)' -r 'df.merge(other, on="k")' samples/bad.py --format text --color never
samples/bad.py
@@ -4,7 +4,7 @@
5  5 │ for i, row in df.iterrows():
6  6 │     df["a"][i] = row["a"] * 2
7  7 │ df.dropna(inplace=True)
8    │-m = pd.merge(df, other, on="k")
   8 │+m = df.merge(other, on="k")
9  9 │ out = pd.DataFrame()
10 10│ for f in ["x.csv"]:
11 11│     out = pd.concat([out, pd.read_csv(f)])
```

`--format json` prints the matches. Lines and columns in the JSON count from zero, so
`"line":4` is the line that the text output shows as line 5.

```console
$ xpd rules run -p '$X.iterrows()' samples/bad.py --format json | jq -c '.[0] | keys, .range'
["charCount","file","language","lines","metaVariables","range","text"]
{"byteOffset":{"start":99,"end":112},"start":{"line":4,"column":14},"end":{"line":4,"column":27}}
```

`--debug-query` prints the syntax tree of the pattern on stderr, which shows why a pattern
matches less than you expect.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Something matched |
| 1 | Nothing matched |
| 2 | xpd could not run or could not read an explicitly named input path |
