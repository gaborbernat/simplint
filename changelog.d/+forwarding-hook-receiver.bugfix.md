[`forwarding-function`](../src/rules/logic/forwarding_function.md) (SMP501) no longer reports a method of a class that
another class derives from, since a subclass may override it and calling the target at each call site would skip that
override. A method that forwards through its receiver, such as `self.client.get`, is now reported, and a method that
only calls its receiver, such as `self(name)`, is not.
