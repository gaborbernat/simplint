`mock-assertion-typo` (SMP814) reports mock assertion methods written without their `assert_` prefix, such as
`called_once_with`, or misspelled, such as `assert_called_ones_with`, and renames each to the assertion it stands for.
The fix is unsafe: a call that checked nothing becomes a check, which may now fail. The unprefixed names return a child
mock and pass silently on Python 3.11 and older, and so do the unprefixed `awaited` names on every version.
