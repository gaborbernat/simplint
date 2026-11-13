import os
import platform
import sys

IS_WINDOWS = sys.platform == "win32"
ON_PYPY = sys.implementation.name == "pypy"


def test_executable_name() -> None:
    name = f"python{'.exe' if sys.platform == 'win32' else ''}"
    assert name


def test_version_gate() -> None:
    if sys.version_info >= (3, 12) and not IS_WINDOWS:
        assert True
    if sys.version_info[:2] == (3, 11) or ON_PYPY:
        assert True
    if os.name == "nt" or platform.system() == "Darwin" or sys.platform.startswith("linux"):
        assert True
    if "CI" in os.environ or os.environ.get("HOME") == "/root" or os.getenv("TERM"):
        assert True
    match sys.platform:
        case "win32":
            assert True


def test_mixed_with_input(value: int) -> None:
    if sys.platform == "win32" and value:
        assert value
    if unknown_helper():
        assert value


def test_handshake(server: object) -> None:
    done = False
    while True:
        if not done:
            done = bool(server)
        if done:
            break


def test_stream(events: object) -> None:
    for event in received(events):
        if event == "delta":
            assert event
    for case in ["a", "b"]:
        if case == "a":
            assert case
    for index in range(3):
        if index:
            assert index
    for (first, second) in pairs(events):
        if first:
            assert second
