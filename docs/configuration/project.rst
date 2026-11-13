Project settings
################

.. _settings-shared-with-ruff:

Settings shared with Ruff
=========================

simplint resolves `Ruff <https://docs.astral.sh/ruff/>`__'s configuration with Ruff's own code, so the two tools agree
on it. The first of these wins:

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
`requires-python <https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python>`__, then from
the project's other declarations, which Ruff does not read: the ``python`` dependency of
``[tool.poetry.dependencies]``, and the ``python_requires`` of setuptools in ``setup.cfg`` or as a string literal in
``setup.py``. Without any of these, simplint's default is Python 3.11.

Every rule and fix that depends on the language version follows it:

.. list-table::
    :header-rows: 1
    :widths: 25 75

    * - Python
      - What changes
    * - 3.8
      - :doc:`walrus-candidate </rules/SMP502>` offers ``:=`` and :doc:`missing-final </rules/SMP601>` offers ``Final``
        only from here.
    * - 3.9
      - :doc:`single-use-variable </rules/SMP504>` inlines a value into a decorator that is not a dotted name only from
        here.
    * - 3.10
      - :doc:`dataclass-frozen </rules/SMP701>`, :doc:`dataclass-slots </rules/SMP702>` and
        :doc:`dataclass-kw-only </rules/SMP703>` need it. :doc:`up-backport-import </rules/SMP305>` reports
        ``importlib_metadata`` and ``importlib_resources`` imports from here. :doc:`up-traceback-exception-arguments
        </rules/SMP509>` takes the exception alone from here. :doc:`up-deprecated-stdlib-alias </rules/SMP525>` reports
        the camelCase ``threading`` names from here.
    * - 3.11
      - ``except*`` clauses exist only from here, and ``ExceptionGroup.split`` or ``subgroup`` in a handler count as
        catches for :doc:`indistinct-error-class </rules/SMP704>`. :doc:`up-backport-import </rules/SMP305>` reports
        ``tomli``, ``exceptiongroup``, ``async_timeout`` and ``taskgroup`` imports, and :doc:`up-unittest-enter-context
        </rules/SMP812>` offers ``self.enterContext`` from here. :doc:`up-exc-info-exception </rules/SMP510>` offers
        ``sys.exception()`` and :doc:`up-chdir-restore </rules/SMP513>` offers ``contextlib.chdir`` from here.
        :doc:`up-deprecated-stdlib-alias </rules/SMP525>` reports ``unittest.makeSuite`` and the other helpers it
        removes from here.
    * - 3.12
      - The :doc:`string-concatenation </rules/SMP503>` fix reuses the string's quotes inside an f-string only from
        here, and :doc:`single-use-variable </rules/SMP504>` inlines a value that holds the f-string's quote into a
        replacement field as written only from here. Before it, the value's strings take the other quote, where they
        can. Type statements and type parameters are 3.12 syntax: :doc:`module-variable-case </rules/SMP102>` and
        :doc:`module-variable-prefix </rules/SMP404>` skip ``type`` aliases, :doc:`undocumented-public-api
        </rules/SMP903>` and :doc:`rst-unknown-attribute </rules/SMP921>` count them, :doc:`imprecise-annotation
        </rules/SMP602>` checks their bounds, defaults and values, :doc:`definition-before-use </rules/SMP201>` treats
        them as lazy, and :doc:`test-class </rules/SMP801>` moves the type parameters a method uses onto its function,
        and skips a defaulted one until 3.13. :doc:`up-deprecated-stdlib-alias </rules/SMP525>` reports
        ``calendar.January``, ``SafeConfigParser`` and the ``pkgutil`` loaders from here.
    * - 3.13
      - :doc:`up-re-error-alias </rules/SMP518>` offers ``re.PatternError`` from here.
        :doc:`up-warnings-deprecated </rules/SMP524>` offers ``@warnings.deprecated`` from here.
    * - 3.14
      - :doc:`definition-before-use </rules/SMP201>` and :doc:`future-annotations </rules/SMP301>` treat annotations as
        lazy from here, and future-annotations reports no import when a class body binds a name its annotations read.
        :doc:`single-user-constant </rules/SMP202>` and the renames leave names read in t-string interpolations alone.
        :doc:`up-context-var-token-reset </rules/SMP512>` offers ``with`` on a ``ContextVar`` token and
        :doc:`up-strptime-date </rules/SMP514>` offers ``date.strptime`` and ``time.strptime`` from here.
        :doc:`up-compression-modules </rules/SMP523>` offers the ``compression`` imports from here.
    * - 3.15
      - :doc:`module-attribute-import </rules/SMP304>` keeps the ``lazy`` keyword when it rewrites a lazy import, and
        :doc:`init-module-content </rules/SMP407>` accepts ``__lazy_modules__``. :doc:`up-type-checking-lazy-import
        </rules/SMP306>` and :doc:`up-lazy-modules-list </rules/SMP307>` offer ``lazy`` imports from here.
        :doc:`up-argparse-suggest-on-error </rules/SMP519>` drops ``suggest_on_error=True``, the default from here.
        :doc:`up-comprehension-unpacking </rules/SMP508>` unpacks in the element and :doc:`up-object-sentinel
        </rules/SMP511>` offers ``sentinel`` from here. :doc:`up-utf8-encoding-default </rules/SMP521>` drops
        ``encoding="utf-8"`` from here, and :doc:`up-re-prefixmatch </rules/SMP522>` offers ``re.prefixmatch`` from
        here.
    * - any
      - :doc:`dependency-import-guard </rules/SMP303>` counts a module as standard library only when every version from
        the target onward ships it, so a submodule such as ``asyncio.taskgroups`` counts from the version that adds it.
        :doc:`single-use-variable </rules/SMP504>` never inlines a conditional read, an annotation, a ``type`` value, or
        a ``case`` or ``except`` clause. A fix that adds syntax the target version rejects is reverted and reported as a
        bug.

