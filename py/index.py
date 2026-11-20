"""Write xpd entries from Griffe's static API model.

Griffe resolves the public names and reads the documentation. Each entry names the definitions
whose parameters it lists, by file and line, and xpd reads each parameter there as the source
writes it.
"""

import ast
import importlib.metadata
import json
import logging
import os
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import TypedDict

GRIFFE_VERSION = "2.4.1"


def require_griffe() -> None:
    try:
        installed = importlib.metadata.version("griffe")
    except importlib.metadata.PackageNotFoundError:
        installed = None
    if installed != GRIFFE_VERSION:
        source, package, version = sys.argv[1:]
        sys.exit(
            f"Index building needs griffe=={GRIFFE_VERSION}. Run "
            f"uv run --with griffe=={GRIFFE_VERSION} xpd index "
            f"{source} --package {package} --version {version}."
        )


if __name__ == "__main__":
    require_griffe()
import griffe  # noqa: E402  Name the exact install command before importing the build dependency.

Unresolved = (griffe.AliasResolutionError, griffe.CyclicAliasError)


class Source(TypedDict):
    """A function that the source defines, where its definition starts.

    `receiver` says that Python binds the first parameter to the object or the class, so a
    call does not pass it.
    """

    name: str
    file: str
    line: int
    receiver: bool


class Parameter(TypedDict):
    """One part of a parameter list: a parameter, or the `*` before the keyword-only ones."""

    name: str | None
    written: str


class Built(TypedDict):
    """A function that Griffe builds, such as the `__init__` of a dataclass, with its parameters."""

    name: str
    parameters: list[Parameter]
    receiver: bool


Definition = Source | Built


class Member(TypedDict):
    name: str
    definitions: list[Definition]
    summary: str
    property: bool


class Entry(TypedDict):
    kind: str
    package: str
    name: str
    callable: str
    definitions: list[Definition]
    summary: str
    keywords: list[str]
    members: list[Member]
    rules: list[str]


