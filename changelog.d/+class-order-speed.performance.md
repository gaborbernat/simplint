[`definition-before-use`](../src/rules/layout/definition_before_use.md) and
[`class-member-order`](../src/rules/layout/class_member_order.md) compare each class statement with the others by name
instead of with every statement, and share one build of each class order, so a class of 64,000 attributes takes 0.05
seconds under SMP201 instead of 2.8 and 0.06 under SMP203 instead of 2.6.
