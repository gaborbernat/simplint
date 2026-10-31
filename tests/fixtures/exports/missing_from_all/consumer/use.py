import missing_from_all_noall
import missing_from_all_guard
from missing_from_all import LIMIT, shared

VALUE = (LIMIT, shared, missing_from_all_noall.shared, missing_from_all_guard.shared)
from missing_from_all_multiline import _PRIVATE, shared_multiline

MORE = (_PRIVATE, shared_multiline)

from unittest import mock

import single_user_constant

PATCHED = mock.patch("single_user_constant._RETRIES", 1)
OBJECT = mock.patch.object(single_user_constant, "_PATTERN", "y")

from module_variable_case_noall import shared_value
from missing_from_all_aug import aug_shared
from missing_from_all_call import call_shared
from missing_from_all_empty import empty_shared
from .. import nothing

EXTRA = (shared_value, aug_shared, call_shared, empty_shared, nothing)
from walrus_candidate import exported
from missing_from_all_nested import run_dynamic
from undocumented_public_api_imported import documented_import, shared_doc
from missing_from_all_hanging import gamma
from missing_from_all_hanging_comment import gamma as other_gamma
from missing_from_all_bare import gamma as bare_gamma
from missing_from_all_reexport import helper as reexported_helper
from missing_from_all_sorted import Gamma, alpha, beta