class Callables(griffe.Extension):
    """Keep the callables that Griffe's reading drops. Each case names one that is lost."""

    def __init__(self) -> None:
        self.source_files: dict[griffe.Function, str] = {}
        self.merged: set[griffe.Function] = set()
        self.functions: list[griffe.Function] = []
        self.assignments: list[griffe.Attribute] = []
        self.imports: dict[str, tuple[str, bool]] = {}
        self.imported: dict[str, griffe.Alias] = {}
        self.computed_exports: set[str] = set()
        self.stub_exports: set[str] = set()

    def definitions(self, obj: griffe.Object | griffe.Alias) -> list[Definition]:
        """The definitions whose parameters are the parameters of `obj`.

        A class takes the parameters of its constructor. A function that only stubs declare, as
        overloads, takes the parameters of every overload.
        """
        if obj.is_class:
            obj = constructor(obj)
            if obj is None:
                return []
        function = obj.final_target if obj.is_alias else obj
        overloads = function.overloads if function in self.merged else [function]
        return [self.definition(f) for f in overloads]

    def definition(self, function: griffe.Function) -> Definition:
        """Where xpd reads the parameters of `function`.

        A function of a class takes its object or its class first, unless it is a static method.
        `pandas.compat.set_function_name` is a function of a module with a parameter named `cls`,
        which a call passes.
        """
        receiver = function.parent.is_class and "staticmethod" not in function.labels
        file = self.source_files.get(function)
        if file is None:
            return Built(
                name=function.name, parameters=built_parameters(function), receiver=receiver
            )
        return Source(name=function.name, file=file, line=function.lineno, receiver=receiver)

    def on_alias_instance(self, *, alias: griffe.Alias, **_kwargs: object) -> None:
        """Griffe's dataclass reading raises on a class member imported from another library.

        `hamilton.plugins.polars_post_1_0_0_extensions.PolarsSpreadsheetWriter` imports a
        polars type in its class body.
        """
        parent = alias.parent
        if parent.is_class and alias.target_path.split(".")[0] != parent.package.name:
            parent.del_member(alias.name)
        elif parent.is_module and not alias.wildcard:
            self.imported[alias.path] = alias

    def on_module_members(self, *, mod: griffe.Module, **_kwargs: object) -> None:
        self.add_overloads(mod)
        stub = mod.filepath.suffix == ".pyi"
        for name, member in mod.members.items():
            if not member.is_alias:
                continue
            # `from .lobpcg import *` can bind `lobpcg` over the submodule it reads.
            target = member.wildcard or member.target_path
            leaf = target.rpartition(".")[2] if member.wildcard else name
            bound = f"{target}.{leaf}" if member.wildcard else target
            if stub or f"{mod.path}.{leaf}" not in self.imports:
                self.imports[f"{mod.path}.{leaf}"] = (bound, bool(member.wildcard))

    def on_class_members(self, *, cls: griffe.Class, **_kwargs: object) -> None:
        self.add_overloads(cls)

    def add_overloads(self, obj: griffe.Module | griffe.Class) -> None:
        """`numpy.zeros` exists only as overloads in `_core/multiarray.pyi`."""
        if obj.filepath.suffix != ".pyi":
            return
        for name, overloads in obj.overloads.items():
            if name not in obj.members and overloads:
                first = overloads[0]
                merged = {p.name: p for overload in overloads for p in overload.parameters}
                function = griffe.Function(
                    name,
                    parameters=griffe.Parameters(*merged.values()),
                    returns=first.returns,
                    lineno=first.lineno,
                    endlineno=first.endlineno,
                    docstring=first.docstring,
                )
                function.overloads = overloads
                self.merged.add(function)
                obj.set_member(name, function)

    def on_attribute_instance(
        self, *, node: ast.AST, attr: griffe.Attribute, **_kwargs: object
    ) -> None:
        if not attr.parent.is_module:
            return
        if attr.name == "__all__":
            try:
                griffe.get__all__(node, attr.parent)
            except Exception:  # noqa: BLE001  Griffe raises a different error for each form.
                self.computed_exports.add(attr.parent.path)
            else:
                if attr.parent.filepath.suffix == ".pyi":
                    self.stub_exports.add(attr.parent.path)
        elif str(attr.annotation or "TypeAlias").endswith("TypeAlias") and isinstance(
            attr.value, (griffe.ExprName, griffe.ExprAttribute, griffe.ExprSubscript)
        ):
            self.assignments.append(attr)

    def on_function_instance(self, *, func: griffe.Function, **_kwargs: object) -> None:
        """Keep the import when a module also defines a stand-in that takes anything.

        `plotly.figure_factory.create_hexbin_map` is imported when pandas is installed, and
        is otherwise `def create_hexbin_map(*args, **kwargs)`, which raises.

        Record the file being read, because Griffe moves a function of a stub into the module
        that the stub describes.
        """
        self.source_files[func] = str(func.module.filepath)
        if not func.parent.is_module:
            return
        imported = self.imported.get(func.path)
        if imported is not None and [p.name for p in func.parameters] == ["args", "kwargs"]:
            func.parent.set_member(func.name, imported)
        elif func.parent.filepath.suffix == ".py":
            self.functions.append(func)

    def on_package(self, *, pkg: griffe.Module, **_kwargs: object) -> None:
        for function in self.functions:
            self.restore_function(pkg, function)
        for attribute in self.assignments:
            self.follow_assignment(pkg, attribute)

    @staticmethod
    def module(pkg: griffe.Module, path: str) -> griffe.Module | None:
        """The module at `path` after Griffe merged the stubs, without following an import."""
        module = pkg
        for part in path.split(".")[1:]:
            module = module.members.get(part)
            if module is None or module.is_alias or not module.is_module:
                return None
        return module

    def restore_function(self, pkg: griffe.Module, function: griffe.Function) -> None:
        """`numpy.polynomial.hermite.hermadd` is a function that its stub types as an attribute."""
        module = self.module(pkg, function.parent.path)
        current = module.members.get(function.name) if module else None
        if current is not None and not current.is_alias and current.is_attribute:
            module.set_member(function.name, function)

    def follow_assignment(self, pkg: griffe.Module, assigned: griffe.Attribute) -> None:
        """`numpy.concat` is `concat = numeric.concatenate` in `_core/__init__.py`.

        `pandas.io.pytables.Term` is `Term: TypeAlias = PyTablesExpr`.
        `hamilton.function_modifiers.configuration` is assigned over a submodule of that name.
        `numpy.int64` is `int64 = signedinteger[_64Bit]` in `__init__.pyi`.
        """
        module = self.module(pkg, assigned.parent.path)
        current = module.members.get(assigned.name) if module else None
        value = assigned.value
        if isinstance(value, griffe.ExprSubscript):
            value = value.left
        if not isinstance(value, (griffe.ExprName, griffe.ExprAttribute)):
            return
        target = value.canonical_path
        if current is None or current.is_alias or target.split(".")[0] != pkg.name:
            return
        if current.is_module:
            self.imports[current.path] = (target, False)
            return
        if not current.is_attribute or target == current.path:
            return
        alias = griffe.Alias(assigned.name, target, parent=module)
        try:
            is_callable = alias.final_target.is_class or alias.final_target.is_function
        except Unresolved:
            return
        if is_callable:
            module.set_member(assigned.name, alias)


