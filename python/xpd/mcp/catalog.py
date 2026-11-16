"""The rule catalog: the `rules` and `rule` tools, the rule resources, and completion of the
package and rule arguments of templates and prompts."""

import json
from collections.abc import Iterable
from dataclasses import dataclass

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import (
    Annotations,
    CompletionArgument,
    CompletionContext,
    InputRequiredResult,
    PromptReference,
    ResourceTemplateReference,
)

from xpd.mcp.binary import RULES, Xpd
from xpd.mcp.models import Rule, RuleList, RuleSummary, Severity
from xpd.mcp.root import Project
from xpd.mcp.tools import READS

RULES_URI = "xpd://rules"
PACK_URI = "xpd://rules/{package}"
RULE_URI = "xpd://rules/{package}/{rule}"
# The rule docs serve the agent. The catalog is a reference it reads when it needs one.
FOR_AGENT = Annotations(audience=["assistant"], priority=0.5)


def summary(rule: Rule) -> RuleSummary:
    return RuleSummary.model_validate(rule.model_dump(exclude={"doc"}))


@dataclass(frozen=True)
class Catalog:
    """Every rule of the binary, read once, since the binary's rules do not change while the
    server runs."""

    rules: tuple[Rule, ...]

    @property
    def packages(self) -> list[str]:
        return list(dict.fromkeys(rule.package for rule in self.rules))

    def of(self, package: str) -> list[Rule]:
        found = [rule for rule in self.rules if rule.package == package]
        if not found:
            msg = f"No pack is named {package!r}. The packs are {', '.join(self.packages)}."
            raise ToolError(msg)
        return found

    def find(self, package: str, rule: str) -> Rule:
        for entry in self.of(package):
            if entry.rule == rule:
                return entry
        msg = f"The {package} pack has no rule {rule!r}. The rules tool lists them."
        raise ToolError(msg)

    def complete(
        self, argument: CompletionArgument, context: CompletionContext | None
    ) -> list[str]:
        """Candidates for a `package` or `rule` argument. A rule is completed within the package
        already chosen, when there is one."""
        if argument.name == "package":
            names = self.packages
        elif argument.name == "rule":
            chosen = (context.arguments or {}).get("package") if context else None
            names = [rule.rule for rule in self.rules if chosen in (None, rule.package)]
        else:
            return []
        return [name for name in names if name.startswith(argument.value)]


async def load(ctx: Context, xpd: Xpd, project: Project, fallback: Catalog) -> Catalog:
    root = await project.root(ctx)
    if isinstance(root, InputRequiredResult):
        return fallback
    return Catalog(tuple(await xpd.run(RULES, root.path, [])))


def register(mcp: FastMCP, catalog: Catalog, xpd: Xpd, project: Project) -> None:
    @mcp.tool(name="rules", tags={"rules"}, annotations=READS)
    async def list_rules(
        ctx: Context,
        package: str | None = None,
        severity: Severity | None = None,
        *,
        fixable: bool | None = None,
        query: str | None = None,
    ) -> RuleList:
        """List xpd's rules and package checks, without their documentation. Every filter is
        optional, and they combine. Use `rule` to read one rule with examples.

        Args:
            package: Only this pack, by module name, such as pandas, numpy, scipy, sklearn,
                sklego, mapie, category_encoders, linearmodels, arch, pymoo or plotly.
            severity: Only rules of this severity.
            fixable: True for rules with a fix, False for rules without one.
            query: Only rules whose id, message or documentation contains this text, in any
                case.
        """
        selected = await load(ctx, xpd, project, catalog)
        rules = selected.of(package) if package else list(selected.rules)
        needle = query.casefold() if query else None
        return RuleList(
            rules=[
                summary(rule)
                for rule in rules
                if (severity is None or rule.severity == severity)
                and (fixable is None or (rule.fix is not None) == fixable)
                and (
                    needle is None
                    or needle in f"{rule.rule}\n{rule.message}\n{rule.doc or ''}".casefold()
                )
            ]
        )

    @mcp.tool(name="rule", tags={"rules"}, annotations=READS)
    async def one_rule(ctx: Context, package: str, rule: str) -> Rule:
        """Read one rule: why it matters, what it reports, and its before and after examples.

        Args:
            package: The rule's pack, by module name, such as pandas.
            rule: The rule id, such as chained-assignment.
        """
        return (await load(ctx, xpd, project, catalog)).find(package, rule)

    @mcp.resource(
        RULES_URI,
        name="rules",
        mime_type="application/json",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def all_rules(ctx: Context) -> str:
        """Every rule and package check of xpd, without their documentation."""
        return dumps(summary(rule) for rule in (await load(ctx, xpd, project, catalog)).rules)

    @mcp.resource(
        PACK_URI,
        name="pack-rules",
        mime_type="application/json",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def pack_rules(ctx: Context, package: str) -> str:
        """The rules of one pack, such as xpd://rules/pandas, without their documentation."""
        return dumps(summary(rule) for rule in (await load(ctx, xpd, project, catalog)).of(package))

    @mcp.resource(
        RULE_URI,
        name="rule-doc",
        mime_type="text/markdown",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def rule_doc(ctx: Context, package: str, rule: str) -> str:
        """One rule's documentation, such as xpd://rules/pandas/chained-assignment."""
        found = (await load(ctx, xpd, project, catalog)).find(package, rule)
        return found.doc or f"#### {found.rule}\n\n{found.message}\n"

    @mcp.completion
    async def complete(
        ref: PromptReference | ResourceTemplateReference,
        argument: CompletionArgument,
        context: CompletionContext | None,
    ) -> list[str] | None:
        if isinstance(ref, ResourceTemplateReference) and ref.uri in {PACK_URI, RULE_URI}:
            return (await load(Context(mcp), xpd, project, catalog)).complete(argument, context)
        if isinstance(ref, PromptReference):
            return (await load(Context(mcp), xpd, project, catalog)).complete(argument, context)
        return None


def dumps(rules: Iterable[RuleSummary]) -> str:
    return json.dumps([rule.model_dump(mode="json") for rule in rules])
