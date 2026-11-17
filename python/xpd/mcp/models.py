"""The JSON that the xpd binary prints, parsed at the boundary where the server reads it."""

from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, JsonValue, computed_field

Severity = Literal["error", "warning", "info", "hint"]
FixSafety = Literal["safe", "unsafe"]


class Output(BaseModel):
    """A shape the binary owns, so a field it adds later is kept, not refused."""

    model_config = ConfigDict(extra="allow", frozen=True, serialize_by_alias=True)


class CaseOutcome(Output):
    id: str
    source: str
    status: str
    actual: dict[str, object] | None = None


class CaseReport(Output):
    cases: list[CaseOutcome]
    failed: int


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


class SourcePosition(Output):
    line: int
    column: int


class ByteRange(Output):
    start: int
    end: int


class SourceRange(Output):
    byte_offset: ByteRange = Field(alias="byteOffset")
    start: SourcePosition
    end: SourcePosition


class CodeMatch(Output):
    text: str
    file: str
    language: str
    range: SourceRange
    replacement: str | None = None
    replacement_offsets: dict[str, int] | None = Field(default=None, alias="replacementOffsets")
    meta_variables: dict[str, JsonValue] = Field(default_factory=dict, alias="metaVariables")


class OutlineEntry(Output):
    name: str
    role: str
    symbol_type: str = Field(alias="symbolType")
    range: SourceRange
    signature: str | None = None
    ast_kind: str = Field(alias="astKind")
    is_import: bool = Field(default=False, alias="isImport")
    is_exported: bool = Field(default=False, alias="isExported")
    is_public: bool | None = Field(default=None, alias="isPublic")
    members: list["OutlineEntry"] = Field(default_factory=list)


class OutlineFile(Output):
    path: str
    language: str
    items: list[OutlineEntry]


class CreatedFiles(Output):
    created_files: list[str] = Field(alias="createdFiles")
    modified_files: list[str] = Field(default_factory=list, alias="modifiedFiles")


class Step(Output):
    """One step of a pipeline: a function that takes a frame and returns it with columns, or
    one `.assign(...)` call of a method chain."""

    name: str = Field(description="The function, or the call of a chain link: .assign(gross).")
    kind: str = Field(default="function", description="function, or link for a chain link.")
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
    """A function that runs steps in order, as frame.pipe(a).pipe(b), or a method chain."""

    name: str
    file: str
    line: int
    steps: list[Step]
    links: list[Link]
    source: list[str] = Field(
        default_factory=list,
        description="Columns the steps read that no earlier step adds: columns= of the read.",
    )
    source_complete: bool = Field(
        default=True, description="False when a step also reads columns named at run time."
    )
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


class ProfileFinding(Output):
    """What the columns of a file suggest about its shape or its dtypes."""

    finding: str = Field(description="values-as-column-names or category-candidate.")
    columns: list[str] = Field(description="The columns the finding is about.")
    message: str = Field(description="What was measured on the file, and what to write.")


class Profile(Output):
    """The columns of a data file as pandas reads it."""

    file: str
    pandas: str = Field(description="The pandas version that read the file.")
    rows: int
    memory_bytes: int
    columns: list[ColumnProfile]
    findings: list[ProfileFinding] = Field(
        default_factory=list,
        description="Column names that hold values, and text columns that suit a category.",
    )


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
    url: str | None = Field(description="The rule documentation URL, when supplied.")


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


# The stages of a pipeline run, from raw rows to a saved model, and what each hands to the next.
# The training script writes the Contract, and the other shapes are results of xpd's own tools.
#
#   raw data          RawData       the file, the target column and the row count
#   profile           Profile       each column's dtype and nulls, for the choice of steps
#   split             Split         the seed and the rows held out, which no later stage fits on
#   preprocessing     InputColumn   the raw columns that the one Pipeline takes
#                     FittedStep    each step of that Pipeline, in order
#   model             FittedStep    the last step
#   validation        Validation    scores on folds of the training rows, one on the held out rows
#   proof             Proof         what check, graph, refit and equiv found
#   persisted model   SavedModel    the file, the versions that wrote it and the code it imports
#   verdict           GateReport    one Gate per requirement
Stage = Literal[
    "discovery",
    "raw data",
    "profile",
    "split",
    "preprocessing",
    "model",
    "validation",
    "proof",
    "persisted model",
]
Evidence = Literal["profile", "check", "graph", "equiv", "contract", "api", "source", "refit"]


class Written(BaseModel):
    """A shape the training script writes. A section xpd does not know, such as one that a rule
    pack asks for, is kept."""

    model_config = ConfigDict(extra="allow", frozen=True)


PARAMETERS_WRITTEN = (
    "The parameter list as the source writes it: each parameter with its type annotation and "
    "default, and a * or / that stands alone between parameters. A stub file writes ... for a "
    "default, which says only that the parameter is optional."
)


class ApiMember(Output):
    name: str
    parameters: list[str]
    parameters_written: list[str] = Field(default_factory=list, description=PARAMETERS_WRITTEN)
    summary: str
    property: bool = Field(default=False, description="The member is read and not called.")


class DiscoveryEntry(Output):
    kind: str
    package: str
    name: str
    callable: Literal["class", "function", "method", "property"] | None
    parameters: list[str]
    parameters_written: list[str] = Field(default_factory=list, description=PARAMETERS_WRITTEN)
    summary: str
    members: list[ApiMember]
    rules: list[str]

    @computed_field
    @property
    def signature(self) -> str | None:
        """Return indexed parameter names. `parameters_written` has the defaults and kinds."""
        if self.kind != "symbol" or self.callable in {None, "property"}:
            return None
        return f"{self.name}({', '.join(self.parameters)})"


