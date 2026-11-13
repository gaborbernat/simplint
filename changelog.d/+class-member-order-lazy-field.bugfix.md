[`class-member-order`](../src/rules/layout/class_member_order.md) no longer reads a class-level annotation as evaluated
when the class body runs while annotations are lazy, which withheld the fix that moves a nested class it names.
