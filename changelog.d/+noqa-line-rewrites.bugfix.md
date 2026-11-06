`test-class` (SMP801) no longer flattens a test class with an E501 `noqa` or `ruff: ignore` in it: dedenting its methods
shortens the line the directive covers, which Ruff then reported as RUF100.
