`unnamed-regex-group` (SMP532) reports a regular expression pattern with two or more unnamed capture groups, or with one
that the code reads by its number, so each group gets a name the code can read. The groups a pattern's code never reads
become non-capturing under an unsafe fix.
