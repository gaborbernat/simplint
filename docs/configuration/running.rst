Running
#######

Run options
===========

Set these options in ``[tool.simplint]`` to change how ``simplint check`` runs.

.. code-block:: toml

    [tool.simplint]
    fix = false            # apply safe fixes, like --fix
    fix-only = false       # fix and report nothing else, like --fix-only
    unsafe-fixes = false   # set true to include unsafe fixes, or omit to show a hint
    show-fixes = false     # list what was fixed, like --show-fixes
    output-format = "full" # like --output-format

The settings resolved for the current directory apply to the whole run. Command-line flags override them.
Each boolean flag has a ``--no-`` form. Use ``--output-format`` for the report format.

Cache
-----

simplint caches clean results and, for checks without fixes, can reuse a file's reports.
It also caches import facts used to build the project index. It checks modification time, permissions and size, using
the filesystem's change information to detect edits that preserve all three. On local filesystems with reliable Unix
change times it hashes recently changed files until the clock tick has passed. On Windows it uses the file's change
journal record when available. Other filesystems use a content hash.

With a :ref:`cross-file rule <cross-file-rules>` enabled, a cached result also depends on the names other files use
from that file, so editing an importer checks the files it imports again. Files checked only by per-file rules keep
their cached results.

A failed write cannot be reused because directory permissions may change between runs. During ``--diff``, simplint
keeps edits in memory. Results that depend on those edits are valid only for that run.

Some findings include paths relative to the working directory. Running from a different directory invalidates cached
findings for :doc:`import-cycle </rules/SMP308>`, :doc:`duplicate-code </rules/SMP530>` and
:doc:`undocumented-public-api </rules/SMP903>`. For :doc:`private-import-path </rules/SMP310>`, adding, removing or
renaming a module under the project root or ``src/`` can change what a test may import.

Cached parse failures still produce a warning on each run. A check without fixes can also replay a cached UTF-8
decoding error. If Ruff's ``io-error`` (E902) rule is disabled, simplint logs the read error again on each run.

Where the cache lives:

- In a ``simplint`` directory inside Ruff's cache directory: ``.ruff_cache`` unless Ruff's ``cache-dir`` setting,
  ``--cache-dir`` or ``RUFF_CACHE_DIR`` names another. Ruff and simplint store their results separately.
- Inside the project, when the project picks it. A ``cache-dir`` setting that leads outside falls back to
  ``.ruff_cache``, and a run whose ``.ruff_cache`` links outside does not cache. ``--cache-dir`` and
  ``RUFF_CACHE_DIR`` may point anywhere.
- ``--no-cache`` disables cache reads and writes.
- Each simplint version and cache format has its own subdirectory. A run removes stale build directories after
  30 days without writes.

.. note::

    A project that runs two versions, as one pinned in `pre-commit <https://pre-commit.com/>`__ and another on the
    command line, keeps both caches.
