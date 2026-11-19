A title in capitals only is wider and slower to read, because every word has the same rectangular outline. Write the title in upper and lower case.

```diff lang="python"
-fig = px.bar(df, x="region", y="sales", title="SALES BY REGION")
+fig = px.bar(df, x="region", y="sales", title="Sales by region")
```

## What the rule reports

The rule reports a title written in capitals only, with at least two words of four or more capitals, inside a plotly call. A title is a `title`, a `title_text`, a setting that ends in `_title` or `_title_text`, the `text` of a title dict, a value of the `labels` of a px call, or an item of `subplot_titles`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The book lists as friendly "type is upper-and-lower case, with serifs", and as unfriendly "type
is all capitals, sans serif" (page 183). plotly keeps a title as written, and
`title="SALES BY REGION"` resolved to the same text.

| Title drawn at 20 pixels in the default font | Width in pixels |
| --- | --- |
| `"SALES BY REGION"` | 178, which is 1.31 times as wide |
| `"Sales by region"` | 136 |

Two more titles were 1.26 and 1.35 times as wide in capitals. The rule leaves alone short names
in capitals such as `"RSI"`, `"GDP (USD)"` and `"AUC"`, which are abbreviations and right as
written, and a single word such as `"NVIDIA"`. It skips HTML tags such as `<b>`, and leaves alone
an f-string with a field. The rule has no fix, because the case of each word, such as an
abbreviation, is the author's to choose.
