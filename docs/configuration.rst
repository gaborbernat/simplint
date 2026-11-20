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

- When no ``pyproject.toml`` has the table, simplint uses the :doc:`default rule selection </how-to/select-rules>`.
  Project metadata comes from the nearest ``pyproject.toml`` that describes a project, as below.

- ``--config PATH`` replaces per-directory discovery with one file for every file checked. Ruff reads its settings
  from that file. simplint reads its ``[tool.simplint]`` only when the file is a ``pyproject.toml``; with a
  ``ruff.toml`` or any other name, simplint uses its defaults. A ``pyproject.toml`` named this way also gives the
  project metadata unless a nested test project declares its own.

- ``--isolated`` skips Ruff and simplint configuration files. Some rules still consult project files. For example,
  simplint can read test collection settings from ``pytest.ini``, ``.pytest.ini``, ``tox.ini`` or ``setup.cfg``.

- ``--config "KEY = VALUE"`` overrides one setting on top of every configuration file: a ``[tool.simplint]`` key
  overrides simplint's setting, and any other key Ruff's.

.. _settings-metadata-source:

Where the project metadata comes from
-------------------------------------

The ``[project]`` table and the build settings describe the project a file belongs to, so they come from a different
``pyproject.toml`` than ``[tool.simplint]`` when a workspace needs it. For each file, simplint uses the **nearest**
``pyproject.toml`` **with a** ``[project]`` **or** ``[build-system]`` **table**, in the file's directory or a parent
directory. Without one, it uses the ``pyproject.toml`` that holds the settings.

For example, a workspace can share one ``[tool.simplint]`` table while each member declares its own build backend.
A member with neither a ``[project]`` nor a ``[build-system]`` table inherits metadata from the nearest parent with
either table.

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

SchemaStore's `pyproject.toml schema <https://www.schemastore.org/pyproject.json>`__ references the
`simplint schema <https://www.schemastore.org/partial-simplint.json>`__. Editors using that schema can flag unknown
keys and invalid values in ``[tool.simplint]``. The repository's ``simplint.schema.json`` describes the options for
this checkout.

.. toctree::
   :hidden:

   configuration/running
   configuration/rule-settings
   configuration/project

.. _settings-nested-projects:

Nested test projects
--------------------

Test suites sometimes contain small projects of their own, such as Sphinx's ``roots/test-*`` fixtures. A directory
qualifies as a nested project if it meets all three conditions.

- It lies below the run's project root.
- It is named ``test`` or ``tests``, starts with ``test-`` or ``test_``, or is inside a directory with one of those
  names. A name such as ``testkit`` does not qualify.
- It contains ``conf.py``, ``pyproject.toml``, ``setup.py`` or ``setup.cfg``.

Within a nested project, paths are relative to its root when deciding which files are tests. It inherits settings
and metadata from the enclosing project unless it has a configuration or metadata table of its own.

Each nested project has its own index for imports, duplicate code and documentation references. An import, attribute
read, doctest or dynamic load in the enclosing project can count as a use of a name in a nested project. Reads in
one nested project do not establish uses in a sibling or parent. References in Markdown and reStructuredText pages
stay within the project containing the page.

Docstring rules SMP901–SMP904 and SMP910–SMP925 skip these fixtures, as do visibility rules SMP102 and SMP401–SMP404.

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
