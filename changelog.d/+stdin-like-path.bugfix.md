Source read from standard input is linted as the file `--stdin-filename` names would be: it gets that file's
`per-file-target-version`, its package, and the directory above its package as an import search root, so entry points
outside the `src` directories count as uses.
