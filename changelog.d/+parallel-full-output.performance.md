`simplint check` renders its default `full` output on all cores, so a report with many fix diffs, such as renames that
span whole modules, prints sooner: on `sympy` the run takes about 40% less wall time.
