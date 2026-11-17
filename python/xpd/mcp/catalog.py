"""The rule catalog: the `rules` and `rule` tools, the rule resources, and completion of the
package and rule arguments of templates and prompts."""

import json
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

import yaml
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
# The files of a rule project, as the binary looks for them in the root and each folder above.
CONFIG_NAMES = ("sgconfig.yml", "sgconfig.yaml")
MANIFEST_NAME = "xpd.yml"
RULE_SUFFIXES = {".yml", ".yaml"}

# Each file `xpd rules` reads in one root, with its modification time and size.
Inputs = tuple[tuple[str, int, int], ...]


def summary(rule: Rule) -> RuleSummary:
    return RuleSummary.model_validate(rule.model_dump(exclude={"doc"}))


@dataclass(frozen=True)
class Catalog:
    """Every rule that `xpd rules` lists: the built-in rules, and those of the project's own
    rule project when the root has one."""

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


def folders(config: Path, key: str) -> list[Path]:
    """The folders that the YAML file `config` names under `key`, at the top or in each entry of
    `packs`, as the binary reads `sgconfig.yml` and `xpd.yml`. A file the binary would reject
    names none, since its own time and size already show when it changes."""
    try:
        data = yaml.safe_load(config.read_text())
    except (OSError, UnicodeDecodeError, yaml.YAMLError):
        return []
    if not isinstance(data, dict):
        return []
    entries = [data, *(pack for pack in data.get("packs") or [] if isinstance(pack, dict))]
    named = [entry.get(key) for entry in entries]
    return [
        config.parent / name
        for names in named
        if isinstance(names, list)
        for name in names
        if isinstance(name, str)
    ]


def stamp(path: Path) -> tuple[str, int, int] | None:
    try:
        status = path.stat()
    except OSError:
        return None
    return (str(path), status.st_mtime_ns, status.st_size)


def inputs(root: Path) -> Inputs:
    """What the rules of `root` depend on: the project files in the root and each folder above
    it up to the nearest `sgconfig.yml`, then each YAML file under the rule and utility folders
    of that project."""
    files: list[Path] = []
    for folder in (root, *root.parents):
        files.extend(folder / name for name in (*CONFIG_NAMES, MANIFEST_NAME))
        config = next((folder / name for name in CONFIG_NAMES if (folder / name).is_file()), None)
        if config is None:
            continue
        manifest = folder / MANIFEST_NAME
        dirs = [
            *folders(config, "ruleDirs"),
            *folders(config, "utilDirs"),
            *folders(manifest, "ruleDirs"),
        ]
        files.extend(
            path
            for found in dirs
            for path in sorted(found.rglob("*"))
            if path.suffix in RULE_SUFFIXES and path.is_file()
        )
        break
    return tuple(found for path in dict.fromkeys(files) if (found := stamp(path)))


@dataclass
class Catalogs:
    """The catalog of each project root. It is read once, and again only when a file that
    `xpd rules` reads there has changed, since a run takes over a second."""

    xpd: Xpd
    project: Project
    fallback: Catalog
    read: dict[Path, tuple[Inputs, Catalog]] = field(default_factory=dict)

    async def load(self, ctx: Context) -> Catalog:
        root = await self.project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return self.fallback
        # The files are stamped before the run, so an edit during the run shows on the next call.
        stamps = inputs(root.path)
        cached = self.read.get(root.path)
        if cached is not None and cached[0] == stamps:
            return cached[1]
        catalog = Catalog(tuple(await self.xpd.run(RULES, root.path, [])))
        self.read[root.path] = (stamps, catalog)
        return catalog


def register(mcp: FastMCP, catalog: Catalog, xpd: Xpd, project: Project) -> None:
    catalogs = Catalogs(xpd, project, catalog)

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
        selected = await catalogs.load(ctx)
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
        return (await catalogs.load(ctx)).find(package, rule)

    @mcp.resource(
        RULES_URI,
        name="rules",
        mime_type="application/json",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def all_rules(ctx: Context) -> str:
        """Every rule and package check of xpd, without their documentation."""
        return dumps(summary(rule) for rule in (await catalogs.load(ctx)).rules)

    @mcp.resource(
        PACK_URI,
        name="pack-rules",
        mime_type="application/json",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def pack_rules(ctx: Context, package: str) -> str:
        """The rules of one pack, such as xpd://rules/pandas, without their documentation."""
        return dumps(summary(rule) for rule in (await catalogs.load(ctx)).of(package))

    @mcp.resource(
        RULE_URI,
        name="rule-doc",
        mime_type="text/markdown",
        tags={"rules"},
        annotations=FOR_AGENT,
    )
    async def rule_doc(ctx: Context, package: str, rule: str) -> str:
        """One rule's documentation, such as xpd://rules/pandas/chained-assignment."""
        found = (await catalogs.load(ctx)).find(package, rule)
        return found.doc or f"#### {found.rule}\n\n{found.message}\n"

    @mcp.completion
    async def complete(
        ref: PromptReference | ResourceTemplateReference,
        argument: CompletionArgument,
        context: CompletionContext | None,
    ) -> list[str] | None:
        if isinstance(ref, ResourceTemplateReference) and ref.uri in {PACK_URI, RULE_URI}:
            return (await catalogs.load(Context(mcp))).complete(argument, context)
        if isinstance(ref, PromptReference):
            return (await catalogs.load(Context(mcp))).complete(argument, context)
        return None


def dumps(rules: Iterable[RuleSummary]) -> str:
    return json.dumps([rule.model_dump(mode="json") for rule in rules])
