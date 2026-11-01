`missing-from-all` and `all-format` write `__all__` one name per line with a trailing comma, the layout `ruff format`
gives it, and `missing-from-all` inserts names in sorted order and above a closing `if __name__ == "__main__":` block.
