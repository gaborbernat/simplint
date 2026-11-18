__lazy_modules__ = []

import missing_lazy_module_pkg
from missing_lazy_module_pkg import helper


class Widget(missing_lazy_module_pkg.Base):
    def value(self) -> str:
        return helper.VALUE
