Command line
============

Every option ``simplint`` accepts, generated from the binary itself.

.. tip::

    Settings that apply to every run belong in ``pyproject.toml``; see :doc:`configuration`.

Options
-------

The options of ``simplint check`` are the ones ``ruff check`` has, with the same meaning:

.. list-table::
    :header-rows: 1
    :widths: 35 65

    * - Options
      - What they do
    * - ``--unsafe-fixes``, ``--diff``, ``--fix-only``, ``--show-fixes``
      - ``--unsafe-fixes`` also applies the fixes that can change behavior. ``--diff`` prints the fixes as a unified
        diff and writes nothing, after the same fix rounds as ``--fix``. ``--fix-only`` fixes without reporting what is
        left. ``--show-fixes`` lists what was fixed.
    * - ``--select``, ``--extend-select``, ``--ignore``
      - Take comma-separated rule codes (``SMP701``), code prefixes (``SMP2``) or rule names
        (``dataclass-frozen``). ``--select`` replaces the selection from ``pyproject.toml``.
    * - ``--fixable``, ``--unfixable``, ``--extend-fixable``
      - Pick which of the selected rules may fix.
    * - ``--per-file-ignores``, ``--extend-per-file-ignores``
      - Take ``PATTERN:CODE`` pairs.
    * - ``--output-format``, ``--statistics``
      - ``--output-format`` picks one of Ruff's formats: ``full``, ``concise``, ``grouped``, ``json``,
        ``json-lines``, ``junit``, ``github``, ``gitlab``, ``pylint``, ``rdjson``, ``azure`` or ``sarif``.
        ``--statistics`` counts the reports per rule.
    * - ``--watch``
      - Checks again whenever a Python or TOML file changes.
    * - ``--stdin-filename``
      - Reads standard input and lints it as the file the name gives, ignoring the paths beside it. See
        `Standard input`_.
    * - ``--add-noqa``, ``--add-ignore``
      - ``--add-noqa`` writes a ``# noqa`` comment for every report, and ``--add-ignore`` a
        ``# simplint: ignore[...]`` comment. Both take an optional reason, as in ``--add-noqa="legacy code"``, and both
        refuse standard input with an error.
    * - ``--show-files``, ``--show-settings``
      - ``--show-files`` lists the files a check would cover. ``--show-settings`` prints the settings for the first of
        them.
    * - ``--config``
      - Takes a configuration file or a ``KEY = VALUE`` override. A ``[tool.simplint]`` key, such as
        ``--config "select = ['SMP1']"``, overrides simplint's settings, and any other key Ruff's, such as
        ``--config "target-version = 'py312'"``.
    * - ``--exit-zero``, ``--exit-non-zero-on-fix``, ``--ignore-noqa``, ``--isolated``, ``--color``,
        ``--target-version``, ``--exclude``, ``--extend-exclude``, ``--no-cache``, ``--cache-dir``
      - Behave as in Ruff.
    * - ``--quiet``, ``--silent``, ``--verbose``
      - Set the log level, as in Ruff.
    * - ``--preview``
      - Turns on Ruff's preview mode. simplint has no preview rules, so it only changes the output, which names rules
        instead of showing their codes, and lets simplint's fixes account for Ruff's preview rules.

Standard input
--------------

Without ``--stdin-filename``, simplint reads standard input when ``-`` is the only path. A ``-`` beside other paths is a
path like any other, so it is looked up as a file named ``-`` and reported as E902 when there is none (see `Errors`_).
With ``--stdin-filename``, simplint reads standard input whatever the paths are and lints it as the file the name gives,
so the rules that depend on the file's path apply to it. It ignores the paths beside the name, with a warning:

.. code-block:: console

    $ printf 'x = 1\n' | simplint check --stdin-filename src/app.py a.py --select SMP101 --output-format concise
    warning: Ignoring file a.py in favor of standard input.
    src/app.py:1:1: SMP101 Single-letter name `x`; use a descriptive name
    Found 1 error.

Errors
------

A path the run names that does not exist is reported as E902, Ruff's
`io-error <https://docs.astral.sh/ruff/rules/io-error/>`__ rule. E902 is an error, so the run exits with 1. When E902 is
not selected, the missing path is a warning instead, and the run exits with 0, as in Ruff:

.. code-block:: console

    $ simplint check --output-format concise nope.py
    warning: No Python files found under the given path(s)
    nope.py:1:1: E902 No such file or directory (os error 2)
    Found 1 error.

A fix that cannot be written, such as one to a read-only file, leaves the file as it was. The reports it had stay, and
E902 reports the failed write beside them, with no fix counted:

.. code-block:: console

    $ simplint check --fix --output-format concise --select SMP001 ro_sec.py
    ro_sec.py:1:1: E902 writing /srv/app/ro_sec.py: Permission denied (os error 13)
    ro_sec.py:1:1: SMP001 Section comment groups code; extract a named function or class instead
    Found 2 errors.
    [*] 1 fixable with `--fix`.

Other commands
--------------

As in Ruff, simplint has commands besides ``check``. All but ``clean`` take ``--output-format json``.

.. list-table::
    :header-rows: 1
    :widths: 35 65

    * - Command
      - What it does
    * - ``simplint rules``
      - Lists every rule and whether it has a fix.
    * - ``simplint rule SMP701``
      - Prints the documentation of one rule. ``simplint rule --all`` prints it for every rule.
    * - ``simplint config``
      - Describes the ``[tool.simplint]`` options.
    * - ``simplint linter``
      - Lists the rule groups.
    * - ``simplint clean``
      - Removes its caches from the ``.ruff_cache`` directories under the current directory.
    * - ``simplint version``
      - Prints the version.

.. sphinx_argparse_cli::
    :module: simplint_cli
    :func: build_parser
    :prog: simplint
    :title:
