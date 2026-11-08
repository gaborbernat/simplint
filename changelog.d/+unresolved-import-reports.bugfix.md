`unused-definition` (SMP409), `unexported-public-name` (SMP403), `module-variable-case` (SMP102) and
`module-variable-prefix` (SMP404) no longer skip a name only because a stdlib or third-party import or a doctest spells
it, since such a spelling says nothing about the names of the project. Where such a spelling may read the name, the
rename fix is withheld.
