`up-utf8-encoding-default` (SMP521) reports `encoding="utf-8"` on `open()`, `io.open()`, `Path` text methods and similar
calls from Python 3.15, where UTF-8 is the default encoding. Its fix is unsafe: UTF-8 mode can be turned off with
`PYTHONUTF8=0` or `-X utf8=0`.
