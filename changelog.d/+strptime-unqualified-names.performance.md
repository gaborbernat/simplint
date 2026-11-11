[`up-strptime-date`](../src/rules/logic/up_strptime_date.md) counts the identifiers of the file once instead of scanning
every token of it for each `strptime` call. A module of 32,000 `datetime.strptime(...).date()` calls takes 0.12 seconds
instead of 6.7.
