Settings
========

simplint takes two kinds of settings:

.. list-table::
    :header-rows: 1
    :widths: 35 65

    * - Settings
      - Where they come from
    * - Which files to check, which Python version to assume
      - Ruff's configuration, found and merged exactly as ``ruff check`` finds it. See :ref:`settings-shared-with-ruff`.
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

- When no ``pyproject.toml`` has the table, simplint runs every rule with the defaults, and the nearest
  ``pyproject.toml`` still gives the project metadata.

- ``--config PATH`` replaces per-directory discovery with one file for every file checked. Ruff reads its settings
  from that file. simplint reads its ``[tool.simplint]`` only when the file is a ``pyproject.toml``; with a
  ``ruff.toml`` or any other name, simplint uses its defaults.

- ``--isolated`` ignores every configuration file, as in Ruff.

- ``--config "KEY = VALUE"`` overrides one setting on top of every configuration file: a ``[tool.simplint]`` key
  overrides simplint's setting, and any other key Ruff's.

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

.. toctree::
   :hidden:

   configuration/running
   configuration/rule-settings
   configuration/project
