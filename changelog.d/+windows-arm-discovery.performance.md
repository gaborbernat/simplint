Windows ARM64 checks file identities in parallel during discovery and reuses each symlink check. Across 100 PyPI
packages, the two changes cut cold-check time by 2.68% and a further 2.01%.
