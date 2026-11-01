`string-concatenation` joins strings in different quote styles when one delimiter fits them all, joins implicitly
concatenated literals, puts an expression written over several lines on one, and before Python 3.12 switches the quotes
of string literals inside an expression that would clash with the f-string's own. It leaves alone a literal with a named
escape such as `\N{BULLET}`, and inside another f-string's replacement field keeps the quotes as written.
