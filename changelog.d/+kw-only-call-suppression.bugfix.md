`dataclass-kw-only` (SMP114) rewrites a positional call that a `noqa` comment for another code sits on, or that
holds `noqa` in a string, instead of leaving the dataclass without `kw_only=True`; only a comment that suppresses SMP114
keeps the call as it is.