Files to check
--------------

`exclude <https://docs.astral.sh/ruff/settings/#exclude>`__, `extend-exclude
<https://docs.astral.sh/ruff/settings/#extend-exclude>`__, `lint.exclude
<https://docs.astral.sh/ruff/settings/#lint_exclude>`__, `respect-gitignore
<https://docs.astral.sh/ruff/settings/#respect-gitignore>`__ and `force-exclude
<https://docs.astral.sh/ruff/settings/#force-exclude>`__ pick the files to check, Ruff's built-in excludes included. The
command-line flags of the same names override them.

.. _settings-read-from-project:

Settings read from ``[project]``
================================

simplint also reads the `project metadata <https://peps.python.org/pep-0621/>`__ of each file's project, from the
``pyproject.toml`` described in :ref:`settings-metadata-source`:

- `dependencies <https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies>`__ are the
  packages :doc:`dependency-import-guard </rules/SMP303>` treats as required, so it reports a ``try``/``except
  ImportError`` around their imports. A requirement with an `environment marker
  <https://packaging.python.org/en/latest/specifications/dependency-specifiers/#environment-markers>`__ is conditional
  and does not count.
- `Entry points <https://packaging.python.org/en/latest/specifications/entry-points/>`__ in ``scripts``,
  ``gui-scripts`` and ``entry-points`` count as imports of the names they reference. Fixes leave those names in place,
  and :doc:`missing-from-all </rules/SMP401>` expects them in ``__all__``.

.. _settings-public-api:

Public API
==========

``project-kind`` and ``api-packages`` tell simplint which code users import. The documentation rules and the
unused-export rules read them; :doc:`/how-to/library-api` walks through setting them.

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

A module of an API package is **public unless a part of its path starts with** ``_``, as `PEP 8
<https://peps.python.org/pep-0008/#public-and-internal-interfaces>`__ describes. So a layout that keeps its code in
``my_package/_core.py`` and lists the public names in ``my_package/__init__.py`` exposes only those.

``--show-settings`` prints the result as ``linter.project_kind`` and ``linter.api_packages``.
