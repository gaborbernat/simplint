On Windows ARM64, parallel file identity lookups cut cold-check time by 2.68% across 100 PyPI packages. Reusing symlink
checks saved another 2.01%, and checking for `__init__.py` once per directory saved a further 2.16%.
