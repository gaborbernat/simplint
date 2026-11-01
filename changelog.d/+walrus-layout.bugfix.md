`walrus-candidate` (SMP100) now also fixes values continued with a backslash, and when its fix leaves an `if` that
Ruff's collapsible-if (SIM102) asks to merge, the merge happens in the same run whatever the line length, so a second
run of the pipeline changes nothing.
