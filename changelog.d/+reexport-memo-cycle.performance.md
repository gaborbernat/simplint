Looking up a re-exported class is memoised again in the files that reach no import cycle, even when the project has one
elsewhere. A chain of 16,000 re-exporting packages with two unrelated modules that import each other takes 1.4 seconds
instead of 20.3.
