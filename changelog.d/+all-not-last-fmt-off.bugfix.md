`all-not-last` moves a `# fmt: off` / `# fmt: on` pair that fences `__all__` along with it, so `ruff format` does not
reformat the list and strand a suppression comment that covers several names on one line.
