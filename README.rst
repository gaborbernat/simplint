simplint
========

``simplint`` is a companion linter for `astral-sh/ruff <https://github.com/astral-sh/ruff>`_. It enforces the parts of the ``simp`` house style that
Ruff has no rule for, such as definition order, ``Final`` constants, explicit exports and test layout. It is built on
Ruff's own parser crates, so the 264 files of `tox-dev/tox <https://github.com/tox-dev/tox>`_ lint in about 10 ms, and it fixes what it reports
wherever the fix is mechanical. See the `release history <https://github.com/gaborbernat/simplint/releases>`_ for
version changes.

.. code-block:: console

    $ simplint check --select SMP050,SMP101 src/tox/util/path.py src/tox/util/spinner.py
    src/tox/util/path.py:16:1: SMP050 `_CACHEDIR_TAG` is used by one function only; define it there
    src/tox/util/path.py:16:1: SMP101 [*] `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:41:1: SMP101 [*] `MISS_DURATION` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:181:1: SMP050 `_PERIODS` is used by one function only; define it there
    src/tox/util/spinner.py:181:1: SMP101 `_PERIODS` is never reassigned; annotate it `Final`
    Found 5 issues.
    [*] 2 fixable with `--fix`.
    2 hidden fixes can be enabled with `--unsafe-fixes`.

Install
-------

simplint ships as a single binary inside a Python wheel, like Ruff, so it needs no Python at runtime. An isolated tool
environment keeps it apart from the project it lints.


    .. code-block:: bash

        uv tool install simplint
        simplint --help

Pre-commit
----------

Add the hook to ``.pre-commit-config.yaml`` and set ``rev`` to the required release. The hook lints the whole project
on every run, since several rules need to know which names other modules import:

.. code-block:: yaml

    - repo: https://github.com/gaborbernat/simplint
      rev: ""
      hooks:
        - id: simplint

See `pre-commit/pre-commit <https://github.com/pre-commit/pre-commit>`_ for installation and update commands.

Running it
----------

``simplint check [PATHS]`` lints the Python files under each path, honoring ``.gitignore``. Add ``--fix`` to apply safe
fixes, ``--unsafe-fixes`` to also apply fixes that can change behavior, and ``--diff`` to print the fixes without
writing them. ``--select`` and ``--ignore`` take comma-separated rule codes, code prefixes or rule names.
``simplint rules`` lists every rule and whether it fixes, and ``simplint rule SMP111`` prints one rule's documentation.

Fixes rewrite code without formatting it, so run the formatter afterwards, the same way you would after
``ruff check --fix``:

.. code-block:: bash

    simplint check --fix . && ruff check --fix . && ruff format .

See the `configuration reference <https://simplint.readthedocs.io/en/latest/configuration.html>`_ for settings and the
`rules reference <https://simplint.readthedocs.io/en/latest/rules.html>`_ for every rule with examples and fix safety.

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
