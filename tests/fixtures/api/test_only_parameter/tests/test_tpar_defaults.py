from _test_only_parameter_defaults import (
    dflt_alias,
    dflt_apply,
    dflt_apply_alias,
    dflt_bag,
    dflt_cwd,
    dflt_fetch,
    dflt_lookup,
    dflt_member,
    dflt_probe,
    dflt_run_class,
    dflt_run_data,
    dflt_run_fn,
    dflt_sep,
    dflt_settings,
    dflt_settings_note,
    dflt_show,
    dflt_source,
    dflt_source_alias,
    dflt_tone,
)


class FakeSettings:
    def apply(self, address: str) -> str:
        return address


def test_defaults() -> None:
    assert dflt_lookup("a", table={}) == 0
    assert dflt_alias("a", table={}) == 0
    assert dflt_bag("a", bag={}) == 0
    assert dflt_probe("a", client=print) == "a"
    assert dflt_settings("a", settings={}) == 0
    assert dflt_apply("a", settings=FakeSettings()) == "a"
    assert dflt_source("a", source=FakeSettings) == "a"
    assert dflt_fetch("a", produced=FakeSettings()) == "a"
    assert dflt_apply_alias("a", apply=FakeSettings) == "a"
    assert dflt_source_alias("a", source=FakeSettings) == "a"
    assert dflt_cwd("a", module=print) == "a"
    assert dflt_sep("a", sep="/") == "a"
    assert dflt_run_data("a", retries={}) == 0
    assert dflt_run_class("a", settings=FakeSettings()) == "a"
    assert dflt_run_fn("a", source=FakeSettings) == "a"
    assert dflt_show("a", log=print) is None
    assert dflt_tone("a", shade=None) == "a"
    assert dflt_member("a", shade=None) == "a"
    assert dflt_settings_note("a", settings=FakeSettings()) == "a"
