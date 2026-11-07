`imprecise-annotation` (SMP602) no longer reports `object` as the annotation of `*args` or `**kwargs`, the honest type
for arbitrary values. Before, no annotation satisfied both it and ANN401, which flags `Any` there.
