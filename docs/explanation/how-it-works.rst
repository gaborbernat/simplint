How simplint works
##################

Some rules need information from other files. A module's exports, for example, can depend on what its callers import.

.. _cross-file-rules:

Cross-file rules
================

Rules marked **🔗 Cross-file** on their pages and ✓ in the rules table can change their findings or fixes in response
to edits elsewhere. Before linting, simplint collects the information the selected rules need. This can include imports
and attribute assignments in Python files, entry points in ``pyproject.toml`` and import lines in ``.pth`` files.

.. mermaid::

    flowchart LR
        files["Every Python file in the run"] --> index["Project index:<br/>imported names, assigned attributes,<br/>base classes"]
        entry["pyproject.toml entry points"] --> index
        pth[".pth import lines"] --> index
        index --> lint["Each file is linted"]
        lint --> fix["Rules check which names<br/>other files use"]
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
and :doc:`unused-export </rules/SMP402>` leaves them alone.

:doc:`missing-from-all </rules/SMP401>` uses this information to find public names missing from ``__all__``.

.. warning::

    The index covers only the files in the run, so check the **whole project, tests included**, before trusting these
    rules.

How module names resolve
------------------------

Each module name resolves to a file with ty's module resolver, the one behind
`ruff analyze graph <https://docs.astral.sh/ruff/settings/#analyze>`__. It searches:

- `Ruff <https://docs.astral.sh/ruff/>`__'s `src <https://docs.astral.sh/ruff/settings/#src>`__ directories
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

With ``--fix`` or ``--diff``, simplint handles these dependencies in rounds.

#. Lint and fix every file.
#. If a round rewrote a file, collect its facts again. Recheck files whose index entries changed and rewritten files
   with findings tied to old source ranges, such as an import cycle.
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
      - May change behavior. By default, applying it requires ``--fix --unsafe-fixes``.

These settings control simplint's own fixes. :doc:`/how-to/select-rules` describes how to change a rule's fix
classification. When cleaning up a new Ruff finding introduced by an edit, simplint can also apply an unsafe Ruff fix.

Depending on the rule, any of the following can prevent a fix.

- the statement shares its line with another
- a string spans several lines
- the new name would collide with an existing one

After each round of fixes, simplint parses the result for the file's target Python version. A fix that adds syntax that
version does not accept is reverted and reported as a bug in the fix, the same as a fix that breaks the syntax.
