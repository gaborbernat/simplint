from ._helpers import helper


class Engine:
    def start(self) -> None:
        helper()

    def _stop(self) -> None: ...

    @property
    def ready(self) -> bool:
        return True

    class Settings:
        def load(self) -> None: ...


def run() -> None:
    helper()


def internal() -> None:
    """Only the package uses this."""
