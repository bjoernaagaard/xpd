"""The tools that judge a pipeline run, from raw rows to a saved model: `contract` reads what the
training script saved, and `gates` runs the durable `xpd gates` workflow on the binary."""

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult
from pydantic import ValidationError

from xpd.mcp.base import READS, RUNS_CODE, rooted
from xpd.mcp.binary import GATES_RETRY, GATES_START
from xpd.mcp.models import Contract, DiscoveryProof, GateReport, Scripts
from xpd.mcp.names import ToolName
from xpd.mcp.root import Root
from xpd.mcp.tools import Analysis


def read_contract(root: Root, file: str) -> Contract:
    """The contract in `file`, with its paths relative to the root, as the commands name them."""
    path = root.path / root.inside(file)
    try:
        read = Contract.model_validate_json(path.read_bytes())
    except OSError as error:
        msg = f"Cannot read {file}. {error}"
        raise ToolError(msg) from error
    except ValidationError as error:
        problems = []
        for problem in error.errors():
            place = ".".join(map(str, problem["loc"]))
            problems.append(f"{place} {problem['msg']}".strip())
        fields = "; ".join(problems)
        msg = f"{file} is not a contract. {fields}"
        raise ToolError(msg) from error
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


def register(mcp: FastMCP, analysis: Analysis) -> None:
    project = analysis.project
    xpd = analysis.xpd

    @mcp.tool(name=ToolName.CONTRACT, tags={"pipeline"}, annotations=READS)
    @rooted
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
        return read_contract(await project.required_root(ctx), file)

    @mcp.tool(name=ToolName.GATES, tags={"pipeline"}, annotations=RUNS_CODE, task=True)
    @rooted
    async def gates(
        ctx: Context,
        contract: str,
        scripts: Scripts,
        discovery: DiscoveryProof | None = None,
        run: str | None = None,
    ) -> GateReport | InputRequiredResult:
        """Run the durable `xpd gates` workflow on a pipeline run, from raw rows to a saved model,
        and judge it. Each gate is one requirement, with the commands whose results decide it. The
        run is done when `passed` is true.

        Pass `run` from an earlier failed response to retry the same durable run and reuse cached
        results for unchanged inputs (`profile`, `check`, `graph`, `refit`, and `equiv`). It runs
        the scripts with the project's Python, so run it on code you trust.

        Args:
            contract: The contract that the training script saved, a JSON file inside the
                project root.
            scripts: The training script, the prediction script and the file with the
                feature steps.
            discovery: The result of `discovery_gate` for this run, when the choice of a
                library was recorded.
            run: Optional run ID from a previous `gates` call to retry with cached evidence.
        """
        root = await project.required_root(ctx)
        read_contract(root, contract)
        stdin = discovery.model_dump_json().encode() if discovery is not None else None
        discovery_flags = ("--discovery", "-") if discovery is not None else ()
        if run is not None:
            await ctx.report_progress(0, 1, f"Retrying gates run {run}")
            report = await xpd.run(
                GATES_RETRY,
                root.path,
                [run],
                *discovery_flags,
                project_root=root.path,
                stdin=stdin,
            )
            await ctx.report_progress(1, 1, f"Gates run {run} finished")
            return report

        flags = (
            "--contract",
            root.inside(contract),
            "--train",
            root.inside(scripts.train),
            "--predict",
            root.inside(scripts.predict),
            "--features",
            root.inside(scripts.features),
            *discovery_flags,
        )
        await ctx.report_progress(0, 1, "Running durable pipeline gates")
        report = await xpd.run(
            GATES_START,
            root.path,
            (),
            *flags,
            project_root=root.path,
            stdin=stdin,
        )
        await ctx.report_progress(1, 1, f"Gates run {report.run or ''} finished".strip())
        return report
