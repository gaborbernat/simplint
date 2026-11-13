[`test-class`](../src/rules/testing/test_class.md) keeps its report without a fix when another module imports the class
or a string in the run names it: urllib3 imports `TestSSL` from another test module, and anyio's tests match qualified
names that contain the class.
