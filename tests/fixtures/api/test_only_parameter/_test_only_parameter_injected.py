from collections.abc import Callable, Mapping
from enum import Enum
from types import ModuleType
from typing import Literal, Optional, Protocol

from _test_only_parameter_enums import Hue, Palette, Shade


class Mode(Enum):
    FAST = 1
    SLOW = 2


class Notifier(Protocol):
    def notify(self, address: str) -> None: ...


class Mailer:
    def send(self, address: str) -> None:
        print(address)


def print_hook(address: str) -> None:
    print(address)


DEFAULT_HOOK = print_hook


def deliver(address: str, notifier: Notifier | None = None) -> None:
    if notifier is not None:
        notifier.notify(address)


def relay(address: str, hook: Callable[[str], None] | None = None) -> None:
    if hook is not None:
        hook(address)


def pick(address: str, mailer: Mailer = Mailer()) -> None:
    mailer.send(address)


def notify(address: str, hook=DEFAULT_HOOK) -> None:
    hook(address)


def keep(address: str, notifier: Notifier | None = None) -> Notifier | None:
    return notifier


def hush(address: str, notifier: Optional[Notifier] = None) -> None:
    if notifier:
        notifier.notify(address)


def ring(address: str, clock: Clock = None) -> None:
    clock.tick(address)


def ping(address: str, notifier: Notifier.Base | None = None) -> None:
    notifier.notify(address)


def locate(address: str, module: ModuleType | None = None) -> str:
    return module.__name__ + address


def hook_it(address: str, dep=None) -> None:
    dep(address)


def paint(address: str, palette: Palette | None = None) -> None:
    if palette is not None:
        palette.paint(address)


def route(address: str, mode: Mode = Mode.FAST) -> str:
    return address if mode is Mode.FAST else address.upper()


def tint(address: str, shade: Shade = Shade.LIGHT) -> str:
    return f"{shade.name}:{address}"


def blend(address: str, shade: Shade | None = None) -> str:
    return address if shade is None else address.upper()


def mix(address: str, hue: Optional[Hue] = None) -> str:
    return address if hue is None else hue.value


def style(address: str, tone: Literal["plain", "bold"] = "plain") -> str:
    return f"{tone}:{address}"


def flag(address: str, urgent: bool = False) -> str:
    return address if urgent else address.upper()


def stretch(address: str, limit: int = 3) -> str:
    return address * limit


def knock(address: str, notifier: "Notifier" = None) -> None:
    notifier.notify(address)


def stash(address: str, method_map=None) -> str:
    return method_map.get(address, address)


def cache_it(address: str, options: dict[str, bool] | None = None) -> str:
    return address if options is None else address.upper()


def fold(address: str, items: Mapping[str, int] | None = None) -> str:
    return address if items is None else address.lower()
