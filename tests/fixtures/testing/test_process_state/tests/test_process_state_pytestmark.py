import os

import pytest

pytestmark = [pytest.mark.usefixtures("restore_environment")]


def test_environ() -> None:
    os.environ["MODE"] = "fast"
