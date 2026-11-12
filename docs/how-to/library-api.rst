Tell simplint what your public API is
#####################################

The documentation rules treat a library's public API apart from its internal code:

- :doc:`undocumented-public-api </rules/SMP903>` asks for docstrings on what users import.
- :doc:`function-docstring </rules/SMP901>` removes them everywhere else.
- :doc:`unused-export </rules/SMP402>` keeps ``__all__`` of the private modules to the names other modules
  use, and :doc:`missing-from-all </rules/SMP401>` lists the types those signatures reach.

An application has no Python API, so all of its code is internal.

.. code-block:: toml

    [tool.simplint]
    project-kind = "library"           # the default is "auto"
    api-packages = ["my_package"]      # the default comes from the build backend

Most projects need none of these settings, since the build backend already names the packages. Set them when simplint
guesses wrong:

1. Check the kind simplint picked with ``simplint check --show-settings``. If it says ``application`` for a library, set
   :ref:`project-kind <library-or-application>` to ``"library"``.
2. If it lists the wrong packages, name the right ones in :ref:`api-packages <api-packages-and-modules>`.
3. Keep the modules that users import from directly public: a name without a leading underscore, such as
   ``my_package/errors.py``, is part of the API, and a private module with a leading underscore is not.

The full resolution order for each setting is in :doc:`/configuration/project`.

Layer private packages
**********************

A module imports private names only from its **direct** private child. When a name lives deeper, each package on the
way up re-exports it, and callers import it from the nearest package:

.. code-block:: text

    my_package/
        b.py              from ._a import what
        _a/
            __init__.py   from ._x import what
            _x.py         def what() -> None: ...

Each package then decides what it exposes one level up, and moving ``_x.py`` inside ``_a`` changes only
``_a/__init__.py``. :doc:`private-import-path </rules/SMP310>` reports any import that reaches further, such as
``from ._a._x import what``, and its message names the re-export that fixes it. Two shortcuts only hide the problem:

- An inline ``# simplint: ignore[private-import-path]`` keeps the deep import, so callers still depend on the layout
  inside ``_a``.
- Flattening ``_a/_x.py`` into ``_a_x.py`` next to its callers removes the layer instead of exposing it once;
  :doc:`flattened-package </rules/SMP311>` reports private modules that share a name prefix this way.

Names used from ``pyproject.toml``
***********************************

simplint counts Python names referenced by these tables as used, even when no Python file imports them:

* ``[project.scripts]``, ``[project.gui-scripts]``, and ``[project.entry-points]`` name installed commands or plugins.
  Legacy ``[tool.poetry.plugins]`` values work the same way.
* ``[build-system] build-backend`` names a local backend and its build hooks. Hatch custom build and metadata hook files
  and ``[tool.coverage.run] plugins`` also name code their hosts call. With ``pdm.backend``, ``pdm_build.py`` or the
  file named by ``[tool.pdm.build] custom-hook`` supplies build hooks.
* ``[tool.setuptools.dynamic]`` values with ``attr`` read a module attribute. Values in ``[tool.setuptools.cmdclass]``
  name command classes.
* ``[tool.mypy] plugins`` loads the named module's ``plugin`` function, or the function after a colon.
  ``[tool.pylint.main]`` ``load-plugins`` loads the named module's ``register`` function.
* ``[tool.pytest.ini_options]`` and ``[tool.pytest]`` ``addopts`` load plugin modules named after ``-p``. Public names
  in those modules may be called by pytest.

For example, a function named only by mypy configuration still counts as used:

.. code-block:: toml

    [tool.mypy]
    plugins = ["my_package.mypy_plugin:custom_plugin"]

These references are resolved against Python files in the project. simplint does not execute configuration, so a
command embedded in an arbitrary script setting does not supply a Python reference. The packaging entry point format
is defined by the `Python Packaging User Guide <https://packaging.python.org/en/latest/specifications/entry-points/>`_;
the tool-specific forms are documented by
`setuptools <https://setuptools.pypa.io/en/latest/userguide/pyproject_config.html#dynamic-metadata>`_,
`mypy <https://mypy.readthedocs.io/en/stable/extending_mypy.html#configuring-mypy-to-use-plugins>`_,
`Pylint <https://pylint.pycqa.org/en/stable/development_guide/how_tos/plugins.html>`_,
`pytest <https://docs.pytest.org/en/stable/how-to/plugins.html>`_, and
`Poetry <https://python-poetry.org/docs/pyproject/#plugins>`_. PDM documents its
`local build hooks <https://backend.pdm-project.org/build_config/#local-build-hooks>`_.
