`--add-noqa` and `--add-ignore` keep what follows the codes of a comment they merge into: its reason and any later
comment, such as `# type: ignore[import]` or `# pragma: no cover`. A reason passed to the flag replaces only the old
reason.
