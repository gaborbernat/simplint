Rules
=====

How it decides
--------------

A few rules need to know more than one file. simplint first indexes the whole run: which names each module imports from
the others, including ``module.attr`` access, dotted strings such as ``mocker.patch("pkg.mod.NAME")``, and the entry
points in ``pyproject.toml``. It also records every attribute name any file assigns and every base class name. A name
another module imports is never renamed or moved, and must appear in ``__all__`` (SMP113). The index covers only the
files in the run, so check the whole project, tests included, before trusting these rules.

Fixes come in two strengths. A **safe** fix keeps behavior. An **unsafe** fix, applied only with ``--unsafe-fixes``,
can change it; each rule page says when its fix is which. A rule reports without a fix when the fix cannot be done
safely in text: the statement shares its line with another, a string spans several lines, or the new name would collide
with an existing one.

Reference
---------

🛠️ marks a rule whose fix is always available and 🔧 one whose fix covers some cases.

.. include:: _generated/rules-table.inc
