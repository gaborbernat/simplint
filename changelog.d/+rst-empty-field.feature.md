`rst-empty-field` (SMP914) reports `:param:`, `:raises:`, `:returns:`, `:yields:` and `:var:` fields with no
description, which `rst-field-syntax` reported before. The fixes of the missing-field rules add such fields, so this
rule can be turned off on its own.
