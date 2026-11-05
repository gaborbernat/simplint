`up-strptime-date` (SMP514) flags `datetime.strptime(...).date()` and `.time()` calls, which `date.strptime` and
`time.strptime` replace on Python 3.14 and later, with a safe fix for dates and for times in literal formats without
`%z`.
