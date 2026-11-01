simplint no longer repeats Ruff's warnings about Ruff's own configuration, such as a removed rule left in `ignore` or a
deprecated top-level setting, on every run; `ruff check` already shows them. Warnings about simplint's own work stay.
