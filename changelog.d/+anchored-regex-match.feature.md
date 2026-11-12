[`anchored-regex-match`](../src/rules/logic/anchored_regex_match.md) reports `re.match()` with a pattern anchored at its
end, and `re.search()` with one anchored at both ends, whose `$` lets a trailing newline through; the fix calls
`re.fullmatch()` without the anchors. [`up-re-prefixmatch`](../src/rules/logic/up_re_prefixmatch.md) leaves those calls
to it when both are selected.
