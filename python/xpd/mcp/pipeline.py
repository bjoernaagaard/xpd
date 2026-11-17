"""The tools that judge a pipeline run, from raw rows to a saved model: `contract` reads what the
training script saved, and `gates` turns the results of the other tools into one verdict.

These tools do not run Python or load a model. Each gate reads results that `profile`, `check`,
`graph`, `refit` and `equiv` returned for the same run, which a code mode script hands on."""

from collections.abc import Sequence

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult
from pydantic import ValidationError

from xpd.mcp.catalog import Catalog
from xpd.mcp.models import (
    CheckReport,
    Contract,
    Equivalence,
    Evidence,
    Gate,
    GateReport,
    GraphReport,
    Profile,
    Proof,
    Refit,
    Stage,
)
from xpd.mcp.root import Project
from xpd.mcp.tools import READS

# The rules of each pack that report a split, an estimator, an encoder, a bootstrap, a simulation
# or an optimizer run that draws without a seed. A pack without such a rule is not listed.
SEED_RULES = {
    "sklearn": ("missing-random-state", "random-state-without-shuffle"),
    "sklego": ("unseeded-random-estimator",),
    "mapie": ("unseeded-resampling",),
    "category_encoders": ("encoder-noise-without-seed",),
    "arch": ("arch-unseeded-draws",),
    "pymoo": ("unseeded-run", "numpy-seed-ignored"),
}
PIPELINE = "sklearn.pipeline.Pipeline"
# What a later run needs to rebuild the environment that wrote the model.
VERSIONS = ("python", "scikit-learn", "pandas")
MIN_FOLDS = 2
# The libraries that SEED_RULES covers, as the detail of `split-is-seeded` names them.
SEED_LIBRARIES = "scikit-learn, scikit-lego, MAPIE, Category Encoders, arch or pymoo"


def gate(
    name: str, stage: Stage, evidence: Sequence[Evidence], failures: Sequence[str], found: str
) -> Gate:
    return Gate(
        name=name,
        stage=stage,
        evidence=list(evidence),
        passed=not failures,
        detail=" ".join(failures) or found,
    )


def data_is_profiled(profile: Profile, contract: Contract) -> Gate:
    data = contract.data
    failures = []
    if data.file != profile.file:
        failures.append(f"The contract names {data.file}, and the profile is of {profile.file}.")
    if data.rows != profile.rows:
        failures.append(f"The contract counts {data.rows} rows, and the file has {profile.rows}.")
    target = next((column for column in profile.columns if column.column == data.target), None)
    if target is None:
        failures.append(f"{profile.file} has no column {data.target}.")
    elif target.nulls:
        failures.append(f"The target {data.target} has {target.nulls} nulls.")
    found = f"{data.file} has {data.rows} rows, and the target {data.target} has no nulls."
    return gate("data-is-profiled", "raw data", ["profile", "contract"], failures, found)


def split_covers_every_row(profile: Profile, contract: Contract) -> Gate:
    split = contract.split
    failures = []
    if min(split.train_rows, split.test_rows) < 1:
        failures.append("The split leaves no rows on one side.")
    if split.train_rows + split.test_rows != profile.rows:
        failures.append(
            f"{split.train_rows} training rows and {split.test_rows} held out rows are not the "
            f"{profile.rows} rows of the file."
        )
    found = (
        f"The contract counts {split.train_rows} training rows and {split.test_rows} held out rows."
    )
    return gate("split-covers-every-row", "split", ["profile", "contract"], failures, found)


def split_is_seeded(lint: CheckReport, contract: Contract) -> Gate:
    failures = [
        f"{issue.file}:{issue.line} {issue.package}/{issue.rule}."
        for issue in lint.issues
        if issue.rule in SEED_RULES.get(issue.package, ())
    ]
    found = (
        f"The contract records the seed {contract.split.seed}, and check reports no unseeded "
        f"{SEED_LIBRARIES} draw."
    )
    return gate("split-is-seeded", "split", ["check", "contract"], failures, found)


def check_reports_no_error(lint: CheckReport) -> Gate:
    """Every error of every pack counts, and not a list of rules. The leaks among them are
    `fit-before-split`, `fit-on-test` and `tune-on-test` of sklearn, `calibrate-on-training-rows`
    and `coverage-on-seen-rows` of mapie, and `encode-before-split` and
    `transform-with-test-target` of category_encoders. sklego has no rule about a leak, and its
    errors fail the gate as wrong results.

    The gate does not show that a run has no leak. A rule reads the code and knows the held out
    rows by names such as `X_test`, so it misses a leak under another name and a leak that no
    rule describes."""
    failures = [
        f"{issue.file}:{issue.line} {issue.package}/{issue.rule}."
        for issue in lint.issues
        if issue.severity == "error"
    ]
    return gate("check-reports-no-error", "proof", ["check"], failures, "check reports no error.")


