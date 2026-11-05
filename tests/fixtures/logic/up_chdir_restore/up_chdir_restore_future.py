"""Build helpers."""

from __future__ import annotations

import os


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
