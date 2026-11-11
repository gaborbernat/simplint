Pair simplint with Ruff
#######################

simplint leaves the cases a Ruff rule reports to Ruff, even when your configuration does not select that rule (see
:doc:`/explanation/design-principles`). Each simplint rule page names any Ruff overlap. To have those cases reported:

- Run Ruff with every rule selected (``select = ["ALL"]``) and preview enabled, but the undocumented-* rules
  (``D1``), which ask for
  the docstrings simplint removes (see :ref:`resolve-ruff-conflicts`): `undocumented-public-module <https://docs.astral.sh/ruff/rules/undocumented-public-module/>`__, `undocumented-public-package <https://docs.astral.sh/ruff/rules/undocumented-public-package/>`__, `undocumented-public-class <https://docs.astral.sh/ruff/rules/undocumented-public-class/>`__, `undocumented-public-method <https://docs.astral.sh/ruff/rules/undocumented-public-method/>`__, `undocumented-public-function <https://docs.astral.sh/ruff/rules/undocumented-public-function/>`__, `undocumented-magic-method <https://docs.astral.sh/ruff/rules/undocumented-magic-method/>`__, `undocumented-public-nested-class <https://docs.astral.sh/ruff/rules/undocumented-public-nested-class/>`__, `undocumented-public-init <https://docs.astral.sh/ruff/rules/undocumented-public-init/>`__.
- Add these `flake8-tidy-imports <https://docs.astral.sh/ruff/settings/#lintflake8-tidy-imports>`__ settings.

.. code-block:: toml

    [tool.ruff]
    preview = true

    [tool.ruff.lint]
    select = ["ALL"]
    ignore = ["D1"]

    [tool.ruff.lint.flake8-tidy-imports]
    ban-relative-imports = "parents"

    [tool.ruff.lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"

With ``docstring-style = "rst"`` in ``[tool.simplint]``, also ignore the Ruff rules that read the ``:returns:``,
``:yields:`` and ``:raises:`` fields as missing sections: `docstring-missing-returns <https://docs.astral.sh/ruff/rules/docstring-missing-returns/>`__ (``DOC201``), `docstring-missing-yields <https://docs.astral.sh/ruff/rules/docstring-missing-yields/>`__ (``DOC402``) and `docstring-missing-exception <https://docs.astral.sh/ruff/rules/docstring-missing-exception/>`__ (``DOC501``).

.. code-block:: toml

    [tool.simplint]
    docstring-style = "rst"

    [tool.ruff.lint]
    ignore = ["D1", "DOC201", "DOC402", "DOC501"]

Test-only bans
--------------

Ruff reads `banned-api <https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_banned-api>`__ per configuration
file, so the test-only bans go in a ``ruff.toml`` inside the tests directory. That file
`extends <https://docs.astral.sh/ruff/settings/#extend>`__ the root configuration.

.. warning::

    Repeat the bans above in that file: a nested table replaces the parent's.

The ``unittest.mock`` ban steers tests from `unittest.mock <https://docs.python.org/3/library/unittest.mock.html>`__ to
the ``mocker`` fixture of `pytest-mock <https://pytest-mock.readthedocs.io/en/latest/>`__:

.. code-block:: toml

    extend = "../pyproject.toml"

    [lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"
    "unittest.mock".msg = "mock through pytest-mock's mocker"
    "time.sleep".msg = "wait on an event the mock sets"
