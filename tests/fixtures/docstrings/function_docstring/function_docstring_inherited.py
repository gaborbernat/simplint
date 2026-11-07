class _Mixin:
    def describe(self) -> str:
        """Describe the object for the users of the public class."""
        return "described"

    def _helper(self) -> str:
        """Internal helper."""
        return "helper"


class Child(_Mixin):
    """A public class that inherits the mixin."""
