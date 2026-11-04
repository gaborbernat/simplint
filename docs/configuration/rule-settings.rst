Rule settings
#############

.. _max-paragraph-words:

Paragraph length
================

:doc:`long-paragraph </_generated/rules/SMP002>` reports a comment paragraph, and :doc:`long-docstring-paragraph </_generated/rules/SMP904>` a docstring paragraph, longer than ``max-paragraph-words`` words:

.. code-block:: toml

    [tool.simplint]
    max-paragraph-words = 100  # the default is 100

``--config "max-paragraph-words = 80"`` replaces the value, and ``--show-settings`` prints it as
``linter.max_paragraph_words``.

.. _naming:

Naming
======

Some frameworks read a module variable by its exact name, so renaming it, to the ``UPPER_CASE`` form that :doc:`module-variable-case </_generated/rules/SMP102>`
asks for or to the underscore prefix that :doc:`module-variable-prefix </_generated/rules/SMP404>` asks for, breaks the program. Both rules already leave alone
the names Sphinx, Alembic, gunicorn and WSGI or ASGI servers read. List any other in ``naming.keep-names``:

.. code-block:: toml

    [tool.simplint.naming]
    keep-names = ["urlpatterns"]  # the default is []

``--config "naming.keep-names = ['urlpatterns']"`` replaces the list, and ``--show-settings`` prints it as
``linter.naming.keep_names``.

.. _module-attributes:

Module attributes
=================

:doc:`module-attribute-import </_generated/rules/SMP304>` asks for a name imported from its module rather than reached through the module, as
``findings.DbWriter`` after ``from . import findings``. A module that tests patch attribute by attribute must stay
reached through the module, so list it, by its absolute dotted name, in ``imports.module-attributes``:

.. code-block:: toml

    [tool.simplint.imports]
    module-attributes = ["pkg.findings"]  # the default is []

``--config "imports.module-attributes = ['pkg.findings']"`` replaces the list, and ``--show-settings`` prints it as
``linter.imports.module_attributes``.

.. _logic:

Logic
=====

:doc:`single-caller-helper </_generated/rules/SMP505>` suggests inlining a private helper that one function calls, when the helper is short and its caller stays
short. Two limits set how short. Statements count as the top-level statements of a body, each nested block counting as
one, and a docstring does not count:

.. code-block:: toml

    [tool.simplint.logic]
    max-helper-statements = 3   # the default is 3
    max-caller-statements = 20  # the default is 20

``--config "logic.max-caller-statements = 30"`` replaces a value, and ``--show-settings`` prints them as
``linter.logic.max_helper_statements`` and ``linter.logic.max_caller_statements``.

.. _docstring-markup:

Docstring markup
================

The :doc:`reStructuredText rules </_generated/groups/docstrings>` check the field lists of `reStructuredText docstrings
<https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists>`__, as Sphinx renders them:

.. code-block:: toml

    [tool.simplint]
    docstring-style = "rst"  # unset by default, which turns the reStructuredText rules off

Every docstring has to be **accurate**:

- no field for a parameter or attribute that does not exist
- no ``:returns:`` on a function that returns nothing, no ``:yields:`` on one that is not a generator
- ``:ivar:`` and ``:cvar:`` for instance and class attributes
- no types, which belong in annotations
- every field needs a description that says more than the name
- a field list needs a blank line above it
- a constructor parameter is documented once, in the class docstring or in ``__init__``
- roles such as ``:class:`` need their target in backticks, and inline literals need closing
- the public API leaves out internal names with a leading underscore

A docstring of the public API also has to be **complete**:

- a ``:param:`` for each parameter
- ``:returns:`` or ``:yields:`` when the function hands back a value
- ``:raises:`` for each exception it raises itself
- an ``:ivar:`` or ``:cvar:`` for each attribute of a class

The fixes add the missing fields with empty descriptions, in the order Sphinx renders them, for you to fill in.

.. note::

    Ruff's `docstring-missing-returns <https://docs.astral.sh/ruff/rules/docstring-missing-returns/>`__, `docstring-missing-yields <https://docs.astral.sh/ruff/rules/docstring-missing-yields/>`__ and `docstring-missing-exception <https://docs.astral.sh/ruff/rules/docstring-missing-exception/>`__ read Google and numpydoc sections only, so they report the
    ``:returns:``, ``:yields:`` and ``:raises:`` fields as missing; see :ref:`resolve-ruff-conflicts`.
