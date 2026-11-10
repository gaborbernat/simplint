simplint and Ruff
#################

simplint is a companion to Ruff. It adds checks that make Python code easier to read and review. When a Ruff rule
reports the same code, simplint leaves that case to Ruff, even if simplint could offer a different fix. A simplint
rule stays when it reports cases Ruff misses. Each rule page names any Ruff overlap.

simplint is a separate program. It ships as one binary inside a Python wheel, like Ruff, and you install it as a tool
apart from the project it lints.

Shared settings
===============

The two tools read the same ``pyproject.toml`` and accept the same ``noqa`` comments. simplint resolves Ruff's
configuration with Ruff's own code, so the Python version and the files to check come out the same for both.

Where they meet
===============

A simplint fix can give a Ruff rule something to report that the file was clean for. simplint then checks the new
report. When Ruff has a fix for it, simplint applies that fix too. When Ruff has none, simplint withdraws its own fix
and keeps the report.

The run order is simplint between Ruff's linter and its formatter, as the :doc:`tutorial </tutorial/getting-started>`
shows. Some rules of the two undo each other's work under some settings, and :doc:`/how-to/resolve-ruff-conflicts`
covers those pairs.

To set up both tools on one project, see :doc:`/how-to/pair-with-ruff`.
