`function-docstring` (SMP901) leaves the docstring of a method that a public class of a library inherits, without
overriding it, from a private base or mixin, since users reach it through that class. `undocumented-public-api` (SMP903)
expects one there.
