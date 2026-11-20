simplint and Ruff
#################

simplint adds checks for Python code that Ruff leaves to reviewers. It generally defers to Ruff when their rules
overlap. See :doc:`/explanation/design-principles` for exceptions, including cases where only simplint can offer a fix.

Shared settings
===============

simplint uses `Ruff <https://docs.astral.sh/ruff/>`__'s configuration resolver for file exclusions and settings such
as the Python target version. It also reads its own ``[tool.simplint]`` table. See :doc:`/configuration/project` for
version inference and :doc:`/how-to/suppress-a-report` for the suppression comments the tools recognize.

Where they meet
===============

Run ``ruff check --fix``, then ``simplint check --fix``, then ``ruff format``. When a simplint fix introduces a
finding from an enabled Ruff rule, simplint tries Ruff's fix. If that leaves new findings, simplint can withdraw its
own fix and still report the original problem.

Some rule pairs require a configuration change before simplint will run. See :doc:`/how-to/resolve-ruff-conflicts`.

To set up both tools on one project, see :doc:`/how-to/pair-with-ruff`.
