`unnamed-regex-group` (SMP532) reads the decorators, default values and annotations of a function in the scope around
it, so a match read in a default no longer makes the match look unused and gets an unsafe fix that breaks the default.
