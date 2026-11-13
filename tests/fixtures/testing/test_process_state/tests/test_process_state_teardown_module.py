import os


def teardown_module() -> None:
    os.environ.pop("MODE", None)


def test_environ() -> None:
    os.environ["MODE"] = "fast"
