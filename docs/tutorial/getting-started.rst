Getting started
###############

Install
=======

simplint ships as a single binary inside a Python wheel, like Ruff, so it needs no Python at runtime. Install it as a
tool with `uv <https://docs.astral.sh/uv/concepts/tools/>`__ or `pipx <https://pipx.pypa.io/stable/>`__, which keeps it
apart from the project it lints.

.. tab:: uv

    .. code-block:: bash

        uv tool install simplint
        simplint --help

.. tab:: pipx

    .. code-block:: bash

        python -m pip install pipx-in-pipx --user
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

``simplint check`` works like `ruff check <https://docs.astral.sh/ruff/linter/>`__:

- It finds the same files and honors the same ``exclude`` settings and
  `.gitignore <https://git-scm.com/docs/gitignore>`__.
- It lints every ``.py`` file under the paths you give it, the current directory by default. ``-`` reads one file from
  standard input.
- It prints in the same formats. By default each report shows the offending code, as ``ruff check`` does.
  ``--output-format concise`` prints one line per report, and ``[*]`` marks a report with a safe fix.
- It exits the same way: status 1 while any report remains, and status 2 when it cannot run, such as on invalid
  configuration.

Read a report
=============

Each report names the file, the line and column, the rule and what to change. This is one report from the run on
the home page, in the concise format:

.. code-block:: console

    src/tox/util/path.py:16:1: SMP601 `_CACHEDIR_TAG` is never reassigned; annotate it `Final`

The rule's page in the :doc:`rules reference </rules>` explains the check and its fix.

Apply the safe fixes
====================

To apply the safe fixes, add ``--fix``. Run simplint between Ruff's `linter <https://docs.astral.sh/ruff/linter/>`__
and its `formatter <https://docs.astral.sh/ruff/formatter/>`__:

.. code-block:: bash

    ruff check --fix .
    simplint check --fix .
    ruff format .

**One round is final**: running the three again changes nothing. simplint leaves layout to ``ruff format``, so its
output need not be formatted, only something one ``ruff format`` run finishes.

A simplint fix can give one of the project's Ruff rules something to report that the file was clean for. simplint
then makes sure ``ruff check`` has no new work:

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

Every rule runs by default. To turn one off, list it in the ``[tool.simplint]`` table of ``pyproject.toml``. This
example turns off :doc:`missing-final </rules/SMP601>`, the rule behind the ``Final`` report above:

.. code-block:: toml

    [tool.simplint]
    ignore = ["missing-final"]

Run ``simplint check .`` again. The reports of that rule are gone, and the others stay.

Next steps
==========

- :doc:`/how-to/select-rules` changes which rules run and which fixes apply.
- :doc:`/how-to/suppress-a-report` silences one report and records why.
- :doc:`/rules` documents every rule, with examples and fix safety.
- :doc:`/configuration` documents every setting.
