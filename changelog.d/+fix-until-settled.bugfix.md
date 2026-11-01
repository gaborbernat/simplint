`simplint check --fix` runs its fixes again until they change nothing, so a fix that Ruff's cleanup makes possible, such
as joining the f-string Ruff writes for `%` formatting, or one withdrawn only next to another rule's pending fixes, no
longer waits for a second run.
