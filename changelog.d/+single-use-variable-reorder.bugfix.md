[`single-use-variable`](../src/rules/logic/single_use_variable.md) no longer offers a fix that runs a value with calls
or lookups after a step it used to precede, such as one of two `stack.pop()` operands read in reverse order.
