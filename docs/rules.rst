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

A ``from module import *`` hands the importer every name ``__all__`` of ``module`` lists, or without an ``__all__``
every public name, so the index counts each of those as used, and fixes leave them alone. A name another module takes
from a package whose ``__init__.py`` star-imports a submodule counts in that submodule, through any chain of such
re-exports. When the star-imported module does not resolve to a file, as one found only on a ``sys.path`` entry a
``conftest.py`` adds, it may be any file of the run Python could import under its last name, ``name.py`` or
``name/__init__.py``, so the public names of each such file count as used. A module no file of the run is named after
is outside the run.

A fix can change what the index records: folding ``user.uuid = 7`` into ``MagicMock(uuid=7)`` leaves no assignment to
a ``uuid`` attribute, which lets the dataclass with that field freeze. With ``--fix``, simplint collects the facts of
each file a fix rewrote again, and lints and fixes again every file whose answers from the index changed, until a round
rewrites nothing, so one run is final. Fixes that keep undoing each other's facts stop after eight rounds.

.. mermaid::

    flowchart LR
        files["Every Python file in the run"] --> index["Project index:<br/>imported names, assigned attributes,<br/>base classes"]
        entry["pyproject.toml entry points"] --> index
        index --> lint["Each file is linted"]
        lint --> fix["Fixes leave alone names<br/>another module relies on"]
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        classDef teal fill:#8ccbc3,stroke:#145c55,color:#072b28
        classDef orange fill:#f0b48c,stroke:#8a3d12,color:#3d1606
        class files,entry blue
        class index yellow
        class lint teal
        class fix green

Fixes come in two strengths, as in `Ruff <https://docs.astral.sh/ruff/linter/#fix-safety>`__. A **safe** fix keeps
behavior. An **unsafe** fix can change it, so simplint applies it only with ``--unsafe-fixes``. Each rule page says
which kind its fix is. A rule reports without a fix when the fix cannot be done safely in text: the statement shares its
line with another, a string spans several lines, or the new name would collide with an existing one.
