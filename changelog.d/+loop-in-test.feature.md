`loop-in-test` (SMP813) reports a `for` loop in a test function that runs the code under test once per case from a table
the file lists, such as a literal or a module constant, and asserts on each result, since the first failure hides the
rest. Parameterize the cases instead. Loops over values the test computes, `range` sweeps, `while` loops, loops that
only compare items with one result, and assertions inside `self.subTest(...)` or pytest's `subtests.test(...)` are left
alone.
