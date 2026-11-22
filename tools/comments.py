# /// script
# requires-python = ">=3.11,<3.15"
# dependencies = [
#   "tree-sitter==0.25.2", "tree-sitter-rust==0.24.2", "tree-sitter-python==0.25.0",
#   "tree-sitter-javascript==0.25.0", "tree-sitter-typescript==0.23.2",
#   "tree-sitter-toml==0.7.0", "tree-sitter-yaml==0.7.2", "tree-sitter-xml==0.7.0",
# ]
# ///
import argparse
import json
import re
import subprocess
from bisect import bisect_left
from functools import cache
from importlib import import_module
from itertools import groupby
from pathlib import Path

from tree_sitter import Language, Parser

LANGUAGES = {
    ".rs": "rust",
    ".py": "python",
    ".mjs": "javascript",
    ".js": "javascript",
    ".jsx": "javascript",
    ".cjs": "javascript",
    ".ts": "typescript",
    ".tsx": "tsx",
    ".toml": "toml",
    ".yaml": "yaml",
    ".yml": "yaml",
    ".svg": "xml",
}
BASELINE = Path(__file__).with_name("legacy.json")
SCRIPT = Path("tools/comments.py")
REVISION = "8f1403be577f03c577f03432b3cf15871ffb3b76"


@cache
def parser(language):
    module = import_module("tree_sitter_" + ("typescript" if language == "tsx" else language))
    factory = {
        "tsx": "language_tsx",
        "typescript": "language_typescript",
        "xml": "language_xml",
    }.get(language, "language")
    return Parser(Language(getattr(module, factory)()))


def tokens(path, data):
    if path.name == ".gitignore":
        offset = 0
        for line in data.splitlines(keepends=True):
            if line.startswith(b"#"):
                yield offset, offset + len(line.rstrip(b"\r\n")), "comment"
            offset += len(line)
        return
    tree = parser(LANGUAGES[path.suffix]).parse(data)
    if tree.root_node.has_error:
        raise ValueError(f"{path}: invalid syntax; fix the parse error before auditing comments")
    stack = [tree.root_node]
    while stack:
        node = stack.pop()
        if node.type.lower() in {
            "comment",
            "line_comment",
            "block_comment",
            "hash_bang_line",
            "shebang",
        }:
            kind = "rustdoc" if node.child_by_field_name("doc") else node.type.lower()
            if (
                node.text.startswith(b"/**")
                and node.next_named_sibling is not None
                and node.next_named_sibling.type == "export_statement"
            ):
                kind = "exported_jsdoc"
            yield node.start_byte, node.start_byte + len(node.text.rstrip(b"\r\n")), kind
        else:
            stack.extend(reversed(node.named_children))


def protection(path, text, kind, row, source):
    if kind == "exported_jsdoc":
        return "JSDoc on an exported declaration"
    if re.fullmatch(
        r"(?:#|//|/\*) SPDX-License-Identifier: (?:MIT|Apache-2\.0|BSD-3-Clause)(?: \*/)?", text
    ):
        return "machine-readable source license"
    if kind == "rustdoc":
        return "Rust documentation attribute (rustdoc or derive macro input)"
    if kind in {"hash_bang_line", "shebang"} or (
        path.suffix == ".py" and row == 1 and text.startswith("#!")
    ):
        return "interpreter directive"
    if (
        path == SCRIPT
        and b"\n# ///\n" in source
        and source.startswith(b"# /// script\n")
        and (
            text in {"# ///", "# /// script", "# dependencies = [", "# ]"}
            or re.fullmatch(r'# requires-python = "[<>=.,0-9]+"', text)
            or re.fullmatch(r'# +(?:"[a-z-]+==[0-9.]+", *)+', text)
        )
        and row <= source.split(b"\n# ///\n", 1)[0].count(b"\n") + 2
    ):
        return "uv script metadata"
    if path.suffix == ".py":
        if re.fullmatch(
            r"# (?:noqa(?:: [A-Z]+\d+(?:, *[A-Z]+\d+)*)?|(?:fmt|ruff): (?:off|on|skip)|type: ignore(?:\[[\w,-]+\])?)",
            text,
        ):
            return "Python tool directive"
        if path.parts[0] == "samples" and re.fullmatch(r"# xpd: [\w-]+(?:, *[\w-]+)*", text):
            return "expected diagnostic read by the sample validators"
    if path.parts[0] == ".github" and re.fullmatch(
        r"# v[0-9]+(?:\.[0-9]+)*(?: # jactionlint ignore=[\w-]+)?", text
    ):
        line = source.splitlines()[row - 1]
        if re.search(rb"uses: [\w./-]+@[a-f0-9]{40} +#", line):
            return (
                "Dependabot version for a pinned action; jactionlint reads an attached suppression"
            )
    if path.parts[0] == ".github" and re.fullmatch(r"# jactionlint ignore=[\w-]+", text):
        return "jactionlint suppression"
    if path.suffix == ".toml" and re.fullmatch(r"#:schema https?://\S+", text):
        return "TOML schema directive"
    return ""


