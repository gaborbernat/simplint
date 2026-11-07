`rst-missing-param`, `rst-missing-attribute`, `rst-missing-raises` and `rst-returns` no longer offer to add `:param:`,
`:ivar:`, `:raises:`, `:returns:` or `:yields:` fields with empty descriptions, which Sphinx's nitpicky builds reject
and `rst-empty-field` reports. The reports stay, and the author writes the descriptions.
