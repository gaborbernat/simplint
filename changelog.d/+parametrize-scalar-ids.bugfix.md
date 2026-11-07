`parametrize-without-id` (SMP802) no longer reports a row that is a plain string, number, `True`, `False` or `None`
whose value pytest already uses as its id, such as `"select"`. It still reports such a row when another row repeats its
value, since pytest then suffixes the shared id.
