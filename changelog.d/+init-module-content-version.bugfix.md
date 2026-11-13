[`init-module-content`](../src/rules/api/init_module_content.md) leaves a `__version__` alone when setuptools' `attr:`,
hatch's or pdm's version `path`, flit, bump-my-version, bump2version, tbump or `setup.py` reads it, and no longer
reports `from __future__` imports or `__lazy_modules__` in a package that is not public.
