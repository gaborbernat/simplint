Rules
=====

Each rule below reports a pattern that makes code harder to read or review, and most can fix it. Filter the table by
code, name or description, or narrow it to a group or to rules with a fix. The address bar keeps the filter, so you can
share a filtered view as a link. To choose which rules run, see :doc:`configuration`.

.. include:: _generated/rules-table.inc

How it decides
--------------

A few rules need to know more than one file. Before linting, simplint indexes every file in the run and records which
names of each file the others use. It counts ``from module import name``, ``module.attr`` access, dotted strings such
as ``mocker.patch("pkg.mod.NAME")`` from `pytest-mock <https://pytest-mock.readthedocs.io/en/latest/>`__, the
`entry points <https://packaging.python.org/en/latest/specifications/entry-points/>`__ in ``pyproject.toml``, and the
names a ``.pyi`` stub declares. Each module name resolves to a file with ty's module resolver, the one behind
`ruff analyze graph <https://docs.astral.sh/ruff/settings/#analyze>`__: it searches Ruff's
`src <https://docs.astral.sh/ruff/settings/#src>`__ directories and the directory above each package, and, like
pytest, the directories above an importing file, so ``tests/helpers.py`` imported as ``tests.helpers`` counts even
without a ``tests/__init__.py``. The index also records every attribute name any file assigns and every base class
name. Fixes never rename or move a
name another module imports, and :doc:`SMP113 <_generated/rules/SMP113>` reports such a public name when
``__all__`` leaves it out. The index covers only the files in the run, so check the whole project, tests included,
before trusting these rules.

.. mermaid::

    flowchart LR
        files["Every Python file in the run"] --> index["Project index:<br/>imported names, assigned attributes,<br/>base classes"]
        entry["pyproject.toml entry points"] --> index
        index --> lint["Each file is linted"]
        lint --> fix["Fixes leave alone names<br/>another module relies on"]
        classDef blue fill:#3776ab,stroke:#062d59,color:#ffffff
        classDef yellow fill:#ffd43b,stroke:#b8930b,color:#062d59
        classDef green fill:#22c55e,stroke:#15803d,color:#052e16
        classDef teal fill:#14b8a6,stroke:#0f766e,color:#042f2e
        classDef orange fill:#f97316,stroke:#c2410c,color:#431407
        class files,entry blue
        class index yellow
        class lint teal
        class fix green

Fixes come in two strengths, as in `Ruff <https://docs.astral.sh/ruff/linter/#fix-safety>`__. A **safe** fix keeps
behavior. An **unsafe** fix can change it, so simplint applies it only with ``--unsafe-fixes``. Each rule page says
which kind its fix is. A rule reports without a fix when the fix cannot be done safely in text: the statement shares its
line with another, a string spans several lines, or the new name would collide with an existing one.
