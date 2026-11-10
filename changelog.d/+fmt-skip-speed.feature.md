Files with many `# fmt: skip` comments are checked faster: a 20,000-line module with one on every line takes a quarter
of the time it did under `--fix`.