def inputs_are_file_columns(profile: Profile, contract: Contract) -> Gate:
    """Compares names and dtypes. It does not compare values, so a column that the training
    script overwrites before the split passes."""
    raw = {column.column: column.dtype for column in profile.columns}
    failures = []
    for column in contract.inputs:
        if column.column == contract.data.target:
            failures.append(f"The target {column.column} is an input.")
        elif column.column not in raw:
            failures.append(
                f"The input {column.column} is no column of {profile.file}, so code outside the "
                "Pipeline made it."
            )
        elif raw[column.column] != column.dtype:
            failures.append(
                f"The input {column.column} is {column.dtype}, and the file holds "
                f"{raw[column.column]}."
            )
    if not contract.inputs:
        failures.append("The contract lists no input column.")
    found = (
        f"The {len(contract.inputs)} inputs of the contract have the names and dtypes of columns "
        f"of {profile.file}."
    )
    return gate(
        "inputs-are-file-columns", "preprocessing", ["profile", "contract"], failures, found
    )


def one_pipeline(contract: Contract) -> Gate:
    failures = []
    if contract.model.type != PIPELINE:
        failures.append(
            f"The contract names the saved object as {contract.model.type}, not {PIPELINE}."
        )
    if not contract.steps:
        failures.append("The contract lists no step.")
    found = " then ".join(step.estimator for step in contract.steps)
    return gate(
        "one-pipeline",
        "model",
        ["contract"],
        failures,
        f"The contract lists one Pipeline of {found}.",
    )


def dataflow_connects(dataflow: GraphReport, contract: Contract) -> Gate:
    """A step may read the model's input columns and the columns of the steps before it. A read
    of anything else is a column that is missing when the model predicts, or the target."""
    failures = [
        f"{problem.file}:{problem.line} {problem.message}"
        for problem in dataflow.problems
        if problem.severity in {"error", "warning"}
    ]
    inputs = {column.column for column in contract.inputs}
    steps = 0
    for pipeline in dataflow.pipelines:
        known = set(inputs)
        for step in pipeline.steps:
            steps += 1
            failures.extend(
                f"{pipeline.file}:{step.line} {step.name}() reads {column}, which is no input "
                "column and which no earlier step adds."
                for column in step.reads
                if column not in known
            )
            known.update(step.columns)
    found = f"{steps} feature steps read only input columns and columns of earlier steps."
    return gate("dataflow-connects", "preprocessing", ["graph", "contract"], failures, found)


def cross_validated(contract: Contract) -> Gate:
    """Check that the contract records one score per fold."""
    validation = contract.validation
    failures = []
    if validation.folds < MIN_FOLDS:
        failures.append(f"{validation.folds} folds is no cross-validation.")
    if len(validation.cv_scores) != validation.folds:
        failures.append(
            f"The contract has {len(validation.cv_scores)} scores for {validation.folds} folds."
        )
    found = (
        f"The contract records {validation.scoring} {min(validation.cv_scores, default=0)} to "
        f"{max(validation.cv_scores, default=0)} on {validation.folds} folds, and "
        f"{validation.test_score} as the held out score."
    )
    return gate("cross-validated", "validation", ["contract"], failures, found)


def equivalent(name: str, stage: Stage, result: Equivalence, found: str) -> Gate:
    failures = [] if result.equivalent else [result.detail or "The results differ."]
    return gate(name, stage, ["equiv"], failures, found)


def versions_recorded(profile: Profile, contract: Contract) -> Gate:
    versions = contract.model.versions
    failures = [
        f"The contract records no version of {name}." for name in VERSIONS if not versions.get(name)
    ]
    if versions.get("pandas") not in {None, profile.pandas}:
        failures.append(
            f"The model was written with pandas {versions['pandas']}, and the project's Python "
            f"has {profile.pandas}."
        )
    found = ", ".join(f"{name} {version}" for name, version in versions.items())
    return gate(
        "versions-recorded",
        "persisted model",
        ["profile", "contract"],
        failures,
        f"The contract records {found}.",
    )


def refit_matches_model(result: Refit) -> Gate:
    failures = []
    if not result.model_matches:
        failures.append(
            f"The raw training refit differs on {result.output}. "
            f"The largest gap is {result.largest_gap}."
        )
    return gate(
        "refit-matches-model",
        "proof",
        ["refit"],
        failures,
        f"The raw training refit matches the saved model on {result.output}.",
    )


def refit_matches_scores(contract: Contract, result: Refit) -> Gate:
    failures = []
    if contract.validation.test_score != result.refit_test_score:
        failures.append(
            f"The contract held out score is {contract.validation.test_score}, "
            f"and the refit gives {result.refit_test_score}."
        )
    if contract.validation.cv_scores != result.refit_cv_scores:
        failures.append(
            f"The contract fold scores are {contract.validation.cv_scores}, "
            f"and the refit gives {result.refit_cv_scores}."
        )
    if (
        result.rows != contract.data.rows
        or result.train_rows != contract.split.train_rows
        or result.test_rows != contract.split.test_rows
        or len(result.refit_cv_scores) != contract.validation.folds
    ):
        failures.append("The refit row or fold counts differ.")
    return gate(
        "refit-matches-scores",
        "validation",
        ["refit", "contract"],
        failures,
        "The contract scores and row and fold counts match the raw training refit.",
    )


