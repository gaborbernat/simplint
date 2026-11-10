A file that imports a module outside the run, such as one from the standard library, no longer compares its module name
with the path of every file of the run: 16,000 files that import and call standard-library modules take 0.4 seconds
instead of 1.6.
