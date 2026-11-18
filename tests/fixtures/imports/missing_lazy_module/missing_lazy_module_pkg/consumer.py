__lazy_modules__ = [f"{__spec__.parent}.sub"]

import missing_lazy_module_pkg
from . import helper
from .sub import tool


def value() -> str:
    return helper.VALUE + tool.VALUE + missing_lazy_module_pkg.VALUE
