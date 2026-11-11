[`unnamed-regex-group`](../src/rules/logic/unnamed_regex_group.md) decides once per name whether the code reads a group
by index, instead of scanning that name's uses for each pattern bound to it. The repro at 80,000 patterns takes 1.6
seconds under SMP532 instead of 3.1.
