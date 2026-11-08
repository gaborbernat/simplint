`parametrize-without-id` (SMP802) checks a row name bound once to a `pytest.param(...)` call by that call's `id=`, so a
module constant that already has an id is no longer reported. Names the rule cannot read, such as imported or rebound
ones, are skipped.
