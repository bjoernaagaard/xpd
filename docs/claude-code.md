# Claude Code

`xpd hook` runs as a `PostToolUse` hook. After each edit to a `.py` file, Claude Code sends
the event to xpd. When xpd finds issues, it prints them and exits 2. Claude Code then shows
the issues to the agent in the same turn. For a file with a pipeline of steps, the issues include
the errors and warnings of [`xpd graph`](commands.md#step-code), such as a formula that another
step computes already.

Add the hook below to `.claude/settings.json` in your project.

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

| Exit code | Effect |
| --- | --- |
| 0 | Clean, or not a Python file |
| 1 | xpd could not run. Claude Code shows the message to you, not the agent |
| 2 | Issues found. Claude Code shows them to the agent |

## Agent plugin

The xpd repository is a plugin in the
[Agent Plugins](https://github.com/agentplugins/agent-plugins-spec) format, version 1.0.0. A
client that supports the format loads the folder and gets the MCP server and the skills, with
no settings to write. The plugin starts xpd with `uvx`, so the machine needs
[uv](https://docs.astral.sh/uv/).

```text
xpd, loaded as a plugin
├── plugin.json                         the name and version of the plugin
├── skills/                             xpd, python-best-practices and python-review
├── mcp.json                            starts the MCP server of this version with uvx
└── .agents/plugins/marketplace.json    lists the plugin for Codex
```

Codex installs it with two commands.

```sh
codex plugin marketplace add bjoernaagaard/xpd
codex plugin add xpd@xpd
```

Version 1.0.0 of the format has no hooks, so the plugin ships none. Version 1.1.0, a draft,
adds them, and `xpd hook` already answers a client of that version. It tells the two kinds of
client apart by the name of the field that holds the result of the tool call.

| Client | Field in the event | Files that xpd lints | How the agent gets the issues |
| --- | --- | --- | --- |
| Agent Plugins 1.1.0 | `tool_result` | Each `edit`, `write` and `move` in the `file.modify` profile | `additional_context` in JSON on stdout, with exit code 0 |
| Claude Code settings | `tool_response` | `tool_input.file_path` | stderr, with exit code 2 |

A hook of that format runs after every tool call, and xpd prints nothing for a call that
changed no Python file. Such a hook starts in the plugin's folder, so xpd moves to the `cwd` of
the event before it lints, which is where it finds the project's venv.

## Agent skill

The skill that ships with xpd tells the agent when to run `profile` and `equiv`. Save it with the command below.

```sh
mkdir -p .claude/skills/xpd && xpd skill > .claude/skills/xpd/SKILL.md
```

## MCP server

The hook shows the agent the issues of each edit. The [MCP server](mcp.md) adds tools the agent
calls itself, such as `fix`, `rule` and `equiv`. The command below adds the server.

```sh
claude mcp add xpd -- uvx xpd mcp
```
