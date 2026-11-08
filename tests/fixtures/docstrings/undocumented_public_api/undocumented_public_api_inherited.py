class _Mixin:
    def describe(self) -> str:
        return "described"


class Child(_Mixin):
    """A public class that inherits the mixin."""
