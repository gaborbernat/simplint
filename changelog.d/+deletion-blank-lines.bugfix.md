A fix that removes a statement between two blank lines leaves one of them, as `ruff format` would, so files that
`ruff format` does not run on, such as `# fmt: off` ones, gain no new E303.