def inventory(path, data):
    ends = [m.start() for m in re.finditer(b"\n", data)]
    spans = list(tokens(path, data))
    code = bytearray(data)
    for start, end, _ in spans:
        code[start:end] = re.sub(b"[^\n]", b" ", data[start:end])
    lines = bytes(code).split(b"\n")
    for start, end, kind in spans:
        first = bisect_left(ends, start)
        last = bisect_left(ends, max(start, end - 1))
        text = data[start:end].decode()
        only = [i + 1 for i in range(first, last + 1) if not lines[i].strip()]
        yield dict(
            path=str(path),
            line=first + 1,
            text=text,
            only=only,
            column=start - (ends[first - 1] + 1 if first else 0),
            protected=protection(path, text, kind, first + 1, data),
        )


def sources():
    raw = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"]
    )
    for name in sorted(set(raw.decode().split("\0")) - {""}):
        path = Path(name)
        if (
            path.is_file()
            and path.parent != Path("rule-tests/__snapshots__")
            and (path.name == ".gitignore" or path.suffix in LANGUAGES)
        ):
            yield path


def unchanged_lines(path, line_count):
    patch = subprocess.check_output(
        [
            "git",
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--unified=0",
            REVISION,
            "--",
            str(path),
        ]
    )
    mapping = {}
    old = new = 1
    for match in re.finditer(rb"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", patch, re.M):
        a, b, c, d = (int(x if x is not None else 1) for x in match.groups())
        old_start = a if b else a + 1
        new_start = c if d else c + 1
        mapping.update(zip(range(new, new_start), range(old, old_start), strict=True))
        old, new = old_start + b, new_start + d
    mapping.update((i, old + i - new) for i in range(new, line_count + 1))
    return mapping


def ranges(lines):
    parts = []
    for _, run in groupby(enumerate(sorted(lines)), lambda item: item[1] - item[0]):
        group = [n for _, n in run]
        parts.append(str(group[0]) if len(group) == 1 else f"{group[0]}-{group[-1]}")
    return ",".join(parts)


def expand(value):
    result = set()
    for part in value.split(","):
        bounds = list(map(int, part.split("-")))
        result.update(range(bounds[0], bounds[-1] + 1))
    return result


def check(retire):
    changed = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", "-z", REVISION], capture_output=True
    )
    if changed.returncode:
        raise SystemExit(
            f"Cannot compare the audit revision. Fetch it with git fetch origin {REVISION}"
        )
    modified = set(changed.stdout.decode().split("\0"))
    baseline = json.loads(BASELINE.read_text())
    retained = {}
    errors = []
    total = 0
    for path in sources():
        name = str(path)
        data = path.read_bytes()
        allowed = expand(baseline[name]) if name in baseline else set()
        kept = set()
        mapping = original = None
        if name in modified and allowed:
            mapping = unchanged_lines(path, data.count(b"\n") + 1)
            before = subprocess.check_output(["git", "show", f"{REVISION}:{name}"])
            original = {(r["line"], r["column"], r["text"]) for r in inventory(path, before)}
        for row in inventory(path, data):
            if row["protected"]:
                continue
            line = row["line"]
            old = line if mapping is None else mapping.get(line)
            if (
                old in allowed
                and old not in kept
                and (original is None or (old, row["column"], row["text"]) in original)
            ):
                kept.add(old)
            else:
                errors.append(
                    f"{path}:{line}: ordinary comment: remove it or document its required consumer"
                )
        if kept:
            retained[name] = ranges(kept)
            total += len(kept)
    if not retire:
        for name, value in baseline.items():
            if value != retained.get(name):
                errors.append(
                    f"{name}: legacy occurrences disappeared: run uv run tools/comments.py retire"
                )
    if errors:
        print("\n".join(errors))
        return 1
    if retire:
        BASELINE.write_text(json.dumps(retained, indent=2) + "\n")
    print(f"Comment policy passed ({total} reviewed occurrences).")
    return 0


def main():
    args = argparse.ArgumentParser()
    args.add_argument("command", choices=["scan", "files", "lint", "retire"])
    command = args.parse_args().command
    if command in {"lint", "retire"}:
        return check(command == "retire")
    for path in sources():
        if command == "files":
            print(path)
        else:
            for item in inventory(path, path.read_bytes()):
                print(json.dumps(item, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
