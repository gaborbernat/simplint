Getting started
###############

Install
=======

The Python wheel requires Python 3.11 or later. It contains a standalone binary, so the Python version you install it
with is independent of the version your project targets. Install it in an isolated tool environment with
`uv <https://docs.astral.sh/uv/concepts/tools/>`__ or `pipx <https://pipx.pypa.io/stable/how-to/install-pipx.html>`__.

.. tab:: uv

    .. code-block:: bash

        uv tool install simplint
        simplint --help

.. tab:: pipx

    .. code-block:: bash

        pipx install simplint
        simplint --help

.. tab:: pip

    .. code-block:: bash

        python -m pip install --user simplint
        simplint --help

    .. note::

        A `user installation <https://pip.pypa.io/en/stable/user_guide/#user-installs>`__ shares the interpreter's
        package set. Prefer ``uv`` or ``pipx`` when an operating system or package manager owns that interpreter.

Check a project
===============

Run simplint from the project root:

.. code-block:: bash

    simplint check .

.. important::

    Check the **whole project, tests included**. Several rules need to know which names other modules import.

simplint uses Ruff's file discovery and honors its exclusions and
`.gitignore <https://git-scm.com/docs/gitignore>`__ settings. Under a directory it lints ``.py`` files and indexes
``.pyi`` stubs. It skips notebooks, Markdown and TOML files. With no paths, it checks the current directory.
Use ``-`` as the only path to check standard input.

By default each report shows the offending code. Use ``--output-format concise`` for one line per report.
A normal check exits with 0 when it finds nothing, 1 when reports remain, or 2 when it cannot run.

Read a report
=============

Each report gives the file, line, column, rule and suggested change. For example, a file named ``module.py`` containing
``x = 1`` produces this report with ``--select SMP101 --output-format concise``:

.. code-block:: console

    module.py:1:1: SMP101 Single-letter name `x`; use a descriptive name

The rule's page in the :doc:`rules reference </rules>` explains the check and its fix.

Apply the safe fixes
====================

To apply the safe fixes, add ``--fix``. Run simplint between Ruff's `linter <https://docs.astral.sh/ruff/linter/>`__
and its `formatter <https://docs.astral.sh/ruff/formatter/>`__:

.. code-block:: bash

    ruff check --fix .
    simplint check --fix .
    ruff format .

simplint leaves formatting to Ruff. If a simplint fix introduces a report from an enabled Ruff rule, simplint tries
Ruff's fix. When it cannot clear the new report, it withdraws the simplint fix.

.. mermaid::

    flowchart TD
        fix["simplint fix"] --> new{"New Ruff report?"}
        new -->|no| keep["Keep the fix"]
        new -->|yes| has{"Ruff has a fix?"}
        has -->|yes| ruff["Apply Ruff's fix too"]
        has -->|no| withdraw["Withdraw the fix,<br/>keep the report"]
        keep --> format["ruff format"]
        ruff --> format
        withdraw --> format
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class fix,format blue
        class new,has yellow
        class keep,ruff,withdraw green

An import a fix leaves unused is the typical case: Ruff's own fix removes it.

Turn a rule off
===============

To turn a rule off, list it under ``ignore`` in the ``[tool.simplint]`` table of ``pyproject.toml``. This example
turns off :doc:`single-letter-name </rules/SMP101>`, the rule behind the report above:

.. code-block:: toml

    [tool.simplint]
    ignore = ["single-letter-name"]

Run ``simplint check .`` again. The reports of that rule are gone, and the others stay.

Next steps
==========

- :doc:`/how-to/select-rules` changes which rules run and which fixes apply.
- :doc:`/how-to/suppress-a-report` silences one report and records why.
- :doc:`/rules` documents every rule, with examples and fix safety.
- :doc:`/configuration` documents every setting.
