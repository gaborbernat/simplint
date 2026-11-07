`dataclass-frozen` (SMP701), `dataclass-slots` (SMP702) and `dataclass-kw-only` (SMP703) read the options a `**` splat
passes from a module-level dict, so `@dataclass(**DC_KW)` no longer gets a fix that repeats `frozen=` or `slots=` and
breaks imports. A `**` splat simplint cannot read leaves the class unreported, and a call whose splat repeats a field is
no longer rewritten to pass that field by keyword.
