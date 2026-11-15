# MCP server

`xpd mcp` starts an [MCP](https://modelcontextprotocol.io) server, so an agent can run xpd
through tools instead of a shell. It gets every rule and every result from the xpd binary, so
it always matches the installed version. The server is built on
[FastMCP](https://gofastmcp.com) and needs the `mcp` extra.

It gives an agent:

- tools that lint, fix, read the dataflow, profile data and compare two scripts, each with a
  typed result,
- the rules, each with its section of [Rules](rules.md), as tools and resources,
- the three agent skills that ship with xpd,
- prompts for the workflows of the xpd skill,
- tables and a dataflow chart in hosts that render MCP Apps.

## Install

For Claude Code:

```sh
claude mcp add xpd -- uvx --from 'xpd[mcp]' xpd mcp
```

For another client, add the server to its `mcpServers` setting:

```json
{
  "mcpServers": {
    "xpd": {
      "command": "uvx",
      "args": ["--from", "xpd[mcp]", "xpd", "mcp"]
    }
  }
}
```

The client starts the server in the project folder, which becomes the project root. With xpd
installed without the extra, `xpd mcp` prints the `uvx` command above and exits 2.

## Tools

Every path is relative to the project root, and a path outside it is refused.

| Tool | What it does | Changes files |
| --- | --- | --- |
| `check` | Lints files and folders, as [`xpd check`](commands.md#check). `unsafe_fixes` counts unsafe fixes in `fixable` too | No |
| `fix` | Writes the safe fixes, then lints what is left. With `unsafe`, it asks the user to confirm before it writes unsafe fixes too | Yes |
| `graph` | Reads the dataflow of step code and Hamilton modules, as [`xpd graph`](commands.md#graph) | No |
| `profile` | Profiles a data file, as [`xpd profile`](commands.md#profile). Needs pandas in the project's Python | No |
| `equiv` | Runs two scripts and compares their `result`, as [`xpd equiv`](commands.md#equiv). Needs pandas in the project's Python | Whatever the scripts do |
| `rules` | Lists the rules, filtered by `package`, `severity`, `fixable` and a text `query` | No |
| `rule` | Shows one rule with its documentation and examples | No |
| `show_issues` | Shows what `check` finds as a table | No |
| `show_dataflow` | Shows what `graph` reads as a chart and a table | No |
| `show_columns` | Shows what `profile` reads as a table | No |

The first seven return structured results with an output schema. The `show_` tools return a UI
for hosts that render [MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps), and the
same JSON as text for hosts that do not.

`check`, `fix`, `profile` and `equiv` can run as background tasks for clients that ask for one.
They report progress and log what they run.

## Resources

| URI | Content |
| --- | --- |
| `xpd://rules` | Every rule as JSON, without its documentation |
| `xpd://rules/{package}` | The rules of one pack, such as `xpd://rules/pandas` |
| `xpd://rules/{package}/{rule}` | One rule's documentation as Markdown, such as `xpd://rules/pandas/chained-assignment` |
| `xpd://version` | The version of the xpd binary |
| `skill://xpd/SKILL.md` | The [xpd skill](claude-code.md#agent-skill) |
| `skill://python-best-practices/SKILL.md` | The Python style skill |
| `skill://python-review/SKILL.md` | The Python review skill, with its template |

A client can complete the `package` and `rule` parts of a URI, and a rule is completed within
the package already chosen. Clients that support the Skills extension also list the skills.

## Prompts

| Prompt | Arguments | What the agent does |
| --- | --- | --- |
| `lint_and_fix` | `path` | Lints, fixes, and lints again until only explained warnings are left |
| `review` | `target`, `package`, `rule` | Reviews a path or a git range with the review skill and `check` |
| `refactor_data_logic` | `path`, `goal` | Refactors a script and proves with `equiv` that its result did not change |
| `add_to_dataflow` | `path`, `change` | Adds a step or node, with `graph` before and after |

`package` and `rule` complete as they do in resource URIs.

## Settings

Each flag has an environment setting, for clients that start the server without flags.

| Flag | Environment | Default | Effect |
| --- | --- | --- | --- |
| `--root PATH` | `XPD_MCP_ROOT` | The working directory | The project root. Every path must be inside it |
| `--code-mode` | `XPD_MCP_CODE_MODE=1` | Off | Shows the tools through FastMCP's [code mode](https://gofastmcp.com/docs/servers/transforms/code-mode) as `search`, `get_schema` and `execute` |
| `--transport stdio\|http` | `FASTMCP_TRANSPORT` | `stdio` | How clients connect |
| `--host HOST` | `FASTMCP_HOST` | `127.0.0.1` | The address to bind for HTTP |
| `--port PORT` | `FASTMCP_PORT` | `8000` | The port for HTTP |

`xpd mcp -h` lists the flags. Most clients start a server with only the environment variables
they are given, so set these in the client's `env` setting rather than in your shell.

## HTTP

To serve several clients, or a client on another machine, start the server over HTTP:

```sh
uvx --from 'xpd[mcp]' xpd mcp --transport http --port 8000
```

Clients connect to `http://127.0.0.1:8000/mcp`. The server binds `127.0.0.1` unless `--host`
says otherwise. It has no authentication, so bind another address only on a network you trust.

## Other ways to run it

FastMCP's own commands work with the server. `fastmcp run -m xpd.mcp` starts it as a module,
configured from the environment settings above, and the client commands start `xpd mcp`:

```sh
fastmcp list --command 'xpd mcp' --resources --prompts
fastmcp call --command 'xpd mcp' --target rule --input-json '{"package": "pandas", "rule": "row-loop"}'
```
