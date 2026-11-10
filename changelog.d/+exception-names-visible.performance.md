Finding the exception classes a file's names can refer to, which
[indistinct-error-class](../src/rules/classes/indistinct_error_class.md) relies on, no longer checks every exception
class of the run in every file: 16,000 modules that each define an exception class take 0.3 seconds instead of 10.1.
