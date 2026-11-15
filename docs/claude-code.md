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

## Agent skill

The skill that ships with xpd tells the agent when to run `profile` and `equiv`. Save it with the command below.

```sh
mkdir -p .claude/skills/xpd && xpd skill > .claude/skills/xpd/SKILL.md
```

## MCP server

The hook shows the agent the issues of each edit. The [MCP server](mcp.md) adds tools the agent
calls itself, such as `fix`, `rule` and `equiv`. The command below adds the server.

```sh
claude mcp add xpd -- uvx --from 'xpd[mcp]' xpd mcp
```
