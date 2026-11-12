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


def run(address: str) -> str:
    dflt_lookup(address)
    dflt_alias(address)
    dflt_bag(address)
    dflt_probe(address)
    dflt_settings(address)
    dflt_apply(address)
    dflt_source(address)
    dflt_fetch(address)
    dflt_apply_alias(address)
    dflt_source_alias(address)
    dflt_cwd(address)
    dflt_sep(address)
    dflt_run_data(address)
    dflt_run_class(address)
    dflt_run_fn(address)
    dflt_show(address)
    dflt_tone(address)
    dflt_member(address)
    return dflt_settings_note(address)
