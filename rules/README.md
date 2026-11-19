# Rules

Each rule has a page that says what goes wrong, what to write instead, and whether xpd can make
the edit for you. Open a rule from the link in an issue, or search for its id, such as
`pandas/chained-assignment`.

xpd names the rule in each issue it prints. The id in brackets is the address of the rule's page.

```text
bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
```

The same text reaches you in four places. `xpd rules --format json` prints it as the `doc` field of
each rule, the editor shows it when you hover over an issue, the MCP server returns it as the
resource `xpd://rules/<pack>/<rule>`, and this site shows it as the rule's page.

A rule's fix tells you how much xpd changes on its own.

| Fix | What the edit does | How to apply it |
| --- | --- | --- |
| safe | keeps what the code does | `xpd check --fix` |
| unsafe | can change what the code does, so read the edit | `xpd check --fix --unsafe-fixes` |
| none | xpd shows the change as advice | edit the code by hand |

A pack holds the rules for one library. xpd runs a pack on the files that import its library,
and the Python and xpd packs on every file.
