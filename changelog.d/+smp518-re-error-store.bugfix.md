`up-re-error-alias` leaves `re.error = ...` and `del re.error` alone, which patch the module rather than read the
exception.
