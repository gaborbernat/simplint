`module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report a module-level alias of a
function the module defines, such as `_build_cli = build_cli`, which follows the function's name. An alias of an
imported function is still reported.