class DiscoveryMatches(Output):
    """Candidates ranked separately within each kind, including entries read through api."""

    symbols: list[DiscoveryEntry] = Field(default_factory=list)
    rules: list[DiscoveryEntry] = Field(default_factory=list)
    recipes: list[DiscoveryEntry] = Field(default_factory=list)

    def entries(self) -> list[DiscoveryEntry]:
        return [*self.symbols, *self.rules, *self.recipes]


class DiscoveryQuestion(Written):
    task: str = Field(min_length=1)


class DiscoveryChoice(Written):
    name: str = Field(description="The selected candidate's exact name.", min_length=1)
    reason: str = Field(min_length=1)
    symbols: list[str] = Field(
        min_length=1,
        description="Indexed callable symbols the code must call, including a recipe's symbols.",
    )


class Discovery(Written):
    question: DiscoveryQuestion
    candidates: DiscoveryMatches
    choice: DiscoveryChoice

    def decision(self) -> tuple[DiscoveryQuestion, list[tuple[str, str]], DiscoveryChoice]:
        """The question, the kind and name of each candidate in order, and the choice. The text
        of a candidate is left out, because xpd rewrites a rule's text and an index entry between
        releases, and the decision is the same."""
        return (
            self.question,
            [(entry.kind, entry.name) for entry in self.candidates.entries()],
            self.choice,
        )


class RawData(Written):
    file: str = Field(description="The data file, relative to the project root.")
    target: str = Field(description="The column the model predicts.")
    rows: int = Field(description="Rows in the file, before the split.")


class Split(Written):
    """How the rows were divided once, before any step was fitted."""

    seed: int = Field(description="The random_state of the split.")
    test_size: float = Field(gt=0, lt=1, description="The share of the rows held out.")
    stratified: bool = Field(description="True when the split keeps the target's proportions.")
    train_rows: int
    test_rows: int


class InputColumn(Written):
    """One column the saved model takes, as pandas reads it from the data file."""

    column: str
    dtype: str


class FittedStep(Written):
    """One step of the Pipeline. The last one is the model."""

    name: str
    estimator: str = Field(description="The class name, such as ColumnTransformer.")


class Splitter(Written):
    model_config = ConfigDict(validate_by_name=True, serialize_by_alias=True)

    class_name: str = Field(alias="class", min_length=1)
    args: dict[str, JsonValue]


class Refit(Output):
    rows: int
    train_rows: int
    test_rows: int
    output: Literal["predict_proba", "decision_function", "predict"]
    largest_gap: float | None
    model_matches: bool
    contract_test_score: float
    refit_test_score: float
    contract_cv_scores: list[float]
    refit_cv_scores: list[float]
    test_score_matches: bool
    cv_scores_match: bool
    counts_match: bool
    passed: bool


class Validation(Written):
    splitter: Splitter
    scoring: str = Field(description="The scorer name, such as accuracy.")
    folds: int
    cv_scores: list[float] = Field(description="One score per fold.")
    test_score: float = Field(description="The score on the held out rows.")


class SavedModel(Written):
    file: str = Field(description="The saved model, relative to the project root.")
    type: str = Field(description="The class of the saved object, with its module.")
    versions: dict[str, str] = Field(description="Python and each library that wrote the file.")
    code: list[str] = Field(
        default_factory=list,
        description="The project files the model imports when it loads, relative to the "
        "project root. A Pipeline saves a function such as the one in a FunctionTransformer "
        "by its module and name, so the model loads only where that module can be imported.",
    )


class Contract(Written):
    """What a training script saves beside its model: all that a later run needs to load the
    model, check its input and repeat the split."""

    discovery: Discovery | None = None
    data: RawData
    split: Split
    inputs: list[InputColumn]
    steps: list[FittedStep]
    validation: Validation
    model: SavedModel


class Gate(BaseModel):
    """One requirement of a pipeline run, and whether the evidence meets it."""

    model_config = ConfigDict(frozen=True)

    name: str
    stage: Stage
    evidence: list[Evidence] = Field(description="The tools whose results decide the gate.")
    passed: bool
    detail: str = Field(description="What was found, or each reason the gate failed.")


class GateReport(BaseModel):
    model_config = ConfigDict(frozen=True)

    passed: bool = Field(description="True when every gate passed.")
    failed: list[str] = Field(description="The names of the gates that failed.")
    gates: list[Gate]


class DiscoveryProof(BaseModel):
    model_config = ConfigDict(frozen=True)

    discovery: Discovery
    paths: list[str]
    gate: Gate


class Proof(BaseModel):
    """What xpd's tools found about the scripts of a pipeline run."""

    model_config = ConfigDict(frozen=True)

    discovery: DiscoveryProof | None = None
    lint: CheckReport = Field(description="`check` on the training, prediction and step files.")
    dataflow: GraphReport = Field(description="`graph` on the file with the feature steps.")
    reproducible: Equivalence = Field(description="`equiv` of the training script with itself.")
    refit: Refit = Field(description="`refit` on the saved contract.")
    served: Equivalence = Field(
        description="`equiv` of the training script with the script that loads the saved model."
    )
