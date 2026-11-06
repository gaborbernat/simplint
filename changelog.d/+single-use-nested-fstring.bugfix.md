`single-use-variable` (SMP504) no longer offers a fix before Python 3.12 that puts an f-string with a string in its
replacement field into another f-string. Such a fix broke the file's syntax, and simplint reverted every fix in it.
