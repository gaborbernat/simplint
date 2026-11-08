How simplint works
##################

Most rules look at one file at a time. This page covers the parts that look further: the rules that read other files,
and how ``--fix`` keeps going until the code is clean.

.. _cross-file-rules:

Cross-file rules
================

A few rules need to know more than one file. Their pages carry a **🔗 Cross-file** badge and the rules table marks them ✓
under **Cross-file**: an edit to another file can change what they report or fix in this one, and checking a single file
on its own gives them less to go on. Before linting, simplint:

#. indexes every Python file in the run, the entry points in ``pyproject.toml`` and the ``.pth`` files of the
   directories modules resolve from, recording the names each file imports, the attributes it assigns and the base
   classes it names;
#. lints each file, asking the index which of its names other files use;
#. leaves alone, in every fix, a name another module relies on.

.. mermaid::

    flowchart LR
        files["Every Python file in the run"] --> index["Project index:<br/>imported names, assigned attributes,<br/>base classes"]
        entry["pyproject.toml entry points"] --> index
        pth[".pth import lines"] --> index
        index --> lint["Each file is linted"]
        lint --> fix["Fixes leave alone names<br/>another module relies on"]
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        classDef teal fill:#8ccbc3,stroke:#145c55,color:#072b28
        classDef orange fill:#f0b48c,stroke:#8a3d12,color:#3d1606
        class files,entry,pth blue
        class index yellow
        class lint teal
        class fix green

What the index records
----------------------

A name counts as used when another file reaches it through:

- ``from module import name``
- ``module.attr`` access
- dotted strings such as ``mocker.patch("pkg.mod.NAME")`` from
  `pytest-mock <https://pytest-mock.readthedocs.io/en/latest/>`__
- the `entry points <https://packaging.python.org/en/latest/specifications/entry-points/>`__ in ``pyproject.toml``
- the hooks of the build backend that ``build-backend`` names in ``[build-system]``, which a build frontend calls by
  name
- the ``coverage_init`` of each plugin module that ``[tool.coverage.run] plugins`` lists, which coverage calls by name;
  the module is found in the project's directory or in a ``pythonpath`` (pytest) or ``extra-paths`` (ty) directory
- the ``import`` lines of a ``.pth`` file in those directories, which the interpreter runs at startup, and the
  attributes of the modules they import that the lines read
- the names a ``.pyi`` stub declares

The index also records every attribute name any file assigns and every base class name.

A class or alias that the annotations, bases or bounds of an exported name refer to is public too, though no file
imports it. Such names count through the same places of every class and alias they reach, so ``__all__`` may list them,
and :doc:`unused-export </_generated/rules/SMP402>` leaves them alone.

Fixes never rename or move a name another module imports or an exported signature reaches, and
:doc:`missing-from-all </_generated/rules/SMP401>` reports such a public name when ``__all__`` leaves it out.

.. warning::

    The index covers only the files in the run, so check the **whole project, tests included**, before trusting these
    rules.

How module names resolve
------------------------

Each module name resolves to a file with ty's module resolver, the one behind
`ruff analyze graph <https://docs.astral.sh/ruff/settings/#analyze>`__. It searches:

- Ruff's `src <https://docs.astral.sh/ruff/settings/#src>`__ directories
- the directory above each package
- like pytest, the directories above an importing file, so ``tests/helpers.py`` imported as ``tests.helpers`` counts
  even without a ``tests/__init__.py``

Star imports
------------

A ``from module import *`` hands the importer every name ``__all__`` of ``module`` lists, or without an ``__all__``
every public name. The index counts each of those as used, and fixes leave them alone.

A name another module takes from a package whose ``__init__.py`` star-imports a submodule counts in that submodule,
through any chain of such re-exports.

The star-imported module may not resolve to a file, as one found only on a ``sys.path`` entry a ``conftest.py`` adds.
It may then be any file of the run Python could import under its last name, ``name.py`` or ``name/__init__.py``, so the
public names of each such file count as used. A module no file of the run is named after is outside the run.

Fix rounds
==========

A fix can change what the index records. Folding ``user.uuid = 7`` into ``MagicMock(uuid=7)`` leaves no assignment to
a ``uuid`` attribute, which lets the dataclass with that field freeze.

With ``--fix``, simplint therefore works in rounds, so **one run is final**:

#. Lint and fix every file.
#. If a round rewrote a file, collect that file's facts again, then lint and fix the files whose index answers changed.
#. Stop when a round rewrites nothing, or after the eighth round, whose reports then stand.

.. mermaid::

    flowchart TD
        first["Lint and fix every file"] --> rewrote{"Any file rewritten?"}
        rewrote -->|no| done["Done"]
        rewrote -->|yes| facts["Collect the facts of<br/>rewritten files again"]
        facts --> relint["Lint and fix files whose<br/>index answers changed"]
        relint --> limit{"Eighth round?"}
        limit -->|no| rewrote
        limit -->|yes| stop["Stop: last reports stand"]
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class first,facts,relint blue
        class rewrote,limit yellow
        class done,stop green

The eight-round limit stops fixes that keep undoing each other's facts.

.. _fix-safety:

Fix safety
==========

Fixes come in two strengths, as in `Ruff <https://docs.astral.sh/ruff/linter/#fix-safety>`__. Each rule page says which
kind its fix is.

.. list-table::
    :header-rows: 1
    :widths: 20 80

    * - Kind
      - Behavior
    * - ✓ **safe**
      - Keeps behavior. ``--fix`` applies it.
    * - ✗ **unsafe**
      - Can change behavior, so simplint applies it only with ``--unsafe-fixes``.

A rule reports without a fix when the fix cannot be done safely in text:

- the statement shares its line with another
- a string spans several lines
- the new name would collide with an existing one

After each round of fixes, simplint parses the result for the file's target Python version. A fix that adds syntax that
version does not accept is reverted and reported as a bug in the fix, the same as a fix that breaks the syntax.
