[`unregistered-marker`](../src/rules/testing/unregistered_marker.md) reports a `pytest.mark` name that the project's
pytest configuration does not register under `markers`, so a misspelled mark that pytest only warns about is caught
before the run. It reads the same configuration files pytest does.
