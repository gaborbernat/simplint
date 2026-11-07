.. _resolve-ruff-conflicts:

Resolve conflicts with Ruff
###########################

Some simplint rules undo what a Ruff rule asks for, under some Ruff settings. With both on for a file, one side's fix
undoes the other's, or leaves code the other reports. No run of ``ruff check --fix`` and ``simplint check --fix`` then
leaves that file clean.

simplint checks every file's settings before it lints any, and **exits with status 2** when it finds such a pair. The
error names:

- both rules, and whether their fixes undo each other or one only reports what the other removes
- why they clash
- how to resolve it, the recommended way first
- the configuration files their settings come from
- the first file they meet in

.. code-block:: text

    simplint failed
      Cause: simplint and Ruff rules work against each other, so fixing never leaves the code clean:
      - SMP301 (future-annotations) and Ruff's I002 (missing-required-import) undo each other's fixes.
        Why: SMP301 removes `from __future__ import annotations` on Python 3.14+; I002 adds it back because `lint.isort.required-imports` lists it.
        Fix: drop the import from `required-imports`, or ignore SMP301 in `[tool.simplint]`.
        simplint settings: /project/pyproject.toml; Ruff settings: /project/pyproject.toml; first file: src/app.py

The pairs are:

.. list-table::
    :header-rows: 1
    :widths: 20 25 55

    * - simplint
      - Ruff
      - Conflict and resolution
    * - :doc:`function-docstring </_generated/rules/SMP901>`
      - `undocumented-public-class <https://docs.astral.sh/ruff/rules/undocumented-public-class/>`__, `undocumented-public-method <https://docs.astral.sh/ruff/rules/undocumented-public-method/>`__, `undocumented-public-function <https://docs.astral.sh/ruff/rules/undocumented-public-function/>`__, `undocumented-magic-method <https://docs.astral.sh/ruff/rules/undocumented-magic-method/>`__, `undocumented-public-nested-class <https://docs.astral.sh/ruff/rules/undocumented-public-nested-class/>`__, `undocumented-public-init <https://docs.astral.sh/ruff/rules/undocumented-public-init/>`__
      - :doc:`function-docstring </_generated/rules/SMP901>` removes the docstrings these ask for. Ignore them, and let :doc:`undocumented-public-api </_generated/rules/SMP903>` ask
        for documentation on the public API.
    * - :doc:`test-module-docstring </_generated/rules/SMP902>`
      - `undocumented-public-module <https://docs.astral.sh/ruff/rules/undocumented-public-module/>`__, `undocumented-public-package <https://docs.astral.sh/ruff/rules/undocumented-public-package/>`__
      - :doc:`test-module-docstring </_generated/rules/SMP902>` removes the module docstrings of tests these ask for. Turn them off for test files in Ruff's
        ``per-file-ignores``.
    * - :doc:`future-annotations </_generated/rules/SMP301>`
      - `missing-required-import <https://docs.astral.sh/ruff/rules/missing-required-import/>`__
      - :doc:`future-annotations </_generated/rules/SMP301>` removes ``from __future__ import annotations`` on Python 3.14 and newer, which `missing-required-import <https://docs.astral.sh/ruff/rules/missing-required-import/>`__ adds back when
        ``lint.isort.required-imports`` lists it. Drop it from the list once the project targets Python 3.14.
    * - :doc:`up-utf8-encoding-default </_generated/rules/SMP521>`
      - `unspecified-encoding <https://docs.astral.sh/ruff/rules/unspecified-encoding/>`__
      - :doc:`up-utf8-encoding-default </_generated/rules/SMP521>` removes ``encoding="utf-8"`` on Python 3.15 and newer, which `unspecified-encoding <https://docs.astral.sh/ruff/rules/unspecified-encoding/>`__ adds back.
        Ignore one of the two.
    * - :doc:`up-unittest-assert-helper </_generated/rules/SMP811>`
      - `pytest-unittest-assertion <https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/>`__
      - :doc:`up-unittest-assert-helper </_generated/rules/SMP811>` rewrites ``self.assertTrue`` and ``self.assertFalse`` into a ``TestCase`` helper, while `pytest-unittest-assertion <https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/>`__ asks for a bare ``assert``.
        Ignore one of the two.
    * - :doc:`absolute-sibling-import </_generated/rules/SMP302>`
      - `relative-imports <https://docs.astral.sh/ruff/rules/relative-imports/>`__
      - :doc:`absolute-sibling-import </_generated/rules/SMP302>` makes imports from the file's own package relative, which `relative-imports <https://docs.astral.sh/ruff/rules/relative-imports/>`__ turns back when
        ``lint.flake8-tidy-imports.ban-relative-imports`` is ``"all"``. Set it to ``"parents"``.
    * - :doc:`rst-returns </_generated/rules/SMP923>`, :doc:`rst-missing-raises </_generated/rules/SMP924>`
      - `docstring-missing-returns <https://docs.astral.sh/ruff/rules/docstring-missing-returns/>`__, `docstring-missing-yields <https://docs.astral.sh/ruff/rules/docstring-missing-yields/>`__, `docstring-missing-exception <https://docs.astral.sh/ruff/rules/docstring-missing-exception/>`__
      - With ``docstring-style = "rst"``, :doc:`rst-returns </_generated/rules/SMP923>` asks for the ``:returns:`` and ``:yields:`` fields that `docstring-missing-returns <https://docs.astral.sh/ruff/rules/docstring-missing-returns/>`__
        and `docstring-missing-yields <https://docs.astral.sh/ruff/rules/docstring-missing-yields/>`__ read as missing sections, and :doc:`rst-missing-raises </_generated/rules/SMP924>` for the ``:raises:`` fields `docstring-missing-exception <https://docs.astral.sh/ruff/rules/docstring-missing-exception/>`__ reads the same way,
        since those know Google and numpydoc sections only. Ignore the Ruff rules.

Things to know when resolving a conflict:

- Per-file ignores on either side count, so a Ruff rule turned off for tests does not conflict there.
- In preview, as in Ruff's reports, the message names rules instead of codes.
- No setting accepts a conflicting pair; turn one side off.
- ``--show-settings`` and ``--show-files`` do not check for conflicts, so they help find where a rule comes from.

.. note::

    Other combinations clash only on some code, such as :doc:`single-user-constant </_generated/rules/SMP202>` inlining a value into
    a comparison Ruff's `magic-value-comparison <https://docs.astral.sh/ruff/rules/magic-value-comparison/>`__ reports. simplint then offers no fix for that report and keeps it, and the run goes
    on.
