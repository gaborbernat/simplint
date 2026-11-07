`class-member-order` (SMP203) no longer reports the statement of a Django model that
[DJ012](https://docs.astral.sh/ruff/rules/django-unordered-body-content-in-model/) reports, when the project's Ruff
configuration selects it, unless that report carries the fix for the class, which DJ012 lacks. Properties, class methods
and other members it reports are still reported here.
