import _missing_from_all_noall
import _missing_from_all_guard
from _missing_from_all import LIMIT, shared

VALUE = (LIMIT, shared, _missing_from_all_noall.shared, _missing_from_all_guard.shared)
from _missing_from_all_multiline import _PRIVATE, shared_multiline

MORE = (_PRIVATE, shared_multiline)

from unittest import mock

import single_user_constant

PATCHED = mock.patch("single_user_constant._RETRIES", 1)
OBJECT = mock.patch.object(single_user_constant, "_PATTERN", "y")

from _module_variable_case_noall import shared_value
from _missing_from_all_aug import aug_shared
from _missing_from_all_call import call_shared
from _missing_from_all_empty import empty_shared
from .. import nothing

EXTRA = (shared_value, aug_shared, call_shared, empty_shared, nothing)
from walrus_candidate import exported
from _missing_from_all_nested import run_dynamic
from undocumented_public_api_imported import documented_import, shared_doc
from _missing_from_all_hanging import gamma
from _missing_from_all_hanging_comment import gamma as other_gamma
from _missing_from_all_bare import gamma as bare_gamma
from _missing_from_all_reexport import helper as reexported_helper
from _missing_from_all_sorted import Gamma, alpha, beta
import _missing_from_all_imported
from _missing_from_all_imported import Path, join, run
from _missing_from_all_empty_comment import commented_shared
from _missing_from_all_last_comment import gamma as last_gamma
from _missing_from_all_last_no_comma import gamma as bare_last_gamma
from _missing_from_all_last_bracket import gamma as bracket_gamma
from _missing_from_all_last_comma_line import gamma as comma_line_gamma

IMPORTED = (_missing_from_all_imported.sys, Path, join, run)
from _missing_from_all_parenthesized_tail import gamma as tail_gamma
from _missing_from_all_parenthesized_next import beta as next_beta
from _missing_from_all_noall_no_eol import shared_no_eol
