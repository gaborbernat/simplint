from _test_only_parameter_enums import Hue, Shade
from _test_only_parameter_injected import (
    Mailer,
    Mode,
    blend,
    cache_it,
    deliver,
    flag,
    fold,
    hook_it,
    hush,
    keep,
    knock,
    locate,
    mix,
    notify,
    paint,
    ping,
    pick,
    relay,
    ring,
    route,
    stash,
    stretch,
    style,
    tint,
)


class FakeNotifier:
    def notify(self, address: str) -> None:
        assert address


def test_injected() -> None:
    assert deliver("a", notifier=FakeNotifier()) is None
    assert relay("a", hook=print) is None
    assert pick("a", mailer=Mailer()) is None
    assert notify("a", hook=print) is None
    assert keep("a", notifier=FakeNotifier()) is not None
    assert hush("a", notifier=FakeNotifier()) is None
    assert ring("a", clock=print) is None
    assert ping("a", notifier=FakeNotifier()) is None
    assert locate("a", module=print) == "a"
    assert hook_it("a", dep=print) is None
    assert paint("a", palette=None) is None


def test_flags() -> None:
    assert route("a", mode=Mode.SLOW) == "A"
    assert tint("a", shade=Shade.DARK) == "DARK:a"
    assert blend("a", shade=Shade.DARK) == "A"
    assert mix("a", hue=Hue.BLUE) == "blue"
    assert style("a", tone="bold") == "bold:a"
    assert flag("a", urgent=True) == "a"
    assert stretch("a", limit=5) == "aaaaa"
    assert knock("a", notifier=FakeNotifier()) is None
    assert stash("a", method_map={}) == "a"
    assert cache_it("a", options={}) == "A"
    assert fold("a", items={}) == "a"
