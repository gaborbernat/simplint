Configuration
=============

simplint takes two kinds of settings. Which files to check and which Python version to assume come from Ruff's
configuration, found and merged exactly as ``ruff check`` finds it. simplint's own settings come from the
``[tool.simplint]`` table of `pyproject.toml <https://packaging.python.org/en/latest/specifications/pyproject-toml/>`__:
for each file, the nearest ``pyproject.toml`` with a ``[tool.simplint]`` table in the file's directory or a parent
directory. As Ruff does with ``[tool.ruff]``, simplint passes over a ``pyproject.toml`` without the table, so a nested
package inherits its parent's settings. That directory is the project root, and the paths in the settings below are
relative to it. When no ``pyproject.toml`` has the table, simplint runs every rule with the defaults, and the nearest
``pyproject.toml`` still gives the project metadata.

``--config PATH`` reads both from one file instead, for every file checked, and ``--isolated`` ignores every
configuration file, as in Ruff. ``--config "KEY = VALUE"`` overrides one setting on top of every configuration file:
a ``[tool.simplint]`` key overrides simplint's setting, and any other key Ruff's. ``simplint config`` lists the
``[tool.simplint]`` options, and ``simplint config KEY`` describes one.

Selecting rules
---------------

simplint's own settings live in ``[tool.simplint]``:

.. code-block:: toml

    [tool.simplint]
    select = ["ALL"]                              # the default
    extend-select = []
    ignore = ["SMP002"]
    per-file-ignores = { "tests/*" = ["SMP101"] }
    extend-per-file-ignores = {}

- ``select`` enables rules. The default, ``ALL``, enables every rule.
- ``extend-select`` adds rules to ``select``.
- ``ignore`` removes rules from the selection.
- ``per-file-ignores`` maps a glob pattern to the rules to disable in matching files. A pattern matches either the path
  relative to the project root or the bare file name, and a leading ``!`` disables the rules in every file the pattern
  does not match. ``extend-per-file-ignores`` adds to it.

Each entry is a rule code (``SMP101``), a code prefix (``SMP1``) or a rule name (``missing-final``). An entry that
matches no rule stops the run with an error, and so does a key the table does not know. As in Ruff, the more specific
entry wins when ``select`` and ``ignore`` both match a rule, so ``select = ["SMP1"]`` with ``ignore = ["SMP101"]``
enables every ``SMP1`` rule but ``SMP101``, and ``ignore`` wins a tie. On the command line, ``--select`` replaces
``select``, ``extend-select`` and ``ignore``, ``--extend-select`` adds rules, and ``--ignore`` removes rules from
whatever is selected. ``--per-file-ignores`` replaces ``per-file-ignores`` and ``--extend-per-file-ignores`` adds to
it.

Fixes
-----

Which fixes apply and how safe they count follows the same pattern:

.. code-block:: toml

    [tool.simplint]
    fixable = ["ALL"]                # the default
    extend-fixable = []
    unfixable = ["SMP111"]
    extend-unfixable = []
    extend-safe-fixes = ["SMP101"]
    extend-unsafe-fixes = ["SMP001"]

- ``fixable`` lists the rules whose fixes simplint offers, ``extend-fixable`` adds to it and ``unfixable`` and
  ``extend-unfixable`` remove from it. A rule left out is still reported, without a fix.
- ``extend-safe-fixes`` makes the unsafe fixes of the listed rules safe, so ``--fix`` applies them without
  ``--unsafe-fixes``; ``extend-unsafe-fixes`` makes safe fixes unsafe. When both list a rule, the more specific entry
  wins, and ``extend-unsafe-fixes`` wins a tie.

The ``--fixable``, ``--unfixable``, ``--extend-fixable`` and ``--extend-unfixable`` flags override these keys.

Run options
-----------

The table also takes the run options Ruff reads from ``[tool.ruff]``, with the same names. The options of the table
found for the current directory apply to the whole run, and the matching command-line flag, or its ``--no-`` form,
overrides them:

.. code-block:: toml

    [tool.simplint]
    fix = false            # apply safe fixes, like --fix
    fix-only = false       # fix and report nothing else, like --fix-only
    unsafe-fixes = false   # include unsafe fixes, like --unsafe-fixes; unset shows a hint instead
    show-fixes = false     # list what was fixed, like --show-fixes
    output-format = "full" # like --output-format

Like Ruff, simplint caches which files had nothing to report, keyed by each file's modification time. The cache lives
in a ``simplint`` directory inside Ruff's cache directory, ``.ruff_cache`` unless Ruff's ``cache-dir`` setting,
``--cache-dir`` or ``RUFF_CACHE_DIR`` names another, and never touches Ruff's own files there; ``--no-cache`` skips it.
A cached result also depends on the names other files use from that file, so editing an importer checks the files it
imports again.

Settings shared with Ruff
-------------------------

simplint resolves Ruff's configuration with Ruff's own code, so the two tools agree on it: ``--isolated``, then
``--config``, then the closest ``.ruff.toml``, ``ruff.toml`` or ``pyproject.toml`` with a ``[tool.ruff]`` table, then
your user-level Ruff configuration, then Ruff's defaults. ``extend`` chains and hierarchical configuration work as in
Ruff.

