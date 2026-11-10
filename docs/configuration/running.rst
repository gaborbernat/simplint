Running
#######

Run options
===========

The table also takes the run options Ruff reads from ``[tool.ruff]``, with the same names:

.. code-block:: toml

    [tool.simplint]
    fix = false            # apply safe fixes, like --fix
    fix-only = false       # fix and report nothing else, like --fix-only
    unsafe-fixes = false   # include unsafe fixes, like --unsafe-fixes; unset shows a hint instead
    show-fixes = false     # list what was fixed, like --show-fixes
    output-format = "full" # like --output-format

The options of the table found for the **current directory** apply to the whole run. The matching command-line flag,
or its ``--no-`` form, overrides them.

Cache
-----

Like Ruff, simplint caches which files had nothing to report, keyed by each file's modification time and
permissions, and also by its size. With a :ref:`cross-file rule <cross-file-rules>` enabled, a cached result also
depends on the names other files use from that file, so editing an importer checks the files it imports again. A file
only per-file rules check keeps its cached result. A file whose fixes cannot be written is not cached, because whether
its directory lets it be written is not part of its key. A ``--diff`` run caches nothing computed from a fix it keeps in
memory, neither for the file it fixes nor for another file linted after that fix, because those reports describe the
fixed source, not the disk.

Without fixes, a file that does not parse, or cannot be read because it is not UTF-8, keeps its syntax error or read
error too, so a warm run reports it again without building the project index. A read error that is only logged, because
``io-error`` (E902) is not selected, is not cached, so every run logs it again.

.. mermaid::

    flowchart TD
        file["File in the run"] --> same{"mtime, permissions<br/>or size changed?"}
        same -->|yes| lint["Lint it"]
        same -->|no| clean{"Cached clean under the<br/>same settings and, for<br/>cross-file rules, index?"}
        clean -->|yes| skip["Skip it"]
        clean -->|no| lint
        lint --> result{"Nothing to report?"}
        result -->|yes| remember["Remember it as clean"]
        classDef blue fill:#9cc0e3,stroke:#1f4f7d,color:#062d59
        classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
        classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
        class file,lint blue
        class same,clean,result yellow
        class skip,remember green

Where the cache lives:

- In a ``simplint`` directory inside Ruff's cache directory: ``.ruff_cache`` unless Ruff's ``cache-dir`` setting,
  ``--cache-dir`` or ``RUFF_CACHE_DIR`` names another. It never touches Ruff's own files there.
- Inside the project, when the project picks it. A ``cache-dir`` setting that leads outside falls back to
  ``.ruff_cache``, and a run whose ``.ruff_cache`` links outside does not cache. ``--cache-dir`` and
  ``RUFF_CACHE_DIR`` may point anywhere.
- ``--no-cache`` skips it.
- Each simplint build keeps its own subdirectory there. A run removes the subdirectories of other builds that no run
  has written to for 30 days.

.. note::

    A project that runs two versions, as one pinned in pre-commit and another on the command line, keeps both caches.
