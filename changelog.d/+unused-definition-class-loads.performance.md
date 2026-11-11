[`unused-definition`](../src/rules/api/unused_definition.md) finds the names a class body reads in one walk, instead of
walking the whole body for each method and attribute. A class of 32,000 unused methods takes 0.08 seconds under SMP409
instead of 10.2.
