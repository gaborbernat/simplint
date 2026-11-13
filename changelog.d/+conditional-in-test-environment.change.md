[`conditional-in-test`](../src/rules/testing/conditional_in_test.md) no longer reports branches on the interpreter or
its environment, such as `sys.platform` or `os.environ`, nor branches on what a `while` loop or a `for` loop over
received values got, since neither can become a parameter.
