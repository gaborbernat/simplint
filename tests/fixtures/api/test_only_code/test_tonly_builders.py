from unittest import mock

import tonly_builders
from tonly_builders import (
    TonlyPublished,
    TonlyShared,
    TonlyUsed,
    _private_tonly,
    build_tonly_fixture,
    registered_tonly,
    shared_tonly,
)


def test_builders() -> None:
    assert build_tonly_fixture() == 1
    assert tonly_builders.TonlyRecord
    assert _private_tonly() == 5
    assert registered_tonly() == 6
    assert shared_tonly() == 2
    assert TonlyShared and TonlyPublished and TonlyUsed
    with mock.patch("tonly_builders.recurse_tonly"):
        pass