- `target-version <https://docs.astral.sh/ruff/settings/#target-version>`__ and
  `per-file-target-version <https://docs.astral.sh/ruff/settings/#per-file-target-version>`__ set the Python version
  the rules assume for each file, and ``--target-version`` overrides them. Without them, the version comes from
  `requires-python <https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python>`__, and
  then from Ruff's default, Python 3.14. Every rule and fix that depends on the language version follows it: SMP100
  offers ``:=`` and SMP101 offers ``Final`` only from Python 3.8, SMP111 and SMP114 need 3.10, the SMP112 fix reuses
  the string's quotes inside an f-string only from 3.12, SMP020 and SMP108 treat annotations as lazy from 3.14, and
  SMP040 counts a module as standard library only when every version from the target onward ships it.
- `exclude <https://docs.astral.sh/ruff/settings/#exclude>`__,
  `extend-exclude <https://docs.astral.sh/ruff/settings/#extend-exclude>`__,
  `lint.exclude <https://docs.astral.sh/ruff/settings/#lint_exclude>`__,
  `respect-gitignore <https://docs.astral.sh/ruff/settings/#respect-gitignore>`__ and
  `force-exclude <https://docs.astral.sh/ruff/settings/#force-exclude>`__ pick the files to check, Ruff's built-in
  excludes included, and the command-line flags of the same names override them.

Settings read from ``[project]``
--------------------------------

simplint also reads the `project metadata <https://peps.python.org/pep-0621/>`__:

- `dependencies
  <https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies>`__
  are the packages SMP040 treats as required, so it reports a ``try``/``except ImportError`` around their imports. A
  requirement with an `environment marker
  <https://packaging.python.org/en/latest/specifications/dependency-specifiers/#environment-markers>`__ is conditional
  and does not count.
- `Entry points <https://packaging.python.org/en/latest/specifications/entry-points/>`__ in ``scripts``,
  ``gui-scripts`` and ``entry-points`` count as imports of the names they reference. Fixes leave those names in place,
  and `SMP113 <https://simplint.readthedocs.io/en/latest/_generated/rules/SMP113.html>`__ expects them in
  ``__all__``.

Suppressing a report
--------------------

simplint honors the same `noqa comments <https://docs.astral.sh/ruff/linter/#error-suppression>`__ as Ruff. Put the
comment on the line where the report starts, and give a reason after the codes:

.. code-block:: python

    x = solve(matrix)  # noqa: SMP010 matches the paper's notation

A report on a multi-line string or a line continued with ``\`` takes the comment on the last line of the string or
continuation, as in Ruff. A bare ``# noqa`` suppresses every simplint rule on its line, but
`SMP060 <https://simplint.readthedocs.io/en/latest/_generated/rules/SMP060.html>`__ reports it for giving no reason.

The file-level comments work as in Ruff too. ``# ruff: noqa: SMP010`` (or ``# flake8: noqa: SMP010``) on its own line
suppresses the listed rules in the whole file, a bare ``# ruff: noqa`` every rule but SMP060, and
``# ruff: file-ignore[SMP010]`` at module scope the listed rules.

``# simplint: ignore[SMP010]`` suppresses the listed rules on its line too, the way Ruff's ``# ruff: ignore[...]``
does for Ruff's rules. ``simplint check --add-noqa`` writes a ``# noqa`` comment for every
report left, merged into the line's existing ``# noqa`` comment, and ``--add-ignore`` writes ``# simplint: ignore``
comments the same way. Both take a reason to write after the codes: ``--add-noqa="matches the paper's notation"``.

Ruff does not know the ``SMP`` codes. List them in
`lint.external <https://docs.astral.sh/ruff/settings/#lint_external>`__ so that Ruff keeps these comments instead of
reporting them as unused (`RUF100 <https://docs.astral.sh/ruff/rules/unused-noqa/>`__):

.. code-block:: toml

    [tool.ruff]
    lint.external = ["SMP"]

Pairing with Ruff
-----------------

simplint does not repeat what Ruff already checks. To cover what its rules leave to Ruff, run Ruff with every rule
selected (``select = ["ALL"]``) and add these
`flake8-tidy-imports <https://docs.astral.sh/ruff/settings/#lintflake8-tidy-imports>`__ settings:

.. code-block:: toml

    [tool.ruff.lint.flake8-tidy-imports]
    ban-relative-imports = "parents"

    [tool.ruff.lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"

Ruff reads `banned-api <https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_banned-api>`__ per
configuration file, so the test-only bans go in a ``ruff.toml`` inside the tests directory. That file
`extends <https://docs.astral.sh/ruff/settings/#extend>`__ the root configuration and repeats the bans above, because a
nested table replaces the parent's. The ``unittest.mock`` ban steers tests from
`unittest.mock <https://docs.python.org/3/library/unittest.mock.html>`__ to the ``mocker`` fixture of
`pytest-mock <https://pytest-mock.readthedocs.io/en/latest/>`__:

.. code-block:: toml

    extend = "../pyproject.toml"

    [lint.flake8-tidy-imports.banned-api]
    "typing.Any".msg = "use the precise type"
    "typing.cast".msg = "narrow with isinstance or TypeIs instead"
    "unittest.mock".msg = "mock through pytest-mock's mocker"
    "time.sleep".msg = "wait on an event the mock sets"

.. toctree::
   :hidden:

   cli
