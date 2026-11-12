import json
import os
import sys
from unittest import mock

import pytest


def test_environ_item() -> None:
    os.environ["MODE"] = "fast"
    assert os.environ["MODE"] == "fast"


def test_environ_update() -> None:
    os.environ.update({"MODE": "fast"})
    os.putenv("MODE", "fast")
    del os.environ["MODE"]


def test_chdir(tmp_path) -> None:
    os.chdir(tmp_path)


def test_sys_state() -> None:
    sys.path.insert(0, "src")
    sys.argv = ["tool", "--help"]
    del sys.modules["json"]
    sys.modules.pop("json")


def test_module_attribute() -> None:
    json.dumps = repr


@pytest.fixture
def environment() -> None:
    os.environ["MODE"] = "fast"


class TestState:
    def test_inside_class(self) -> None:
        sys.path.append("src")


def test_monkeypatch(monkeypatch: pytest.MonkeyPatch, tmp_path) -> None:
    monkeypatch.setenv("MODE", "fast")
    monkeypatch.chdir(tmp_path)


def test_patch_dict() -> None:
    with mock.patch.dict(os.environ, {"MODE": "fast"}):
        os.environ["OTHER"] = "slow"


def test_open_block(tmp_path) -> None:
    with open(tmp_path / "file", "w"):
        os.environ["MODE"] = "fast"


def test_finally(tmp_path) -> None:
    try:
        os.chdir(tmp_path)
    finally:
        os.chdir("/")


def test_saved_directory(tmp_path) -> None:
    previous = os.getcwd()
    os.chdir(tmp_path)
    os.chdir(previous)


def test_saved_path() -> None:
    saved = sys.path[:]
    sys.path.insert(0, "src")
    sys.path[:] = saved


def test_saved_attribute() -> None:
    original = json.dumps
    json.dumps = repr
    json.dumps = original


def test_cleanup(request: pytest.FixtureRequest) -> None:
    request.addfinalizer(lambda: os.environ.pop("MODE"))
    os.environ["MODE"] = "fast"


@pytest.fixture
def yielded_environment():
    os.environ["MODE"] = "fast"
    yield
    del os.environ["MODE"]


@pytest.mark.usefixtures("restore_environment")
def test_restoring_fixture() -> None:
    os.environ["MODE"] = "fast"


def test_pytester(pytester: pytest.Pytester) -> None:
    os.chdir(pytester.path)


def test_rebinding() -> None:
    os.environ = {}


class TestWithTeardown:
    def setup_method(self) -> None:
        os.environ["MODE"] = "fast"

    def teardown_method(self) -> None:
        del os.environ["MODE"]


def helper() -> None:
    os.environ["MODE"] = "fast"


def test_saved_copies() -> None:
    environment = dict(os.environ)
    arguments = list(sys.argv)
    modules = sys.modules.copy()
    os.environ["MODE"] = "fast"
    sys.argv = ["tool"]
    sys.modules["fake"] = json
    os.environ.update(environment)
    sys.argv = arguments
    sys.modules.update(modules)
