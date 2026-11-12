Design principles
#################

These are the rules simplint's own rules follow. They decide what a new rule may report, when it may fix, and how it
shares a project with `Ruff <https://docs.astral.sh/ruff/>`__.

Opinionated, but objective where it can be
==========================================

The rules reflect the maintainer's preferences for code that is easy to read and review, while aiming to be as objective
as possible. Select the rules that fit your project; :doc:`/how-to/select-rules` shows how.

Fix what is mechanical
======================

When the fix for a report is mechanical, the rule offers it. A fix that could change behavior is marked unsafe and runs
only with ``--unsafe-fixes``. A fix never renames or moves a name another module uses: cross-file rules ask the project
index first, as :doc:`/explanation/how-it-works` describes.

A companion to Ruff, never a second copy of it
==============================================

simplint runs next to Ruff, not instead of it. A report both tools give is noise, so **simplint does not report what a
Ruff rule reports**. Where a Ruff rule covers part of what a simplint rule checks, the simplint rule leaves that part
out and reports only the rest. It replicates the Ruff rule's conditions, so Ruff and simplint together still report
everything the simplint rule would report alone.

This holds whatever your Ruff configuration selects:

- **Preview rules count.** A Ruff rule still in preview covers its cases just as a stable rule does, and where a Ruff
  rule reports more in preview, simplint leaves the preview cases to it too.
- **Selection does not matter.** simplint drops the overlap even when your project leaves the Ruff rule out. To have
  those cases reported, select the Ruff rule; :doc:`/how-to/pair-with-ruff` gives a configuration that does.
- **Ruff's settings do.** Where a Ruff rule's settings change what it reports, such as pep8-naming's ``ignore-names``
  or flake8-tidy-imports' ``banned-api``, simplint reads them from the same ``pyproject.toml`` and leaves exactly the
  cases Ruff reports under them.

A rule page's **Ruff overlap** section names the Ruff rules that take part of its cases.

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

What is not an overlap
----------------------

A Ruff rule that reports the same code for a different problem does not overlap: both reports stand, and the rule page
says how they relate. When a simplint fix and a Ruff fix undo each other, the pair is a conflict, not an overlap;
:doc:`/how-to/resolve-ruff-conflicts` lists those pairs and the setting that settles each.

Syntax errors are Ruff's
------------------------

Ruff reports every syntax error whatever its configuration selects, so simplint does not. A file that does not parse
gets a warning that simplint skipped it, and does not change the exit status.
