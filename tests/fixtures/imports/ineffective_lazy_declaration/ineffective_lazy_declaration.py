from typing import TYPE_CHECKING

__lazy_modules__ = [
    "json",
    "pathlib",
    "json",  # again
    "os",
    "xml",
    "typing_extensions",
    "tomllib",
    "csv",
]

import csv
import json
import os
import xml.etree.ElementTree

if TYPE_CHECKING:
    import typing_extensions

try:
    import tomllib
except ImportError:
    tomllib = None


def use() -> object:
    return csv, json, os, xml, typing_extensions, tomllib
