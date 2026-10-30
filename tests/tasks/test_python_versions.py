from __future__ import annotations

import io
import runpy
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from tasks import python_versions

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


def test_with_a_candidate_accepts_only_release_candidates(mocker: MockerFixture) -> None:
    mocker.patch(
        "tasks.python_versions.fetch",
        autospec=True,
        return_value=[
            {"name": "Python 3.15.0rc1"},
            {"name": "Python 3.16.0b2"},
            {"name": "Python 2.7.18rc1"},
            {"name": "Python 3.15.0"},
        ],
    )

    assert python_versions.with_a_candidate() == {15}


def test_oldest_supported_skips_ended_cycles(mocker: MockerFixture) -> None:
    mocker.patch(
        "tasks.python_versions.fetch",
        autospec=True,
        return_value=[{"cycle": "3.10", "eol": "2000-01-01"}, {"cycle": "3.11", "eol": "2999-01-01"}],
    )

    assert python_versions.oldest_supported() == 11


@pytest.mark.parametrize("index", range(4), ids=("settings", "docs", "requires-python", "classifiers"))
def test_written_places_match_expected_spelling(tmp_path: Path, index: int) -> None:
    place = python_versions.written_places(15, 11)[index]
    previous = python_versions.written_places(14, 10)[index].spelling
    source = tmp_path / "source"
    source.write_text(f"before\n{previous}\nafter\n")

    assert python_versions.Place(path=source, pattern=place.pattern, spelling=place.spelling).said() == previous


def test_place_write_replaces_the_old_spelling(tmp_path: Path) -> None:
    source = tmp_path / "source"
    source.write_text("then to Python 3.10 here\n")
    place = python_versions.Place(
        path=source, pattern=python_versions.written_places(15, 11)[1].pattern, spelling="then to Python 3.11"
    )

    place.write(place.said())

    assert source.read_text() == "then to Python 3.11 here\n"


@pytest.fixture
def drifted(tmp_path: Path, mocker: MockerFixture) -> Path:
    source = tmp_path / "settings.rs"
    source.write_text("target_version: PythonVersion::PY310,\n")
    mocker.patch("tasks.python_versions._ROOT", tmp_path)
    mocker.patch("tasks.python_versions.newest_candidate", autospec=True, return_value=15)
    mocker.patch("tasks.python_versions.oldest_supported", autospec=True, return_value=11)
    place = python_versions.Place(
        path=source,
        pattern=python_versions.written_places(15, 11)[0].pattern,
        spelling="target_version: PythonVersion::PY311,",
    )
    mocker.patch("tasks.python_versions.written_places", autospec=True, return_value=[place])
    return source


def test_main_writes_the_drifted_spelling(drifted: Path) -> None:
    python_versions.main(check=False)

    assert drifted.read_text(encoding="utf-8") == "target_version: PythonVersion::PY311,\n"


@pytest.mark.usefixtures("drifted")
def test_main_check_fails_on_drift() -> None:
    with pytest.raises(SystemExit, match="1"):
        python_versions.main(check=True)


@pytest.mark.usefixtures("drifted")
def test_main_reports_when_nothing_drifted(capsys: pytest.CaptureFixture[str]) -> None:
    python_versions.main(check=False)
    capsys.readouterr()

    python_versions.main(check=True)

    assert capsys.readouterr().out == "the defaults name 3.11 through 3.15\n"


def test_newest_candidate_needs_both_a_candidate_and_a_classifier(mocker: MockerFixture) -> None:
    mocker.patch("tasks.python_versions.with_a_candidate", autospec=True, return_value={14, 15, 16})
    mocker.patch("tasks.python_versions.with_a_classifier", autospec=True, return_value={13, 14, 15})

    assert python_versions.newest_candidate() == 15


def test_fetch_decodes_the_json_body(mocker: MockerFixture) -> None:
    response = mocker.MagicMock(**{"__enter__.return_value": io.BytesIO(b'[{"name": "x"}]')})
    opened = mocker.patch("urllib.request.urlopen", autospec=True, return_value=response)

    assert (python_versions.fetch("https://example.com"), opened.call_args.kwargs) == ([{"name": "x"}], {"timeout": 30})


def test_said_exits_when_nothing_matches(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("tasks.python_versions._ROOT", tmp_path)
    source = tmp_path / "source"
    source.write_text("nothing\n")
    place = python_versions.Place(path=source, pattern=python_versions.written_places(15, 11)[0].pattern, spelling="x")

    with pytest.raises(SystemExit, match="1"):
        place.said()


def test_command_checks(monkeypatch: pytest.MonkeyPatch, mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=OSError("offline"))
    monkeypatch.setattr("sys.argv", ["tasks/python_versions.py", "--check"])

    with pytest.raises(OSError, match="offline"):
        runpy.run_path(str(Path(__file__).parents[2] / "tasks" / "python_versions.py"), run_name="__main__")


def test_with_a_classifier_reads_the_published_classifiers() -> None:
    assert {11, 12, 13, 14, 15} <= python_versions.with_a_classifier()
