"""Prompts for the workflows of the xpd skill. Each names the tools to run and when it is done."""

from fastmcp import FastMCP


def register(mcp: FastMCP) -> None:
    @mcp.prompt(tags={"lint"})
    def lint_and_fix(path: str = ".") -> str:
        """Lint a path with xpd, fix what it reports, and lint again until it is clean.

        Args:
            path: A file or folder inside the project root.
        """
        return f"""Lint {path} with xpd and fix what it finds.

1. Call the `check` tool with paths ["{path}"].
2. Call the `fix` tool with the same paths to write the safe fixes.
3. For each issue left, read the rule with the `rule` tool and change the code as its
   example shows. Fix every `error`. Fix a `warning` unless you can say why the code is right
   here, and say so. Don't silence rules.
4. If an issue has an unsafe fix, you may call `fix` with `unsafe` true. The user is asked to
   confirm. Read the diff afterwards.
5. A `missing-<package>` warning names a package the project lacks. Ask the user before you add
   it.
6. Call `check` again. You are done when it reports no error and every warning left has a
   reason."""

    @mcp.prompt(tags={"review"})
    def review(target: str, package: str = "", rule: str = "") -> str:
        """Review a file, a folder or a diff against xpd's rules and the Python review skill.

        Args:
            target: A path inside the project root, or a git range such as main..HEAD.
            package: Look closest at this pack's rules, such as pandas.
            rule: Look closest at this rule, such as chained-assignment.
        """
        focus = ""
        if rule and package:
            focus = (
                f"\nLook closest at the rule {package}/{rule}. "
                f"Read xpd://rules/{package}/{rule} first."
            )
        elif package:
            focus = f"\nLook closest at the {package} rules. xpd://rules/{package} lists them."
        return f"""Review {target} and report what is slow, wrong or hard to maintain.{focus}

1. Read skill://python-review/SKILL.md and follow it. It cites a rule for every finding.
2. If {target} is a git range, list the changed .py files with git and review those.
3. Call the `check` tool on the files. Each issue is a finding. Cite it as
   `<package>/<rule>` and link the rule's `url` from the `rule` tool.
4. If a file has a pipeline of steps or Apache Hamilton nodes, call the `graph` tool on it and
   add its errors and warnings as findings.
5. Then read the code for what a linter cannot see, as the skill describes.
6. Report the findings, worst first, each with the file and line, the rule, and the fix. Change
   no file unless the user asks."""

    @mcp.prompt(tags={"refactor"})
    def refactor_data_logic(path: str, goal: str) -> str:
        """Refactor data logic and prove with `equiv` that the result did not change.

        Args:
            path: The script to refactor, inside the project root.
            goal: What the refactor should achieve, such as "replace the row loop".
        """
        return f"""Refactor {path} so that: {goal}

1. If the script reads a data file, call the `profile` tool on it first, so your choices fit
   the real dtypes, nulls and sizes.
2. Copy {path} to a new file beside it, such as {path}.old.py, before you change anything.
   Both files must leave their output in a variable named `result`. Add one if it is missing.
3. Change {path}. Call the `check` tool on it and fix what it reports.
4. Call the `equiv` tool with old set to the copy and new set to {path}. The refactor is done
   only when it returns `equivalent` true. If it returns false, read `detail` and fix the new
   code, not the comparison.
5. Delete the copy, and tell the user what changed and that `equiv` passed."""

    @mcp.prompt(tags={"dataflow"})
    def add_to_dataflow(path: str, change: str) -> str:
        """Add or change a step of step code or a node of Apache Hamilton modules, and check the
        dataflow with `graph` before and after.

        Args:
            path: The module with the steps or nodes, inside the project root.
            change: The step or node to add, move or rename, and what it computes.
        """
        return f"""Make this change to the dataflow in {path}: {change}

1. Call the `graph` tool on {path} first. For Apache Hamilton, pass every module of the
   Driver. Read `pipelines[].steps` or `nodes` to see what exists.
2. Reuse a column or node that exists instead of computing its formula again.
3. In step code, return `frame.assign(...)` or `frame.join(...)`, and don't write
   `frame["x"] = ...` on the frame a step was given.
4. Make the change, then call `graph` again. Fix every `error`. Fix a `warning` unless you can
   say why the code is right. For Hamilton, a warning about an input one edit from a node name
   is usually a typing error.
5. Call the `check` tool on {path} and fix what it reports."""
