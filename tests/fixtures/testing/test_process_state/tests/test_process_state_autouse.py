import os

import pytest


@pytest.fixture(autouse=True)
def clean_env():
    saved = dict(os.environ)
    yield
    os.environ.clear()
    os.environ.update(saved)


def test_write() -> None:
    os.environ["MODE"] = "debug"
