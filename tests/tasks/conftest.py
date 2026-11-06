from __future__ import annotations

import time
from types import SimpleNamespace
from typing import TYPE_CHECKING

import pytest

from tasks import download as download_module

if TYPE_CHECKING:
    from pytest_mock import MockerFixture, MockType


@pytest.fixture
def sleep(mocker: MockerFixture) -> MockType:
    # The module's own clock, so the patch does not reach other threads that call the shared `time.sleep`.
    sleep = mocker.create_autospec(time.sleep)
    mocker.patch.object(download_module, "time", SimpleNamespace(sleep=sleep))
    return sleep