class Loader(griffe.GriffeLoader):
    """Read an `__all__` that Griffe cannot evaluate as unknown. Griffe reads it as empty.

    `scipy.signal.butter` reaches `scipy.signal` through `from ._signal_api import *`, and
    `_signal_api` computes `__all__` from `dir()`. An empty `__all__` exports nothing.
    """

    def __init__(self, callables: Callables, **options: object) -> None:
        super().__init__(extensions=griffe.load_extensions(callables), **options)
        self.computed = callables.computed_exports
        self.stub_exports = callables.stub_exports

    def expand_exports(self, module: griffe.Module, seen: set | None = None) -> None:
        references = [e for e in module.exports or [] if isinstance(e, griffe.ExprName)]
        sources = {e.canonical_path.rsplit(".", 1)[0] for e in references}
        super().expand_exports(module, seen)
        if module.path not in self.stub_exports and (
            module.path in self.computed
            or sources & self.computed
            or any(e.name != "__all__" for e in references)
        ):
            self.computed.add(module.path)
            module.exports = None


def documentation(obj: griffe.Object | griffe.Alias) -> str:
    return obj.docstring.value.strip() if obj.docstring else ""


def constructor(obj: griffe.Class) -> griffe.Function | None:
    """The nearest `__init__` that is a function, or else the nearest `__new__`."""
    for name in ("__init__", "__new__"):
        for owned in (obj.members, obj.inherited_members):
            member = owned.get(name)
            if member is not None and member.is_function:
                return member
    return None


def summary(obj: griffe.Object | griffe.Alias) -> str:
    """The first paragraph, after the signature that compiled functions put before it.

    A class without documentation takes the summary of its constructor.
    """
    text = documentation(obj)
    if not text and obj.is_class and (init := constructor(obj)):
        text = documentation(init)
    paragraphs = text.split("\n\n")
    if len(paragraphs) > 1 and paragraphs[0].startswith(f"{obj.name}("):
        paragraphs = paragraphs[1:]
    lines = paragraphs[0].split("\n")
    if len(lines) > 1 and set(lines[1].strip()) == {"-"}:
        return ""
    return " ".join(paragraphs[0].split())


def built_parameters(function: griffe.Function) -> list[Parameter]:
    """The parameters of a function that no source defines, each as a `def` would write it."""
    result = []
    keyword_only = False
    for parameter in function.parameters:
        if parameter.kind is griffe.ParameterKind.keyword_only and not keyword_only:
            keyword_only = True
            result.append(Parameter(name=None, written="*"))
        written = parameter.name
        if parameter.annotation is not None:
            written += f": {parameter.annotation}"
        if parameter.default is not None:
            equals = " = " if parameter.annotation is not None else "="
            written += f"{equals}{parameter.default}"
        result.append(Parameter(name=parameter.name, written=written))
    return result


def members(obj: griffe.Class, callables: Callables) -> list[Member]:
    """The public methods and properties, with the inherited ones."""
    result = []
    for name, member in sorted(obj.all_members.items()):
        try:
            if name.startswith("_") or not member.is_public:
                continue
            if member.is_function:
                result.append(
                    Member(
                        name=name,
                        definitions=callables.definitions(member),
                        summary=summary(member),
                        property=False,
                    )
                )
            elif member.is_attribute and member.has_labels("property"):
                result.append(
                    Member(name=name, definitions=[], summary=summary(member), property=True)
                )
        except Unresolved:
            continue
    return result


def package_of(module: griffe.Module) -> str:
    return module.path if module.is_init_module else module.path.rsplit(".", 1)[0]


def below(target: griffe.Object, module: griffe.Module) -> bool:
    """Whether `target` is defined in the package of `module` or below it."""
    return f"{package_of(target.module)}.".startswith(f"{package_of(module)}.")


def from_wildcard(member: griffe.Alias, target: griffe.Object) -> bool:
    """Whether the module that a wildcard import reads gives `member` to its importers.

    A readable `__all__` decides. Without one, the module gives what it defines and what it
    imports from its own package. `scipy.stats._discrete_distns` computes `__all__` and
    imports `interp1d` from `scipy.interpolate` for its own use, so `scipy.stats.interp1d`
    is not a name.
    """
    source = member.target
    module = source.parent
    if module.exports:
        return member.name in module.exports
    if not source.is_alias:
        return True
    if source.wildcard_imported:
        return from_wildcard(source, target)
    return below(target, module)


def is_public(member: griffe.Object | griffe.Alias, target: griffe.Object) -> bool:
    """Apply Griffe's rules, with four exceptions that named symbols prove.

    `pandas.tests.frame.test_api.TestDataFrameMisc` is in a test folder that the wheel ships,
    which Griffe treats as public. `sklearn.conftest.fetch_species_distributions` is in a
    pytest file that the wheel ships, which Griffe treats as public too.

    `pandas.tseries.holiday.USFederalHolidayCalendar` is defined in its module, has a public
    name and is in the pandas user guide, but `__all__` leaves it out, which Griffe treats
    as private. A module's own class or function with a public name is public.

    `pandas.api.types.is_numeric_dtype` comes from a wildcard import and is not in `__all__`,
    which Griffe treats as private. Griffe names this convention public wildcard imports.
    """
    if target.is_module:
        return member.name not in {"tests", "conftest"} and (
            member.is_exported or not member.is_alias
        )
    if not member.is_alias:
        return True
    if member.wildcard_imported:
        return member.is_exported or from_wildcard(member, target)
    return member.is_public


