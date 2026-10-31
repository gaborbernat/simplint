.. image:: https://raw.githubusercontent.com/gaborbernat/simplint/main/docs/_static/wordmark-light.png
   :alt: simplint
   :width: 420

simplint
========

``simplint`` is a companion linter for `Ruff <https://docs.astral.sh/ruff/>`__. It adds checks that make Python code
easier to read and review: `definitions ordered like a book
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP020.html>`__, `explicit exports
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP103.html>`__, `Final constants
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP101.html>`__, `flat test modules
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP200.html>`__, and more. The rules are opinionated. They
reflect the maintainer's preferences while aiming to be as objective as possible, so select the ones that fit your
project.

simplint is written in `Rust <https://rust-lang.org/>`__ on top of Ruff's parser crates, and lints the 264 files of
`tox-dev/tox <https://github.com/tox-dev/tox>`_ in about 10 ms. When the fix for a report is mechanical, it can apply it for you. The
`release history <https://github.com/gaborbernat/simplint/releases>`__ lists the changes in each version.

A run over two tox modules, limited to two rules:

.. code-block:: console

    $ simplint check --output-format concise --select SMP050,SMP101 src/tox/util/path.py src/tox/util/spinner.py
    src/tox/util/path.py:16:1: SMP050 `_CACHEDIR_TAG` is used by one function only; define it there
    src/tox/util/path.py:16:1: SMP101 `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:41:1: SMP101 `MISS_DURATION` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:181:1: SMP050 `_PERIODS` is used by one function only; define it there
    src/tox/util/spinner.py:181:1: SMP101 `_PERIODS` is never reassigned; annotate it `Final`
    Found 5 errors.
    4 unsafe fixes available with `--fix --unsafe-fixes`.

Install
-------

simplint ships as a single binary inside a Python wheel, like Ruff, so it needs no Python at runtime. Install it as a
tool with `uv <https://docs.astral.sh/uv/concepts/tools/>`__ or `pipx <https://pipx.pypa.io/stable/>`__, which keeps it
apart from the project it lints.


    .. code-block:: bash

        uv tool install simplint
        simplint --help

Quick start
-----------

Run simplint from the project root. Several rules need to know which names other modules import, so check the whole
project, tests included:

.. code-block:: bash

    simplint check .

``simplint check`` works like `ruff check <https://docs.astral.sh/ruff/linter/>`__: it finds the same files, honors
the same ``exclude`` settings and `.gitignore <https://git-scm.com/docs/gitignore>`__, prints in the same formats and
exits the same way. It lints every ``.py`` file under the paths you give it, the current directory by default, and
``-`` reads one file from standard input. By default each report shows the offending code, as ``ruff check`` does;
``--output-format concise`` prints one line per report, and ``[*]`` marks a report with a safe fix. The command exits
with status 1 while any report remains, and with status 2 when it cannot run, such as on invalid configuration.

To apply the safe fixes, add ``--fix``. Run simplint between Ruff's `linter <https://docs.astral.sh/ruff/linter/>`__
and its `formatter <https://docs.astral.sh/ruff/formatter/>`__:

.. code-block:: bash

    ruff check --fix .
    simplint check --fix .
    ruff format .

One round is final: running the three again changes nothing. simplint leaves layout to ``ruff format``, so its output
need not be formatted, only something one ``ruff format`` run finishes. When a fix gives one of the project's Ruff rules
something to report that the file was clean for, simplint applies Ruff's own fix for it, as for an import left unused;
when Ruff has no fix, simplint withdraws its own fix and keeps the report.

The options of ``simplint check`` are the ones ``ruff check`` has, with the same meaning:

- ``--unsafe-fixes`` also applies the fixes that can change behavior; ``--diff`` prints the fixes as a unified diff
  and writes nothing; ``--fix-only`` fixes without reporting what is left; ``--show-fixes`` lists what was fixed.
- ``--select``, ``--extend-select`` and ``--ignore`` take comma-separated rule codes (``SMP111``), code prefixes
  (``SMP2``) or rule names (``dataclass-options``). ``--select`` replaces the selection from ``pyproject.toml``.
  ``--fixable``, ``--unfixable`` and ``--extend-fixable`` pick which of the selected rules may fix, and
  ``--per-file-ignores`` and ``--extend-per-file-ignores`` take ``PATTERN:CODE`` pairs.
- ``--output-format`` picks one of Ruff's formats: ``full``, ``concise``, ``grouped``, ``json``, ``json-lines``,
  ``junit``, ``github``, ``gitlab``, ``pylint``, ``rdjson``, ``azure`` or ``sarif``. ``--statistics`` counts the
  reports per rule.
- ``--watch`` checks again whenever a Python or TOML file changes. ``--add-noqa`` writes a ``# noqa`` comment for
  every report, and ``--add-ignore`` a ``# simplint: ignore[...]`` comment; both take an optional reason, as in
  ``--add-noqa="legacy code"``. ``--show-files`` lists the files a check would cover and ``--show-settings`` prints
  the settings for the first of them.
- ``--config`` takes a configuration file or a ``KEY = VALUE`` override. A ``[tool.simplint]`` key, such as
  ``--config "select = ['SMP1']"``, overrides simplint's settings, and any other key Ruff's, such as
  ``--config "target-version = 'py312'"``.
- ``--exit-zero``, ``--exit-non-zero-on-fix``, ``--ignore-noqa``, ``--isolated``, ``--color``, ``--target-version``,
  ``--exclude``, ``--extend-exclude``, ``--no-cache``, ``--cache-dir`` and the ``--quiet``, ``--silent`` and
  ``--verbose`` log levels behave as in Ruff.
- ``--preview`` turns on Ruff's preview mode. simplint has no preview rules, so it only changes the output, which names
  rules instead of showing their codes, and lets simplint's fixes account for Ruff's preview rules.


``simplint rules`` lists every rule and whether it has a fix. As in Ruff, ``simplint rule SMP111`` prints the
documentation of one rule and ``simplint rule --all`` of every rule, ``simplint config`` describes the
``[tool.simplint]`` options, ``simplint linter`` lists the rule groups, ``simplint clean`` removes its caches from the
``.ruff_cache`` directories under the current directory and ``simplint version`` prints the version. All but
``clean`` take ``--output-format json``.

Pre-commit
----------

Add the hook to the ``.pre-commit-config.yaml`` of `pre-commit <https://pre-commit.com/>`__ and set ``rev`` to a
`release tag <https://github.com/gaborbernat/simplint/releases>`__:

.. code-block:: yaml

    - repo: https://github.com/gaborbernat/simplint
      rev: ""
      hooks:
        - id: simplint

The hook runs ``simplint check --fix`` on the whole project rather than on the changed files, because several rules need
to know which names other modules import. List it after Ruff's ``ruff-check`` hook and before its ``ruff-format`` hook,
the order above. It is a `Rust hook <https://pre-commit.com/#rust>`__: pre-commit builds simplint from source with
Cargo, so the first run takes longer than later ones.

The `configuration reference <https://simplint.readthedocs.io/en/latest/configuration.html>`__ covers settings and
suppression comments. The `rules reference <https://simplint.readthedocs.io/en/latest/rules.html>`__ documents every
rule with examples and fix safety.

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
