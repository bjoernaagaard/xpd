"""Build local programs for the import paths measured by the articles."""

from pathlib import Path

imports = {
    "nine": "argparse asyncio decimal email.message http.server json sqlite3 unittest xml.dom.minidom",
    "docker": "json sqlite3 requests yaml rich.console:Console rich.table:Table",
    "sxz": "asyncio csv json sqlite3 statistics urllib.request decimal:Decimal",
    "fleet": "asyncio csv decimal email.message http.client json sqlite3 statistics tomllib urllib.request xml.etree.ElementTree zipfile pathlib:Path requests rich.console:Console rich.table:Table",
    "lemire": "json csv decimal sqlite3 asyncio email.parser http.client urllib.request xml.etree.ElementTree zipfile tarfile statistics numpy pandas requests rich.console",
}


def build(case: str, work: Path):
    if case == "boto":
        for label, stmt in [("eager", "import boto3"), ("lazy", "def upload():\n    import boto3")]:
            (work / f"{case}_{label}.py").write_text(stmt + "\nprint('mycli 1.0')\n")
        return
    if case == "stats":
        common = "import atexit, datetime, json, sys, warnings\nfrom pathlib import Path\nfrom platformdirs import user_cache_dir\nfrom slugify import slugify\nfrom termcolor import colored\n"
        extra = "import httpx\nfrom prettytable import PrettyTable, TableStyle\nfrom pytablewriter import HtmlTableWriter, NumpyTableWriter, PandasDataFrameWriter, RstSimpleTableWriter, String, TsvTableWriter\nfrom pytablewriter.style import Align, Style, ThousandSeparator\nfrom typing import Any\nfrom configparser import ConfigParser\nimport tomllib\n"
        parser = "import argparse, re\nfrom dateutil.relativedelta import relativedelta\np=argparse.ArgumentParser(prog='pypistats')\ns=p.add_subparsers()\nfor name in ('recent','overall','python_major','python_minor','system'): s.add_parser(name)\np.parse_args()\n"
        for label, block in [("inline", common), ("top", common + extra)]:
            (work / f"stats_{label}.py").write_text(block + parser)
        return
    for label in ["eager", "lazy", "lazy_modules"] if case == "fleet" else ["eager", "lazy"]:
        modules = imports[case].split()
        lines = [] if case == "nine" else ["import argparse", "import sys"]
        if label == "lazy_modules":
            lines.append("__lazy_modules__ = " + repr([m.partition(":")[0] for m in modules]))
        for item in modules:
            module, sep, name = item.partition(":")
            stmt = f"from {module} import {name}" if sep else f"import {module}"
            lines.append(("lazy " if label == "lazy" else "") + stmt)
        if case == "nine":
            lines.append("print('ready')")
        elif case == "fleet":
            lines.extend(
                [
                    "p=argparse.ArgumentParser(prog='fleetctl')",
                    "s=p.add_subparsers(required=True)",
                    "for n in ('version','summary','report'): s.add_parser(n)",
                    "p.parse_args()",
                    "print('fleetctl 1.0.0')",
                ]
            )
        else:
            lines.extend(
                [
                    "p=argparse.ArgumentParser()",
                    f"p.add_argument('--version', action='version', version='{case} 1.0')",
                ]
            )
            if case == "sxz":
                lines.extend(
                    [
                        "s=p.add_subparsers()",
                        "for n in ('summary','export','store','fetch','watch'): s.add_parser(n)",
                    ]
                )
            if case == "lemire":
                lines.extend(
                    [
                        "p.add_argument('cmd')",
                        "p.add_argument('path')",
                        "args=p.parse_args()",
                        "if args.cmd=='stats': print(pandas.read_csv(args.path).describe().loc['mean'].to_string())",
                        "else:",
                        "    with open(args.path) as f: print(statistics.fmean(float(row[0]) for row in list(csv.reader(f))[1:]))",
                    ]
                )
            else:
                lines.append("p.parse_args()")
        (work / f"{case}_{label}.py").write_text("\n".join(lines) + "\n")
