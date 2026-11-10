The fixes of [up-context-var-token-reset](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP512.html),
[up-chdir-restore](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP513.html),
[up-warnings-deprecated](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP524.html) and
[early-assignment](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP526.html) write the file's own line
ending, so a CRLF file no longer gets LF lines mixed in.
