[`up-unittest-enter-context`](../src/rules/testing/up_unittest_enter_context.md) counts the names of each test method
once instead of scanning the tokens of the whole file for every patch it checks, so 16,000 manual patch sequences take
0.15 seconds instead of 3.3.
