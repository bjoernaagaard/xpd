# Claude Code

Add `xpd hook` as a `PostToolUse` hook, so Claude Code lints each Python file the agent edits
and shows the issues to the agent in the same turn. Add the hook below to
`.claude/settings.json` in your project.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [{ "type": "command", "command": "xpd hook" }]
      }
    ]
  }
}
```

## What the hook does

Claude Code sends the event of each edit to `xpd hook` on stdin, and xpd lints the file named in
`tool_input.file_path`. When xpd finds issues, it prints them on stderr and exits 2, and Claude
Code shows them to the agent.

```console
$ echo '{"tool_name":"Edit","tool_input":{"file_path":"samples/bad.py"},"tool_response":{}}' | xpd hook
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
$ echo $?
2
```

The hook writes no fix. For a file with a pipeline of steps, the issues also include the errors
and warnings of [`xpd graph`](commands.md#step-code), such as a formula that another step
computes already. In a project with its own rules, which has an `sgconfig.yml`, the hook also
lints an edited file in another language that xpd can parse, so the project's rules for that
language run on it.

| Exit code | When | What Claude Code does |
| --- | --- | --- |
| 0 | The file is clean, or xpd doesn't lint it | Nothing |
| 1 | xpd could not run, such as when stdin is not hook JSON | Shows the message to you, and not to the agent |
| 2 | xpd found issues | Shows the issues to the agent |

## Agent plugin

A client that supports the [Agent Plugins](https://github.com/agentplugins/agent-plugins-spec)
format, version 1.0.0, can load the xpd repository as a plugin and get the MCP server and the
skills with no settings to write. The plugin starts xpd with `uvx`, so the machine needs
[uv](https://docs.astral.sh/uv/).

```text
xpd, loaded as a plugin
├── plugin.json                         the name and version of the plugin, 0.30.0
├── skills/                             xpd, python-best-practices and python-review
├── mcp.json                            starts the MCP server with uvx xpd@0.30.0 mcp
└── .agents/plugins/marketplace.json    lists the plugin for Codex
```

Codex installs the plugin with the two commands below.

```sh
codex plugin marketplace add bjoernaagaard/xpd
codex plugin add xpd@xpd
```

Version 1.0.0 of the format has no hooks, so the plugin ships none. The 1.1.0 draft of the
format adds hooks, and `xpd hook` already answers a client of that draft. xpd tells the two
kinds of client apart by the field that holds the result of the tool call.

| Client | Field in the event | Files that xpd lints | How the agent gets the issues |
| --- | --- | --- | --- |
| Agent Plugins 1.1.0 draft | `tool_result` | The `path` of each `edit` and `write`, and the `to` of each `move`, in the `file.modify` profile | `additional_context` in JSON on stdout, with exit code 0 |
| Claude Code settings | `tool_response` | `tool_input.file_path` | stderr, with exit code 2 |

A hook of the draft format runs after every tool call, and xpd prints nothing for a call that
changed no Python file. Such a hook starts in the plugin's folder, so xpd moves to the `cwd` of
the event before it lints, and it finds the project's venv there.

## Agent skill

The xpd skill tells the agent which xpd command to run and when, such as `xpd profile` before
it chooses methods for a data file and `xpd equiv` after it refactors data logic. Run the
command below to save the skill of your xpd version in the project.

```sh
mkdir -p .claude/skills/xpd && xpd skill > .claude/skills/xpd/SKILL.md
```

`xpd skill --json` prints every skill that ships with xpd, with the path and text of each file.

## MCP server

Add the [MCP server](mcp.md) to give the agent tools it calls itself, such as `fix`, `rule` and
`equiv`, beside the hook that checks each edit.

```sh
claude mcp add xpd -- uvx xpd mcp
```
