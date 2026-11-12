import os

import _test_only_parameter_enums
import _test_only_parameter_settings
from _test_only_parameter_enums import Shade
from _test_only_parameter_settings import SETTINGS, Settings, make_settings

DEFAULTS = {"a": 1}
TABLE_ALIAS = DEFAULTS
DEFAULT_SOURCE = make_settings
DEFAULT_APPLY = Settings


def dflt_lookup(address: str, table=DEFAULTS) -> int:
    return table.get(address, 0)


def dflt_alias(address: str, table=TABLE_ALIAS) -> int:
    return table.get(address, 0)


def dflt_bag(address: str, bag=dict()) -> int:
    return bag.get(address, 0)


def dflt_probe(address: str, client=NOWHERE) -> str:
    return client.ping(address)


def dflt_settings(address: str, settings=SETTINGS) -> int:
    return settings.get(address, 0)


def dflt_apply(address: str, settings=Settings()) -> str:
    return settings.apply(address)


def dflt_source(address: str, source=make_settings) -> str:
    return source().apply(address)


def dflt_fetch(address: str, produced=make_settings()) -> str:
    return produced.apply(address)


def dflt_apply_alias(address: str, apply=DEFAULT_APPLY) -> str:
    return apply().apply(address)


def dflt_source_alias(address: str, source=DEFAULT_SOURCE) -> str:
    return source().apply(address)


def dflt_cwd(address: str, module=os) -> str:
    return module.getcwd() + address


def dflt_sep(address: str, sep=os.sep) -> str:
    return sep.join([address])


def dflt_run_data(address: str, retries=_test_only_parameter_settings.SETTINGS) -> int:
    return retries.get(address, 0)


def dflt_run_class(address: str, settings=_test_only_parameter_settings.Settings()) -> str:
    return settings.apply(address)


def dflt_run_fn(address: str, source=_test_only_parameter_settings.make_settings) -> str:
    return source().apply(address)


def dflt_show(address: str, log=print) -> None:
    log(address)


def dflt_tone(address: str, shade: _test_only_parameter_enums.Shade = None) -> str:
    return shade.name + address


def dflt_member(address: str, shade: Shade = _test_only_parameter_enums.Shade.LIGHT) -> str:
    return shade.name + address


def dflt_settings_note(address: str, settings: _test_only_parameter_settings.Settings | None = None) -> str:
    return settings.apply(address) if settings else address
