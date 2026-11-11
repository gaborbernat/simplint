[`definition-before-use`](../src/rules/layout/definition_before_use.md) follows each chain of helpers that a function is
used from once, instead of once from every function in it. A chain of 64,000 helpers takes 0.20 seconds under SMP201
instead of 2.6.
