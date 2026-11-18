__lazy_modules__ = [
    "ineffective_lazy_declaration_pkg",
    "ineffective_lazy_declaration_pkg.sub",
    "..helper",
    '.missing',
    f"{__spec__.parent}.other",
]

import ineffective_lazy_declaration_pkg
from ineffective_lazy_declaration_pkg.sub import other
from ..helper import VALUE


def use() -> object:
    return ineffective_lazy_declaration_pkg, other, VALUE
