Choose which rules run and fix
##############################

Selecting rules
===============

simplint's own settings live in ``[tool.simplint]``:

.. code-block:: toml

    [tool.simplint]
    select = ["ALL"]                              # the default
    extend-select = []
    ignore = ["SMP901"]
    per-file-ignores = { "tests/*" = ["SMP601"] }
    extend-per-file-ignores = {}

.. list-table::
    :header-rows: 1
    :widths: 30 70

    * - Key
      - Effect
    * - ``select``
      - Enables rules. The default, ``ALL``, enables every rule.
    * - ``extend-select``
      - Adds rules to ``select``.
    * - ``ignore``
      - Removes rules from the selection.
    * - ``per-file-ignores``
      - Maps a glob pattern to the rules to disable in matching files. A pattern matches either the path relative to
        the project root or the bare file name. A leading ``!`` disables the rules in every file the pattern does not
        match.
    * - ``extend-per-file-ignores``
      - Adds to ``per-file-ignores``.

Each entry is a rule code (``SMP601``), a code prefix (``SMP1``) or a rule name (``missing-final``).

.. warning::

    An entry that matches no rule stops the run with an error, and so does a key the table does not know.

When ``select`` and ``ignore`` both match a rule, the **more specific entry wins**, as in Ruff, and ``ignore`` wins a
tie. So ``select = ["SMP6"]`` with ``ignore = ["SMP601"]`` enables every ``SMP6`` rule but ``SMP601``.

On the command line:

- ``--select`` replaces ``select``, ``extend-select`` and ``ignore``.
- ``--extend-select`` adds rules.
- ``--ignore`` removes rules from whatever is selected.
- ``--per-file-ignores`` replaces ``per-file-ignores``, and ``--extend-per-file-ignores`` adds to it.

Fixes
=====

Which fixes apply and how safe they count follows the same pattern:

.. code-block:: toml

    [tool.simplint]
    fixable = ["ALL"]                # the default
    extend-fixable = []
    unfixable = ["SMP701"]
    extend-unfixable = []
    extend-safe-fixes = ["SMP601"]
    extend-unsafe-fixes = ["SMP001"]

.. list-table::
    :header-rows: 1
    :widths: 30 70

    * - Key
      - Effect
    * - ``fixable``
      - Lists the rules whose fixes simplint offers. A rule left out is still reported, without a fix.
    * - ``extend-fixable``
      - Adds to ``fixable``.
    * - ``unfixable``, ``extend-unfixable``
      - Remove from ``fixable``.
    * - ``extend-safe-fixes``
      - Makes the unsafe fixes of the listed rules safe, so ``--fix`` applies them without ``--unsafe-fixes``.
    * - ``extend-unsafe-fixes``
      - Makes safe fixes unsafe.

When both ``extend-safe-fixes`` and ``extend-unsafe-fixes`` list a rule, the more specific entry wins, and
``extend-unsafe-fixes`` wins a tie.

The ``--fixable``, ``--unfixable``, ``--extend-fixable`` and ``--extend-unfixable`` flags override these keys.
