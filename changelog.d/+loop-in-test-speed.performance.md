[`loop-in-test`](../src/rules/testing/loop_in_test.md) follows each chain of local names that a test's table is built
from once instead of once per path through it, so eight chained names with twelve references each take under 0.01
seconds instead of 16.4.
