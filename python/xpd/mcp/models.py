"""The JSON that the xpd binary prints, parsed at the boundary where the server reads it."""

from typing import Literal

from pydantic import BaseModel, ConfigDict, Field

Severity = Literal["error", "warning", "info", "hint"]
FixSafety = Literal["safe", "unsafe"]


class Output(BaseModel):
    """A shape the binary owns, so a field it adds later is kept, not refused."""

    model_config = ConfigDict(extra="allow", frozen=True)


class Issue(Output):
    """One finding of `xpd check`. A package warning has no line or column."""

    package: str = Field(description="Module name of the rule's pack, such as pandas.")
    rule: str = Field(description="Rule id, such as chained-assignment.")
    file: str
    line: int | None
    column: int | None
    severity: Severity
    message: str
    fix: FixSafety | None = Field(description="How safe the rule's fix is, or null for no fix.")


class CheckReport(Output):
    """What `xpd check` found, after the fixes it wrote."""

    fixed: int = Field(description="Number of fixes written to disk.")
    fixable: int = Field(description="Issues left that the fix tool fixes at the level asked for.")
    issues: list[Issue]


class Step(Output):
    """One step of a pipeline: a function that takes a frame and returns it with columns."""

    name: str
    line: int
    reads: list[str] = Field(description="Columns the step reads.")
    columns: list[str] = Field(description="Columns the step adds.")
    computed: bool = Field(description="True when the step also adds columns named at run time.")


class Link(Output):
    """A step that reads columns another step adds."""

    # `from` is a Python keyword, so the field has another name and keeps the binary's in JSON.
    model_config = ConfigDict(validate_by_name=True, serialize_by_alias=True)

    source: str = Field(alias="from")
    to: str
    columns: list[str]


class Pipeline(Output):
    """A function that runs steps in order, as frame.pipe(a).pipe(b)."""

    name: str
    file: str
    line: int
    steps: list[Step]
    links: list[Link]
    free: list[str] = Field(description="Steps that share no column with another step.")


class NodeInput(Output):
    name: str
    optional: bool


class Node(Output):
    """One Apache Hamilton node: a function output and the inputs it reads."""

    name: str
    file: str
    line: int
    function: str
    inputs: list[NodeInput]
    kind: str = Field(description="function, parameterized, extracted or saved.")
    config: str | None = Field(description="The @config.when decorator, when there is one.")


class GraphInput(Output):
    """A name that no function defines, which the caller passes to the Driver."""

    name: str
    optional: bool
    readers: list[str]


class Problem(Output):
    file: str
    line: int
    severity: Literal["error", "warning", "note"]
    message: str


class GraphReport(Output):
    """The dataflow that `xpd graph` read from step code and Hamilton modules."""

    pipelines: list[Pipeline]
    nodes: list[Node]
    inputs: list[GraphInput]
    unread: list[str] = Field(description="Nodes that no other node reads.")
    problems: list[Problem]


class ColumnProfile(Output):
    column: str
    dtype: str
    nulls: int
    unique: int | None = Field(description="Distinct values, or null for unhashable cells.")
    memory_bytes: int


class Profile(Output):
    """The columns of a data file as pandas reads it."""

    file: str
    pandas: str = Field(description="The pandas version that read the file.")
    rows: int
    memory_bytes: int
    columns: list[ColumnProfile]


class Equivalence(Output):
    """Whether two scripts leave an equal `result`."""

    equivalent: bool
    detail: str | None = Field(default=None, description="How the results differ.")


class RuleSummary(Output):
    """One rule or package check of `xpd rules`, without its documentation."""

    package: str = Field(description="Module name of the pack, such as pandas or sklearn.")
    rule: str
    severity: Severity
    message: str
    fix: FixSafety | None
    since: str | None = Field(description="The first library version the rule applies to.")
    url: str = Field(description="The rule's section of the documentation.")


class Rule(RuleSummary):
    """One rule with its section of the rules page, which has examples and notes."""

    doc: str | None = Field(description="The rule's documentation as Markdown.")


class RuleList(Output):
    rules: list[RuleSummary]


class SkillFile(Output):
    path: str = Field(description="Path below the skill folder, such as SKILL.md.")
    text: str


class SkillSource(Output):
    """One agent skill that ships with xpd, with every file of its folder."""

    name: str
    files: list[SkillFile]
