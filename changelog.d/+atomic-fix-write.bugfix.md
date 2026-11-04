`--fix`, `--add-noqa` and `--add-ignore` write a file by renaming a complete copy over it, so a failed write no longer
leaves a cut-short source, and a file edited during the run keeps the edit. A symlink stays a symlink and the file keeps
its permissions.
