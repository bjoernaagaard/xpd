"""Write xpd entries from Griffe's static API model."""

import importlib.metadata
import json
import logging
import platform
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


require_griffe()
import griffe  # noqa: E402  Name the exact install command before importing the build dependency.


class Member(TypedDict):
    name: str
    parameters: list[str]
    summary: str


class Entry(TypedDict):
    kind: str
    package: str
    name: str
    callable: str
    parameters: list[str]
    summary: str
    keywords: list[str]
    members: list[Member]
    rules: list[str]


class StubOverloads(griffe.Extension):
    """Keep callable declarations that exist only as overloads in stubs."""

    def on_module_members(self, *, mod: griffe.Module, **_kwargs: object) -> None:
        self.add_overloads(mod)

    def on_class_members(self, *, cls: griffe.Class, **_kwargs: object) -> None:
        self.add_overloads(cls)

    def on_attribute_instance(self, *, attr: griffe.Attribute, **_kwargs: object) -> None:
        if (
            attr.parent.is_module
            and attr.annotation is None
            and isinstance(attr.value, (griffe.ExprName, griffe.ExprAttribute))
        ):
            target = attr.value.canonical_path
            if target != attr.path:
                attr.parent.set_member(attr.name, griffe.Alias(attr.name, target))

    @staticmethod
    def add_overloads(obj: griffe.Module | griffe.Class) -> None:
        if obj.filepath.suffix != ".pyi":
            return
        for name, overloads in obj.overloads.items():
            if name not in obj.members and overloads:
                first = overloads[0]
                function = griffe.Function(
                    name,
                    parameters=first.parameters,
                    returns=first.returns,
                    lineno=first.lineno,
                    endlineno=first.endlineno,
                    docstring=first.docstring,
                )
                function.overloads = overloads
                obj.set_member(name, function)


def documentation(obj: griffe.Object | griffe.Alias) -> str:
    return obj.docstring.value.strip() if obj.docstring else ""


def summary(obj: griffe.Object | griffe.Alias) -> str:
    paragraphs = documentation(obj).split("\n\n")
    if len(paragraphs) > 1 and ("--" in paragraphs[0] or paragraphs[0].endswith(")")):
        paragraphs = paragraphs[1:]
    return " ".join(paragraphs[0].splitlines())


def parameters(obj: griffe.Object | griffe.Alias) -> list[str]:
    if obj.is_class:
        constructor = obj.all_members.get("__init__")
        if constructor is None or not constructor.is_function:
            return []
        obj = constructor
    return [p.name for p in obj.parameters if p.name not in {"self", "cls"}]


def members(obj: griffe.Class) -> list[Member]:
    result = []
    for name, member in sorted(obj.all_members.items()):
        try:
            if not name.startswith("_") and member.is_public and member.is_function:
                result.append(
                    Member(name=name, parameters=parameters(member), summary=summary(member))
                )
        except (griffe.AliasResolutionError, griffe.CyclicAliasError):
            continue
    return result


def public_members(obj: griffe.Module) -> Iterator[tuple[str, griffe.Object | griffe.Alias]]:
    text = documentation(obj).lower().split("\n\n", 1)[0]
    if any(
        marker in text
        for marker in (
            "should be accessed through",
            "internal use only",
            "not part of the public api",
            "vendored",
        )
    ):
        return
    for name, member in sorted(obj.members.items()):
        if name.startswith("_") or name in {
            "tests",
            "test",
            "testing",
            "vendor",
            "vendored",
            "externals",
        }:
            continue
        if member.is_alias:
            try:
                if member.is_module and not member.is_exported:
                    continue
            except (griffe.AliasResolutionError, griffe.CyclicAliasError):
                continue
        if member.is_public or (member.is_alias and not obj.exports):
            yield name, member


def entries(
    obj: griffe.Object | griffe.Alias,
    path: str,
    package: str,
    ancestors: frozenset[str],
) -> Iterator[Entry]:
    try:
        target = obj.final_target if obj.is_alias else obj
        if target.path in ancestors or (
            not target.path.startswith(f"{package}.") and target.path != package
        ):
            return
        if target.is_module:
            for name, member in public_members(target):
                yield from entries(member, f"{path}.{name}", package, ancestors | {target.path})
        elif target.is_class or target.is_function:
            yield Entry(
                kind="symbol",
                package=package,
                name=path,
                callable="class" if target.is_class else "function",
                parameters=parameters(target),
                summary=summary(target),
                keywords=[documentation(target).split("\nExamples\n", 1)[0]],
                members=members(target) if target.is_class else [],
                rules=[],
            )
    except (griffe.AliasResolutionError, griffe.CyclicAliasError):
        return


def main() -> None:
    source, package, version = sys.argv[1:]
    logging.getLogger("griffe").setLevel(logging.ERROR)
    root = Path(source).resolve()
    if (root / package).is_dir():
        root /= package
    elif (root / "src" / package).is_dir():
        root /= f"src/{package}"
    module = griffe.load(
        root,
        search_paths=[root.parent],
        allow_inspection=False,
        resolve_aliases=True,
        resolve_external=False,
        docstring_parser="numpy",
        extensions=griffe.load_extensions(StubOverloads),
    )
    json.dump(
        {
            "schema_version": 1,
            "package": package,
            "version": version,
            "inputs": {
                "griffe": GRIFFE_VERSION,
                "python": platform.python_version(),
                "archive": None,
                "examples": None,
            },
            "entries": sorted(
                entries(module, package, package, frozenset()), key=lambda entry: entry["name"]
            ),
        },
        sys.stdout,
    )


if __name__ == "__main__":
    main()
