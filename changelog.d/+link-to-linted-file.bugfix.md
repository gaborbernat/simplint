A file the run finds both by its own path and through a link inside the project is linted and fixed once, by its own
path, instead of twice in parallel, where the second write failed and the reports showed twice.
