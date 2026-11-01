`single-user-constant` (SMP050) keeps a local name for a string whose one use calls `format`, since Ruff's f-string rule
(UP032) would treat the inlined literal differently depending on the layout.
