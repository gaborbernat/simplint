Tell simplint what your public API is
#####################################

The documentation rules treat a library's public API apart from its internal code:

- :doc:`undocumented-public-api </_generated/rules/SMP903>` asks for docstrings on what users import.
- :doc:`function-docstring </_generated/rules/SMP901>` removes them everywhere else.
- :doc:`unused-export </_generated/rules/SMP402>` keeps ``__all__`` of the private modules to the names other modules
  use, and :doc:`missing-from-all </_generated/rules/SMP401>` lists the types those signatures reach.

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
