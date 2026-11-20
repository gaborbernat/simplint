Design principles
#################

Rule selection
==============

The rules reflect the maintainer's preferences about reading and reviewing Python code.
:doc:`/how-to/select-rules` explains how to choose which ones your project uses.

Fixes
=====

Rules offer fixes when they can identify a replacement. By default, ``--fix`` applies only fixes classified as safe.
Use ``--unsafe-fixes`` to include fixes that may change behavior. Rules that rename or move definitions can check
where other files use them. :doc:`/explanation/how-it-works` describes what the index covers.

Overlap with Ruff
=================

simplint usually leaves a case to `Ruff <https://docs.astral.sh/ruff/>`__ when both tools detect the same problem.
This includes Ruff's preview rules and usually applies even if your project disables the Ruff rule. To see those
findings, select the corresponding rules in Ruff. The :doc:`pairing guide </how-to/pair-with-ruff>` shows an example.

Where Ruff's settings change which cases it reports, simplint reads those settings too. Examples include pep8-naming's
``ignore-names`` and flake8-tidy-imports' ``banned-api``. Rule pages describe these cases under **Ruff overlap**.

When simplint keeps an overlapping case
---------------------------------------

Four exceptions keep a case in simplint although a Ruff rule reports it too:

- **simplint can fix it and Ruff cannot.** When the Ruff rule has no fix, or only one it shows but never applies, and
  the simplint rule has one, the simplint rule keeps reporting so the fix stays available. An unsafe Ruff fix still
  counts as a fix. The rule page documents the overlap.
- **The recommended setup turns the Ruff rule off.** Some Ruff rules ask for the opposite of what a simplint rule does,
  so :doc:`/how-to/pair-with-ruff` tells you to ignore them. For these, simplint leaves a case to Ruff only while your
  configuration selects the Ruff rule. For example, Ruff's undocumented-* rules (``D1``) ask for docstrings on internal
  code, which :doc:`/rules/SMP901` removes. With ``D1`` ignored, :doc:`/rules/SMP903` reports the undocumented public
  API itself.
- **The Ruff rule reads line breaks that ``ruff format`` moves.** simplint runs before ``ruff format``, which joins
  code that fits on one line. A case that a Ruff rule reports only while it spans lines would then come back to
  simplint on the next run, and ``check --fix`` would not settle in one round. For these, simplint leaves a case to Ruff
  only while your configuration selects the Ruff rule, so ``ruff check --fix`` takes it first. Ruff's COM812 and ISC003
  are such rules, for :doc:`/rules/SMP405` and :doc:`/rules/SMP503`.
- **The Ruff fix breaks code in another file.** Ruff's unused-import (F401) deletes an import that its own file never
  reads, and its safe fix cannot see the module that imports the name. :doc:`/rules/SMP401` keeps its report for such
  an import, since only a check across files sees that importer, and its fix lists the name in ``__all__`` instead. The
  rule page names how to keep Ruff's fix from running first.

Separate findings and conflicts
-------------------------------

Both tools can flag the same line for different problems. If their fixes undo each other, see
:doc:`/how-to/resolve-ruff-conflicts` for the settings that resolve the conflict.

Syntax errors are Ruff's
------------------------

simplint warns when it skips a file that fails to parse. That warning leaves the exit status unchanged.
Run Ruff to see the syntax error.
