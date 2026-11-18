"""Docstring."""

from __future__ import annotations

import csv
import json

__lazy_modules__ = ["json", "zipfile"]

import zipfile


def use() -> object:
    return csv, json, zipfile
