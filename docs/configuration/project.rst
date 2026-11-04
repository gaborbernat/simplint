Project settings
################

.. _settings-shared-with-ruff:

Settings shared with Ruff
=========================

simplint resolves Ruff's configuration with Ruff's own code, so the two tools agree on it. The first of these wins:

1. ``--isolated``
2. ``--config``
3. the closest ``.ruff.toml``, ``ruff.toml`` or ``pyproject.toml`` with a ``[tool.ruff]`` table
4. your user-level Ruff configuration
5. Ruff's defaults

``extend`` chains and hierarchical configuration work as in Ruff. simplint does not repeat Ruff's warnings about that
configuration, such as an ignored rule Ruff has removed: ``ruff check`` already shows them.

.. _python-version:

Python version
--------------

`target-version <https://docs.astral.sh/ruff/settings/#target-version>`__ and
`per-file-target-version <https://docs.astral.sh/ruff/settings/#per-file-target-version>`__ set the Python version the
rules assume for each file, and ``--target-version`` overrides them. Without them, the version comes from
`requires-python <https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python>`__, and then
from Ruff's default, Python 3.14.

Every rule and fix that depends on the language version follows it:

.. list-table::
    :header-rows: 1
    :widths: 25 75

    * - Python
      - What changes
    * - 3.8
      - :doc:`walrus-candidate </_generated/rules/SMP502>` offers ``:=`` and :doc:`missing-final </_generated/rules/SMP601>` offers ``Final`` only from here.
    * - 3.10
      - :doc:`dataclass-frozen </_generated/rules/SMP701>`, :doc:`dataclass-slots </_generated/rules/SMP702>` and :doc:`dataclass-kw-only </_generated/rules/SMP703>` need it.
    * - 3.12
      - The :doc:`string-concatenation </_generated/rules/SMP503>` fix reuses the string's quotes inside an f-string only from here.
    * - 3.14
      - :doc:`definition-before-use </_generated/rules/SMP201>` and :doc:`future-annotations </_generated/rules/SMP301>` treat annotations as lazy from here.
    * - any
      - :doc:`dependency-import-guard </_generated/rules/SMP303>` counts a module as standard library only when every version from the target onward ships it.

Files to check
--------------

`exclude <https://docs.astral.sh/ruff/settings/#exclude>`__,
`extend-exclude <https://docs.astral.sh/ruff/settings/#extend-exclude>`__,
`lint.exclude <https://docs.astral.sh/ruff/settings/#lint_exclude>`__,
`respect-gitignore <https://docs.astral.sh/ruff/settings/#respect-gitignore>`__ and
`force-exclude <https://docs.astral.sh/ruff/settings/#force-exclude>`__ pick the files to check, Ruff's built-in
excludes included. The command-line flags of the same names override them.

.. _settings-read-from-project:

Settings read from ``[project]``
================================

simplint also reads the `project metadata <https://peps.python.org/pep-0621/>`__:

- `dependencies <https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies>`__ are the
  packages :doc:`dependency-import-guard </_generated/rules/SMP303>` treats as required, so it reports a ``try``/``except ImportError`` around their imports. A
  requirement with an `environment marker
  <https://packaging.python.org/en/latest/specifications/dependency-specifiers/#environment-markers>`__ is conditional
  and does not count.
- `Entry points <https://packaging.python.org/en/latest/specifications/entry-points/>`__ in ``scripts``,
  ``gui-scripts`` and ``entry-points`` count as imports of the names they reference. Fixes leave those names in place,
  and :doc:`missing-from-all </_generated/rules/SMP401>` expects them in
  ``__all__``.

.. _settings-public-api:

Public API
==========

``project-kind``, ``api-packages`` and ``api-modules`` tell simplint which code users import. The documentation rules
and the unused-export rules read them; :doc:`/how-to/library-api` walks through setting them.

.. _library-or-application:

Library or application
----------------------

``project-kind`` is ``"library"`` or ``"application"``. ``"auto"`` picks a library when something installs the project:

.. mermaid::

    flowchart TD
        kind{"project-kind"} -->|library| lib["Library"]
        kind -->|application| app["Application"]
        kind -->|auto| build{"[build-system], setup.py<br/>or setup.cfg?"}
        build -->|no| app
        build -->|yes| bin{"uv package = false or<br/>maturin bindings = bin?"}
        bin -->|yes| app
        bin -->|no| lib
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class kind blue
        class build,bin yellow
        class lib,app green

- A ``[build-system]`` table, or a ``setup.py`` or ``setup.cfg``, installs the project.
- ``[tool.uv]`` with ``package = false``, and maturin building only a binary (``bindings = "bin"``), make it an
  application, as does a project with none of these.
- Neither a ``py.typed`` marker nor console scripts decide it: many libraries ship without the first and with the
  second.

.. _api-packages-and-modules:

API packages and modules
------------------------

``api-packages`` names the top-level packages or modules the library ships. Without it, simplint takes the first of
these sources that names any:

.. mermaid::

    flowchart TD
        set{"api-packages set?"} -->|yes| use["Those packages"]
        set -->|no| backend{"Build backend<br/>names packages?"}
        backend -->|yes| use
        backend -->|no| typed{"Packages with<br/>py.typed?"}
        typed -->|yes| use
        typed -->|no| named{"Package named<br/>after the project?"}
        named -->|yes| use
        named -->|no| all["Every module outside<br/>docs and the tests"]
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class set,backend,typed,named yellow
        class use,all green

1. The build backend's settings: hatch ``packages``, uv's ``module-name``, flit's ``module.name``, setuptools
   ``packages`` and ``py-modules``, maturin's ``module-name``.
2. The packages next to the ``pyproject.toml``, or in its ``src`` directory, that hold a ``py.typed`` marker.
3. The package named after the project.
4. When none is found, every module outside ``docs`` and the tests counts.

``api-modules`` names modules, besides the top-level packages, that users import from directly. Their ``__all__`` is
the interface you declare, so :doc:`unused-export </_generated/rules/SMP402>` keeps the names no module of the project imports.

A module of an API package is **public unless a part of its path starts with** ``_``, as `PEP 8
<https://peps.python.org/pep-0008/#public-and-internal-interfaces>`__ describes. So a layout that keeps its code in
``my_package/_core.py`` and lists the public names in ``my_package/__init__.py`` exposes only those.

``--show-settings`` prints the result as ``linter.project_kind``, ``linter.api_packages`` and ``linter.api_modules``.
