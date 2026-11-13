[`shared-mutable-repetition`](../src/rules/logic/shared_mutable_repetition.md) reports a list or tuple repeated with `*`
whose element is a mutable object, such as `[[0] * n] * m`, when the code then changes an element through the name it is
stored in, which changes every copy. Its unsafe fix builds the element in a comprehension.
