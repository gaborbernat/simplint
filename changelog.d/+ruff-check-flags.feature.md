`simplint check` takes the rest of `ruff check`'s flags: `--watch` checks again whenever a Python or TOML file changes,
`--add-noqa[=REASON]` and `--add-ignore[=REASON]` write suppression comments for every report, `--show-files` and
`--show-settings` print the files and settings a check would use, and `--preview`, `--extend-ignore`,
`--per-file-ignores`, `--extend-per-file-ignores`, `--fixable`, `--unfixable`, `--extend-fixable` and
`--extend-unfixable` work as in Ruff, with Ruff's conflicts between flags.
