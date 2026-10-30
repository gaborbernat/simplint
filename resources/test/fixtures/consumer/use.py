import SMP113_noall
from SMP113 import LIMIT, shared

VALUE = (LIMIT, shared, SMP113_noall.shared)
from SMP113_multiline import _PRIVATE, shared_multiline

MORE = (_PRIVATE, shared_multiline)

from unittest import mock

import SMP050

PATCHED = mock.patch("SMP050._RETRIES", 1)
OBJECT = mock.patch.object(SMP050, "_PATTERN", "y")

from SMP102_noall import shared_value
from SMP113_aug import aug_shared
from SMP113_call import call_shared
from SMP113_empty import empty_shared
from .. import nothing

EXTRA = (shared_value, aug_shared, call_shared, empty_shared, nothing)
from SMP100 import exported
