`private-import-path` accepts imports from a matching source file when the test mirrors a private package path or lives
in a test subtree named after that module. The test tree may omit the top-level package directory; a `.pyi` can identify
a compiled module.
