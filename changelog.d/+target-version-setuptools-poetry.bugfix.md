A project without `requires-python` now takes its Python version from Poetry's `python` dependency or the setuptools
`python_requires` in `setup.cfg` or `setup.py`, so rules no longer offer syntax newer than the project supports.
