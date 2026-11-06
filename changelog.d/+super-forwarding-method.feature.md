`super-forwarding-method` (SMP527) reports a method whose only statement returns the same method of `super()` called
with its own arguments, such as `return super().load(name)`, when the module defines the parent's method with the same
parameters. Deleting the method lets the subclass inherit the parent's method. The fix is not offered for an annotated
method, since its annotations are the types the subclass declares.
