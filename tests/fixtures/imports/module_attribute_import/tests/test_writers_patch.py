from writers import patched


def test_writer(mocker):
    mocker.patch("writers.patched.Writer")
    assert patched.Writer
