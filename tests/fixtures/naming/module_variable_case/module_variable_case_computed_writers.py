import sys
from unittest import mock

import module_variable_case_computed_control_other as other
import module_variable_case_computed_patch_object as patched
import module_variable_case_computed_setattr as target


def write_by_setattr(name: str) -> None:
    setattr(target, sys.argv[1], name)


def write_by_patch_object() -> None:
    mock.patch.object(patched, sys.argv[1]).start()


def write_by_fstring(name: str) -> None:
    with mock.patch(f"module_variable_case_computed_fstring.{name}"):
        pass


def write_other_module() -> None:
    setattr(other, sys.argv[1], 1)
