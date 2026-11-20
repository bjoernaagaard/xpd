"""Tool input schemas and the xpd arguments they produce."""

from typing import Literal

from fastmcp.exceptions import ToolError
from pydantic import BaseModel, Field

from xpd.mcp.models import FixSafety, Severity
from xpd.mcp.root import Root


class Query(BaseModel):
    pattern: str | None = Field(default=None, description="AST pattern with $VARIABLE captures.")
    kind: str | None = Field(default=None, description="Syntax kind instead of a pattern.")
    language: str | None = Field(
        default=None,
        description="The language of the pattern, such as python. Default: each file's extension.",
    )
    selector: str | None = Field(
        default=None, description="The kind of the part of the pattern that is the match."
    )
    rewrite: str | None = Field(default=None, description="Replacement template to preview.")

    def flags(self) -> list[str]:
        if (self.pattern is None) == (self.kind is None):
            msg = "Provide exactly one of pattern or kind."
            raise ToolError(msg)
        flags: list[str] = []
        for option, value in (
            ("--pattern", self.pattern),
            ("--kind", self.kind),
            ("--lang", self.language),
            ("--selector", self.selector),
            ("--rewrite", self.rewrite),
        ):
            if value is not None:
                flags.append(f"{option}={value}")
        return flags


class OutlineOptions(BaseModel):
    language: str | None = Field(
        default=None, description="Read only the files of this language, such as python."
    )
    items: Literal["auto", "all", "structure", "imports", "exports"] = Field(
        default="auto",
        description="Which top-level items to list. `auto` is `structure` for files and "
        "`exports` for folders.",
    )
    rules: str | None = Field(default=None, description="Project-relative outline rule file.")

    def flags(self, root: Root) -> list[str]:
        flags = [f"--items={self.items}"]
        if self.language is not None:
            flags.append(f"--lang={self.language}")
        if self.rules is not None:
            flags.append(f"--outline-rules={root.inside(self.rules)}")
        return flags


class NewItem(BaseModel):
    entity: Literal["project", "rule", "test", "util"] = Field(description="What to create.")
    name: str | None = Field(
        default=None,
        description="The rule's name or <pack>/<name>, the test's rule id, or the utility's name.",
    )
    directory: str = Field(
        default=".",
        description="The project folder, which holds sgconfig.yml, relative to the project root.",
    )
    language: str | None = Field(
        default=None, description="The language of a structural rule or a utility."
    )
    pack: str | None = Field(
        default=None,
        description="The pack the rule belongs to, or the pack a new project declares.",
    )
    pattern: str | None = Field(
        default=None,
        description="The code to find, with $X for any one node and $$$X for any number of them.",
    )
    message: str | None = Field(
        default=None, description="What the issue says. It may name the pattern's variables."
    )
    severity: Severity | None = Field(default=None, description="Default: warning.")
    note: str | None = Field(
        default=None, description="A longer explanation, for the rule's note and its Markdown."
    )
    rewrite: str | None = Field(
        default=None, description="The code that replaces a match, with the pattern's variables."
    )
    fix_safety: FixSafety | None = Field(
        default=None,
        description="Whether the fix keeps the code's behavior. Required with rewrite.",
    )
    since: str | None = Field(
        default=None, description="The first version of the pack's library the rule applies to."
    )
    until: str | None = Field(
        default=None,
        description="The first version of the pack's library the rule no longer applies to.",
    )
    holds: list[str] = Field(default_factory=list, description="Proofs as VAR={kind: call}.")
    unless_holds: list[str] = Field(
        default_factory=list,
        description="Drop a match when the variable holds a value of this shape, as "
        "VAR={kind: call}.",
    )
    unless_imported: str | None = Field(
        default=None,
        description="Silence the rule in a file that imports a module with a module-level "
        "statement of this shape.",
    )
    public_id: str | None = Field(
        default=None, description="The id that issues show, for one of several rule variants."
    )
    with_test: bool = Field(default=False, description="Also create an empty test case.")
    every_file: bool = Field(
        default=False,
        description="Run a pack this call adds on every Python file, not only on files that "
        "import its module.",
    )

    def flags(self, root: Root) -> list[str]:
        flags = [self.entity]
        flags.extend(("--yes", "--format=json", f"--directory={root.inside(self.directory)}"))
        for option, value in (
            ("--lang", self.language),
            ("--pack", self.pack),
            ("--pattern", self.pattern),
            ("--message", self.message),
            ("--severity", self.severity),
            ("--note", self.note),
            ("--fix", self.rewrite),
            ("--fix-safety", self.fix_safety),
            ("--since", self.since),
            ("--until", self.until),
            ("--unless-imported", self.unless_imported),
            ("--public-id", self.public_id),
        ):
            if value is not None:
                flags.append(f"{option}={value}")
        for option, values in (("--holds", self.holds), ("--unless-holds", self.unless_holds)):
            flags.extend(f"{option}={value}" for value in values)
        if self.with_test:
            flags.append("--with-test")
        if self.every_file:
            flags.append("--every-file")
        return flags


class CaseOptions(BaseModel):
    config: str | None = Field(
        default=None,
        description="A project file to read instead of the nearest sgconfig.yml, relative to the "
        "project root.",
    )
    test_dir: str | None = Field(
        default=None,
        description="Read the test cases in this folder instead of the testConfigs of "
        "sgconfig.yml.",
    )
    filter: str | None = Field(
        default=None, description="Test only the rules whose id matches this regular expression."
    )
    skip_snapshots: bool = Field(
        default=False, description="Check only the valid and invalid examples, and no snapshot."
    )
    update_snapshots: bool = Field(
        default=False, description="Accept every changed snapshot and write it."
    )

    def flags(self, root: Root) -> list[str]:
        if self.skip_snapshots and self.update_snapshots:
            msg = "Choose either skipping or updating snapshots."
            raise ToolError(msg)
        flags: list[str] = []
        for option, path in (("--config", self.config), ("--test-dir", self.test_dir)):
            if path is not None:
                flags.append(f"{option}={root.inside(path)}")
        if self.filter is not None:
            flags.append(f"--filter={self.filter}")
        if self.skip_snapshots:
            flags.append("--skip-snapshot-tests")
        if self.update_snapshots:
            flags.append("--update-all")
        return flags


class CheckOptions(BaseModel):
    rules: list[str] = Field(default_factory=list, description="Packages or public rule names.")
    severity: Severity | None = Field(
        default=None, description="Run only the rules of this severity and above."
    )
    config: str | None = Field(
        default=None,
        description="A project file to read instead of the nearest sgconfig.yml, relative to the "
        "project root.",
    )

    def flags(self, root: Root) -> list[str]:
        flags = [f"--rule={name}" for name in self.rules]
        if self.severity is not None:
            flags.append(f"--severity={self.severity}")
        if self.config is not None:
            flags.append(f"--config={root.inside(self.config)}")
        return flags
