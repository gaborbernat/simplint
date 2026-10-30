Configuration
=============

simplint reads the nearest ``pyproject.toml``. From ``[tool.ruff]`` it takes ``target-version`` (falling back to
``project.requires-python``, then to Python 3.11), ``exclude`` and ``extend-exclude``. Its own settings live in
``[tool.simplint]``:

.. code-block:: toml

    [tool.simplint]
    select = ["ALL"]                              # the default
    extend-select = []
    ignore = ["SMP002"]                           # codes, prefixes or names
    per-file-ignores = { "tests/*" = ["SMP101"] }

From ``[project]`` it reads ``dependencies``, which SMP040 treats as required, and the entry points in ``scripts``,
``gui-scripts`` and ``entry-points``, which count as imports of the names they reference.

Suppress a single line with ``# noqa: SMP010``. Tell Ruff those codes belong to another tool so it does not report them
as unused:

.. code-block:: toml

    [tool.ruff]
    lint.external = ["SMP"]

Command line interface
----------------------

.. literalinclude:: _generated/check-help.txt
    :language: text

Pairing with Ruff
-----------------

simplint does not repeat what Ruff already checks. Run Ruff with ``select = ["ALL"]`` and add this configuration to
cover the rest of the style:

.. code-block:: toml

    [tool.ruff.lint.flake8-tidy-imports]
    ban-relative-imports = "parents"

    [tool.ruff.lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"

Ruff reads ``banned-api`` per configuration file, so the test-only bans go in a ``ruff.toml`` inside the tests
directory. It extends the root configuration and repeats the bans above, since a nested table replaces the parent's:

.. code-block:: toml

    extend = "../pyproject.toml"

    [lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"
    "unittest.mock".msg = "mock through pytest-mock's mocker"
    "time.sleep".msg = "wait on an event the mock sets"