def judge(profile: Profile, contract: Contract, proof: Proof) -> GateReport:
    gates = [
        data_is_profiled(profile, contract),
        split_covers_every_row(profile, contract),
        split_is_seeded(proof.lint, contract),
        inputs_are_file_columns(profile, contract),
        dataflow_connects(proof.dataflow, contract),
        one_pipeline(contract),
        check_reports_no_error(proof.lint),
        cross_validated(contract),
        equivalent(
            "reproducible",
            "proof",
            proof.reproducible,
            "Two runs of the training script, each in its own Python process, leave an equal "
            "`result`.",
        ),
        refit_matches_model(proof.refit),
        refit_matches_scores(contract, proof.refit),
        equivalent(
            "saved-model-predicts",
            "persisted model",
            proof.served,
            "The prediction script, in its own Python process, leaves the `result` that the "
            "training script left.",
        ),
        versions_recorded(profile, contract),
    ]
    if contract.discovery is not None or proof.discovery is not None:
        failures = []
        if proof.discovery is None:
            failures.append("No discovery_gate result was supplied.")
        else:
            gates.append(proof.discovery.gate)
            recorded = contract.discovery.decision() if contract.discovery else None
            if recorded != proof.discovery.discovery.decision():
                failures.append("The contract and proof record different discovery choices.")
        gates.append(
            gate(
                "discovery-is-recorded",
                "discovery",
                ["contract"],
                failures,
                "The contract records the discovery question, candidates and reason of the proof.",
            )
        )
    failed = [gate.name for gate in gates if not gate.passed]
    return GateReport(passed=not failed, failed=failed, gates=gates)


def register(mcp: FastMCP, project: Project, rules: Catalog) -> None:
    # A rule that the binary renamed would never match, and its gate would pass every run.
    for package, names in SEED_RULES.items():
        for rule in names:
            rules.find(package, rule)

    @mcp.tool(tags={"pipeline"}, annotations=READS)
    async def contract(ctx: Context, file: str) -> Contract | InputRequiredResult:
        """Read the contract that a training script saved beside its model: the data file and
        target, the split, the input columns, the steps of the Pipeline, the validation scores,
        and the model file with the versions that wrote it and the project files it imports
        when it loads. Changes no file.

        It fails when the file does not have that shape, or when the data file, the model file
        or a code file it names is not in the project. Pass the result to `gates`.

        Args:
            file: The contract, a JSON file inside the project root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        path = root.path / root.inside(file)
        try:
            read = Contract.model_validate_json(path.read_bytes())
        except OSError as error:
            msg = f"Cannot read {file}. {error}"
            raise ToolError(msg) from error
        except ValidationError as error:
            fields = "; ".join(
                " ".join([".".join(map(str, problem["loc"])), problem["msg"]]).strip()
                for problem in error.errors()
            )
            msg = f"{file} is not a contract. {fields}"
            raise ToolError(msg) from error
        # The paths are compared with those of the other tools, which are relative to the root.
        data, model = root.inside(read.data.file), root.inside(read.model.file)
        code = [root.inside(raw) for raw in read.model.code]
        for raw in (data, model, *code):
            if not (root.path / raw).is_file():
                msg = f"{file} names {raw}, which is no file in the project."
                raise ToolError(msg)
        return read.model_copy(
            update={
                "data": read.data.model_copy(update={"file": data}),
                "model": read.model.model_copy(update={"file": model, "code": code}),
            }
        )

    @mcp.tool(tags={"pipeline"}, annotations=READS)
    def gates(profile: Profile, contract: Contract, proof: Proof) -> GateReport:
        """Judge a pipeline run from raw rows to a saved model. Each gate is one requirement,
        with the tools whose results decide it. The run is done when `passed` is true. Changes
        no file and runs nothing, so pass results of this run.

        The gates: the data is the profiled file and its target has no nulls, the split
        covers every row, `check` reports no draw without a seed in scikit-learn, scikit-lego,
        MAPIE, Category Encoders, arch or pymoo code, the inputs have the names and dtypes of
        columns of the file, one Pipeline holds every step, the feature steps connect, `check`
        reports no error, the contract has a score for each fold, a raw training refit matches
        the saved model and the recorded scores, two training runs leave an equal `result`,
        the prediction script leaves that `result` too,
        and the versions are recorded. When discovery is supplied, include its indexed-api-calls
        result and require the contract's discovery record to match proof.discovery.
        Without discovery in either, thirteen gates apply.

        A passed report does not judge choices made across separate runs or raw columns
        that stand in for the target. Pass fresh tool results from this project.

        Args:
            profile: The result of `profile` on the data file.
            contract: The result of `contract` on the file the training script saved.
            proof: `lint` is the result of `check` on the training, prediction and step files.
                `dataflow` is the result of `graph` on the file with the feature steps.
                `reproducible` compares two runs of the training script with `equiv`.
                `refit` is the result of refitting the saved contract. `served` is the result
                of `equiv` with the training script as old and the loading script as new.
        """
        return judge(profile, contract, proof)
