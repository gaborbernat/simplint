`up-codecs-open` (SMP517) flags `codecs.open()` calls, which the builtin `open()` replaces with `newline=""` and an
encoding, with an unsafe fix since line iteration differs for some separators.
