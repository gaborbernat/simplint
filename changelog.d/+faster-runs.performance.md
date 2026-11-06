Runs are faster: a run over unchanged files skips building the import index and answers from the cache, reports are
written in parallel in every output format, the cache stores fixes and the `full` text of reports ready to use, and the
process exits without freeing what it built. On pandas, scipy, SQLAlchemy and CPython, a second run takes half the time
or less, a run without a cache 11 to 25% less, and a second run with `--output-format json` a quarter of the time. The
cache grows by about the size of the `full` report it keeps.
