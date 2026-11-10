Rule settings
#############

.. _max-paragraph-words:

Paragraph length
================

:doc:`long-paragraph </rules/SMP002>` reports a comment paragraph, and :doc:`long-docstring-paragraph </rules/SMP904>` a docstring paragraph, longer than ``max-paragraph-words`` words:

.. code-block:: toml

    [tool.simplint]
    max-paragraph-words = 100  # the default is 100

``--config "max-paragraph-words = 80"`` replaces the value, and ``--show-settings`` prints it as
``linter.max_paragraph_words``.

.. _naming:

Naming
======

Some frameworks read a module variable by its exact name, so renaming it, to the ``UPPER_CASE`` form that :doc:`module-variable-case </rules/SMP102>`
asks for or to the underscore prefix that :doc:`module-variable-prefix </rules/SMP404>` asks for, breaks the program. Both rules already leave alone
the names Sphinx, Alembic, gunicorn and WSGI or ASGI servers read. List any other in ``naming.keep-names``:

.. code-block:: toml

    [tool.simplint.naming]
    keep-names = ["urlpatterns"]  # the default is []

``--config "naming.keep-names = ['urlpatterns']"`` replaces the list, and ``--show-settings`` prints it as
``linter.naming.keep_names``.

``naming.meaningless-names`` holds the words that :doc:`meaningless-name </rules/SMP103>` reports, such as
``foo`` and ``baz``. ``naming.catch-all-modules`` holds the module and package names that
:doc:`catch-all-module </rules/SMP104>` reports, such as ``utils`` and ``helpers``. Both lists replace their
defaults when set, and ``--show-settings`` prints them as ``linter.naming.meaningless_names`` and
``linter.naming.catch_all_modules``.

.. code-block:: toml

    [tool.simplint.naming]
    meaningless-names = ["foo", "baz", "tmp"]  # the default holds 4 words
    catch-all-modules = ["utils", "helpers"]  # the default holds 7 names

.. _module-attributes:

Module attributes
=================

:doc:`module-attribute-import </rules/SMP304>` asks for a name imported from its module rather than reached through the module, as
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

:doc:`single-caller-helper </rules/SMP505>` suggests inlining a private helper that one function calls, when the helper is short and its caller stays
short. Two limits set how short. :doc:`duplicate-code </rules/SMP530>` reports statements another module
repeats, once the copy is long enough. Statements count as the top-level statements of a body, each nested block
counting as one, and a docstring does not count. :doc:`long-tuple-return </rules/SMP528>` reports a returned
tuple with more elements than its limit, and :doc:`cognitive-complexity </rules/SMP529>` a function whose
score is over its limit:

.. code-block:: toml

    [tool.simplint.logic]
    max-helper-statements = 3      # the default is 3
    max-caller-statements = 20     # the default is 20
    min-duplicate-statements = 6   # the default is 6
    max-tuple-elements = 2         # the default is 2
    max-cognitive-complexity = 20  # the default is 20

``--config "logic.max-caller-statements = 30"`` replaces a value, and ``--show-settings`` prints them as
``linter.logic.max_helper_statements``, ``linter.logic.max_caller_statements``,
``linter.logic.min_duplicate_statements``, ``linter.logic.max_tuple_elements`` and
``linter.logic.max_cognitive_complexity``.

.. _docstring-markup:

Docstring markup
================

The :doc:`reStructuredText rules </groups/docstrings>` check the field lists of `reStructuredText docstrings
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
