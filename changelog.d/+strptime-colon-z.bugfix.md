[`up-strptime-date`](../src/rules/logic/up_strptime_date.md) no longer applies a safe fix to `.time()` on a format with
`%:z`, since `time.strptime` keeps the zone that `.time()` drops, as it does with `%z`.
