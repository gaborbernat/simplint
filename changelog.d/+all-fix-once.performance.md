[missing-from-all](../src/rules/api/missing_from_all.md) (SMP401) and [unused-export](../src/rules/api/unused_export.md)
(SMP402) attach their fix to the first report only, instead of copying the whole listing into every report. A module
with many names no longer grows its output and memory quadratically.
