`missing-from-all` (SMP401) removes the `# noqa: F401` it makes unused when it lists the name in `__all__`, so Ruff's
RUF100 no longer reports it. Names imported from one line share that `noqa`, which is removed once; before, an unapplied
fix for them crashed simplint while it rendered the report.
