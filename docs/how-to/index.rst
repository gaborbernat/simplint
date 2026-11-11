How-to guides
#############

Each guide walks through one task, assuming you already have simplint running; the :doc:`/tutorial/getting-started`
tutorial gets you there.

- :doc:`select-rules`: turn rules and their fixes on or off, for the whole project or some files.
- :doc:`suppress-a-report`: silence one report with a comment, and say why.
- :doc:`pair-with-ruff`: set `Ruff <https://docs.astral.sh/ruff/>`__ up so the two linters cover each other.
- :doc:`resolve-ruff-conflicts`: fix a run that stops because a simplint rule and a Ruff rule undo each other.
- :doc:`pre-commit`: run simplint as a pre-commit hook.
- :doc:`library-api`: tell simplint which modules form a library's public API.

.. toctree::
   :hidden:

   Select rules <select-rules>
   Suppress a report <suppress-a-report>
   Pair with Ruff <pair-with-ruff>
   Ruff conflicts <resolve-ruff-conflicts>
   Pre-commit <pre-commit>
   Public API <library-api>
