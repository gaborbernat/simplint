A file that does not parse is skipped with a warning instead of reporting its syntax errors as `invalid-syntax`, which
Ruff already reports. A run whose only problem is such a file exits with 0.
