`check` reads standard input as Ruff does: with `--stdin-filename` the source is standard input whatever paths come with
it, and a `-` beside other paths is no longer standard input. `--add-noqa` and `--add-ignore` refuse standard input
instead of editing the files under the working directory.
