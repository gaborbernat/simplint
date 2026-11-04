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
        diff and writes nothing. ``--fix-only`` fixes without reporting what is left. ``--show-fixes`` lists what was
        fixed.
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
    * - ``--add-noqa``, ``--add-ignore``
      - ``--add-noqa`` writes a ``# noqa`` comment for every report, and ``--add-ignore`` a
        ``# simplint: ignore[...]`` comment. Both take an optional reason, as in ``--add-noqa="legacy code"``.
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
