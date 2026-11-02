`function-docstring` (SMP002) leaves alone docstrings code reads at runtime: those holding a `>>>` doctest, those of
objects decorated by anything but a known decorator such as `property` or `pytest.fixture`, and all of them when the run
reads `__doc__` or calls `getdoc`.
