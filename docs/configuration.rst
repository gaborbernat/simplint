Settings
========

simplint takes two kinds of settings:

.. list-table::
    :header-rows: 1
    :widths: 35 65

    * - Settings
      - Where they come from
    * - Which files to check, which Python version to assume
      - `Ruff <https://docs.astral.sh/ruff/>`__'s configuration, found and merged exactly as ``ruff check`` finds it.
        See :ref:`settings-shared-with-ruff`.
    * - simplint's own settings
      - The ``[tool.simplint]`` table of
        `pyproject.toml <https://packaging.python.org/en/latest/specifications/pyproject-toml/>`__.

Where ``[tool.simplint]`` comes from
------------------------------------

For each file, simplint uses the **nearest** ``pyproject.toml`` **with a** ``[tool.simplint]`` **table**, in the file's
directory or a parent directory. That directory is the project root, and the paths in the settings below are relative
to it.

.. mermaid::

    flowchart TD
        start["File to check"] --> iso{"--isolated?"}
        iso -->|yes| defaults["Defaults"]
        iso -->|no| cfg{"--config PATH?"}
        cfg -->|yes| named{"Named<br/>pyproject.toml?"}
        named -->|yes| path["Its [tool.simplint],<br/>for every file"]
        named -->|no| defaults
        cfg -->|no| walk{"Nearest pyproject.toml<br/>with [tool.simplint]?"}
        walk -->|found| table["That table"]
        walk -->|none| defaults
        defaults --> over["--config KEY = VALUE<br/>on top"]
        path --> over
        table --> over
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class start blue
        class iso,cfg,named,walk yellow
        class defaults,path,table,over green

Details of each step:

- As Ruff does with ``[tool.ruff]``, simplint passes over a ``pyproject.toml`` without the table, so a nested package
  inherits its parent's settings.

- When no ``pyproject.toml`` has the table, simplint runs every rule with the defaults, and the project metadata comes
  from the nearest ``pyproject.toml`` that describes a project, as below.

- ``--config PATH`` replaces per-directory discovery with one file for every file checked. Ruff reads its settings
  from that file. simplint reads its ``[tool.simplint]`` only when the file is a ``pyproject.toml``; with a
  ``ruff.toml`` or any other name, simplint uses its defaults. A ``pyproject.toml`` named this way also gives the
  project metadata for every file.

- ``--isolated`` ignores every configuration file, as in Ruff.

- ``--config "KEY = VALUE"`` overrides one setting on top of every configuration file: a ``[tool.simplint]`` key
  overrides simplint's setting, and any other key Ruff's.

.. _settings-metadata-source:

Where the project metadata comes from
-------------------------------------

The ``[project]`` table and the build settings describe the project a file belongs to, so they come from a different
``pyproject.toml`` than ``[tool.simplint]`` when a workspace needs it. For each file, simplint uses the **nearest**
``pyproject.toml`` **with a** ``[project]`` **or** ``[build-system]`` **table**, in the file's directory or a parent
directory. Without one, it uses the ``pyproject.toml`` that holds the settings.

A workspace root with ``[tool.simplint]`` but no ``[build-system]`` therefore does not make its members applications: a
member with its own ``[build-system]`` is a library, and a member without a ``[project]`` table inherits the root's.

.. tip::

    ``simplint config`` lists the ``[tool.simplint]`` options, and ``simplint config KEY`` describes one.

The rest of the reference is split by topic:

- :doc:`Running <configuration/running>`: the options of a ``simplint check`` run, and the cache that speeds up the
  next one.
- :doc:`Rule settings <configuration/rule-settings>`: settings that tune individual rules.
- :doc:`Project settings <configuration/project>`: what simplint learns about the project, from its Python version to
  its ``[project]`` table.

Choosing rules, suppressing a report and pairing with Ruff are tasks, covered by the :doc:`how-to guides
</how-to/select-rules>`.

Editor support
--------------

``simplint.schema.json`` in the repository describes the ``[tool.simplint]`` table, so an editor that checks TOML
against a JSON Schema flags unknown keys and wrong values as you type. Editors that read SchemaStore, such as VS Code
with Even Better TOML or Tombi, apply it once SchemaStore lists it. Until then, no editor applies it to
``pyproject.toml`` on its own, because the schema describes the table, not the whole file.

.. toctree::
   :hidden:

   configuration/running
   configuration/rule-settings
   configuration/project

.. _settings-nested-projects:

Nested test projects
--------------------

A library directory whose name only starts with ``test``, such as ``testkit``, is not a project of its own. A directory
of the run whose name is ``test`` or ``tests``, or starts with ``test-`` or ``test_``, or that lies inside one, and that
holds its own ``conf.py``, ``pyproject.toml``, ``setup.py`` or ``setup.cfg``, is a project of its own. Test suites use
these for the small projects they build per test, such as each ``roots/test-*`` directory of Sphinx's test suite. The
directory is the root of its project, so:

- its files are not test modules of the project around them, since the paths of its own root decide that;
- its files are checked with the other files of the same nested project only. A name one of them defines is unused
  when only another nested project uses it, and duplicate code is compared within the project alone, so the copies two
  nested projects make of one another are not reported;
- its ``[tool.simplint]`` is the outer project's, unless a ``pyproject.toml`` at or below the directory has one;
- its metadata and project kind are those of the project around it, unless a ``pyproject.toml`` at or below the
  directory describes a project, which then decides as it does for any project;
- its documentation is the ``.rst`` and ``.md`` pages under the directory, so a name they give is used within it, and
  the pages of the project around it do not reach it, and its pages do not reach the project around it;
- a name the project around it reads, through an import, an attribute, a doctest or a dynamic load, is used within it,
  while a name one nested project reads is not used in the project around it or in another nested project.
- its files are test input for the rules that judge docstrings and the visibility of names: the docstring rules
  (SMP901 to SMP904 and the reStructuredText rules SMP910 to SMP924) and the export and naming visibility rules (SMP102,
  SMP401 to SMP404) skip them, while every other rule checks them as it checks any project.

Only directories below the run's project root count, which is the directory the run starts from, or the one ``--config``
names. A directory outside it is no nested project.

.. code-block:: text

    project/
        pyproject.toml          # [tool.simplint] for the whole project
        roots/
            test-basic/
                conf.py         # a nested project: its own root and index
                parser.py
            test-complex/
                conf.py         # another nested project
                parser.py
