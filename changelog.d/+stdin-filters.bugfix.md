`--stdin-filename` applies the file filters a file on disk gets: a notebook, TOML or Markdown name, or a name
`--force-exclude` leaves out, reports nothing and exits 0. With `--fix`, its source goes out unchanged. A `.pyi` name is
still linted, though a stub on disk is only indexed.
