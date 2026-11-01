Names imported through a directory without `__init__.py`, such as `from tests.helpers import mark` with pytest's rootdir
on `sys.path`, now count as imported, so `module-variable-case`, `unexported-public-name` and `single-user-constant` no
longer rename or move them.
