`constructor-after-method` (SMP203) is renamed `class-member-order`, and it now orders the whole class body rather than
only its constructors. A class body reads in sections: nested classes, class attributes, constructors in the order
Python runs them, alternative constructors, other dunder methods, properties, then methods. A member that sits above a
member of an earlier section is reported, with the section it belongs with, and one fix moves the members into place. A
helper still follows its first caller, whatever its section, and `definition-before-use` (SMP201) and this rule compute
the same order, so one `--fix` settles a class body. Both count moving a method whose decorators are all pure, such as
`@property` or `@classmethod`, as a safe fix.
