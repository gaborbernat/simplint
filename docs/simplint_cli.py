"""Rebuild simplint's command line as an argparse parser so sphinx-argparse-cli can document it."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from argparse import ArgumentParser, RawDescriptionHelpFormatter
from typing import TYPE_CHECKING, NotRequired, TypedDict

if TYPE_CHECKING:
    from argparse import _SubParsersAction


class _Argument(TypedDict):
    id: str
    long: str | None
    short: str | None
    help: str | None
    flag: bool
    multiple: bool
    delimiter: str | None
    default: list[str]
    choices: list[str]
    value_name: str | None


class _Command(TypedDict):
    name: str
    about: str | None
    arguments: list[_Argument]
    subcommands: list[_Command]
    prog: NotRequired[str]


def build_parser() -> ArgumentParser:
    if (binary := shutil.which("simplint")) is None:
        msg = "simplint is not on PATH; build the docs through `tox run -e docs`"
        raise FileNotFoundError(msg)
    schema: _Command = json.loads(
        subprocess.run([binary, "cli-schema"], check=True, capture_output=True, text=True).stdout
    )
    parser = ArgumentParser(prog="simplint", description=schema["about"], formatter_class=RawDescriptionHelpFormatter)
    _add_arguments(parser, schema)
    return parser


def _add_arguments(parser: ArgumentParser, command: _Command) -> None:
    for argument in command["arguments"]:
        # clap help uses Markdown code spans; sphinx-argparse-cli renders help as reStructuredText
        help_text = re.sub(r"`([^`]+)`", r"``\1``", argument["help"] or "")
        if argument["delimiter"] == ",":
            help_text = f"{help_text}; comma-separated"
        if argument["flag"]:
            parser.add_argument(f"--{argument['long']}", action="store_true", help=help_text)
            continue
        default = argument["default"] if argument["multiple"] else next(iter(argument["default"]), None)
        choices = argument["choices"] or None
        if argument["long"] is None:
            parser.add_argument(
                argument["id"],
                nargs="*" if argument["multiple"] else None,
                default=default,
                choices=choices,
                metavar=argument["value_name"],
                help=help_text,
            )
        else:
            flags = [f"--{argument['long']}", *([f"-{argument['short']}"] if argument["short"] else [])]
            parser.add_argument(
                *flags, default=default, choices=choices, metavar=argument["value_name"], help=help_text
            )
    if command["subcommands"]:
        subparsers: _SubParsersAction[ArgumentParser] = parser.add_subparsers(dest="command", required=True)
        for subcommand in command["subcommands"]:
            child = subparsers.add_parser(subcommand["name"], help=subcommand["about"], description=subcommand["about"])
            _add_arguments(child, subcommand)
