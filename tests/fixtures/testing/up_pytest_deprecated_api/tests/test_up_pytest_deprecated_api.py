import pytest


class TestFixtureNamedSetup:
    @pytest.fixture(autouse=True)
    def setup(self):
        pass


def test_tmpdir(tmpdir) -> None:
    assert tmpdir


def test_factories(tmpdir_factory, testdir) -> None:
    assert tmpdir_factory
    assert testdir


@pytest.fixture
def workspace(tmpdir):
    return tmpdir


@pytest.mark.slow
@pytest.fixture
def marked_fixture():
    return 1


@pytest.mark.usefixtures("workspace")
@pytest.fixture
def using_fixture():
    return 1


def test_outcomes() -> None:
    pytest.fail(msg="broken")
    pytest.skip(msg="later")
    pytest.exit(msg="stop")
    pytest.fail(reason="broken")


def test_raises_message() -> None:
    with pytest.raises(ValueError, message="expected"):
        int("x")


def test_warns_none() -> None:
    with pytest.warns(None):
        pass


def test_warns_removed() -> None:
    with pytest.raises(TypeError):
        pytest.warns(None)


def helper(tmpdir):
    return tmpdir