def is_reexport(
    module: griffe.Module, member: griffe.Object | griffe.Alias, target: griffe.Object
) -> bool:
    """Whether an import that Griffe treats as private gives the target a public name.

    The module must have no `__all__` that Griffe can read, and the target must be defined in
    the module's package or below it. `scipy.io.loadmat` is such an import, and
    `pymoo.vendor.vendor_scipy.LocalSearch` is an import from another package.
    """
    return member.is_alias and not module.exports and below(target, module)


def shadowing(
    imports: dict[str, tuple[str, bool]], package: griffe.Module, path: str, target: griffe.Module
) -> griffe.Object:
    """The class or function that a statement binds over a submodule of the same name.

    `scipy.optimize.least_squares` is a function, imported over the submodule that defines
    it. Griffe keeps the submodule. `numpy.memmap` is the same, and its stub names the class.
    """
    seen = set()
    keys = [path, target.path]
    while keys:
        key = keys.pop(0)
        if key in seen or key not in imports:
            continue
        seen.add(key)
        bound, wildcard = imports[key]
        try:
            imported = package.get_member(bound.split(".", 1)[1])
            imported = imported.final_target if imported.is_alias else imported
        except (KeyError, IndexError, *Unresolved):
            continue
        if imported.is_module:
            keys += [bound, imported.path]
        elif (imported.is_class or imported.is_function) and (
            not wildcard or not imported.parent.exports or imported.name in imported.parent.exports
        ):
            return imported
    return target


def public_paths(
    module: griffe.Module,
    path: str,
    imports: dict[str, tuple[str, bool]],
    ancestors: frozenset[str],
) -> Iterator[tuple[str, griffe.Object]]:
    package = module.package
    for name, member in sorted(module.members.items()):
        try:
            if name.startswith("_"):
                continue
            target = member.final_target if member.is_alias else member
            if target.is_module:
                target = shadowing(imports, package, f"{module.path}.{name}", target)
            if not is_public(member, target) and (
                target.is_module or not is_reexport(module, member, target)
            ):
                continue
            if target.path in ancestors or target.path.split(".")[0] != package.name:
                continue
            if target.is_module:
                yield from public_paths(
                    target, f"{path}.{name}", imports, ancestors | {target.path}
                )
            elif target.is_class or target.is_function:
                yield f"{path}.{name}", target
        except Unresolved:
            continue


def entries(module: griffe.Module, callables: Callables) -> list[Entry]:
    """Name each definition once for each public name, by the path with the fewest parts."""
    names: dict[tuple[str, str], tuple[str, griffe.Object]] = {}
    package = module.name
    for path, target in public_paths(module, package, callables.imports, frozenset({module.path})):
        key = (target.path, path.rsplit(".", 1)[1])
        best = names.get(key)
        if best is None or (path.count("."), len(path), path) < (
            best[0].count("."),
            len(best[0]),
            best[0],
        ):
            names[key] = (path, target)
    return [
        Entry(
            kind="symbol",
            package=package,
            name=path,
            callable="class" if target.is_class else "function",
            definitions=callables.definitions(target),
            summary=summary(target),
            keywords=[" ".join(documentation(target).split("\nExamples\n", 1)[0].split())],
            members=members(target, callables) if target.is_class else [],
            rules=[],
        )
        for path, target in sorted(names.values(), key=lambda item: item[0])
    ]


def main() -> None:
    source, package, version = sys.argv[1:]
    logging.getLogger("griffe").setLevel(logging.ERROR)
    root = Path(source).resolve()
    if (root / package).is_dir():
        root /= package
    elif (root / "src" / package).is_dir():
        root /= f"src/{package}"
    # Griffe follows directory links and does not stop at one that points back up.
    for folder, names, _files in os.walk(root):
        for name in names:
            if (Path(folder) / name).is_symlink():
                sys.exit(
                    f"Index building does not follow the directory link {Path(folder) / name}."
                )
    callables = Callables()
    loader = Loader(callables, search_paths=[root.parent], allow_inspection=False)
    module = loader.load(root)
    loader.resolve_aliases(external=False)
    json.dump(
        {
            "package": package,
            "version": version,
            "entries": entries(module, callables),
        },
        sys.stdout,
    )


if __name__ == "__main__":
    main()
