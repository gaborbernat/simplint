[`module-attribute-import`](../src/rules/imports/module_attribute_import.md) keeps its report without a fix when the
file and the module it reads from are in one cycle of runtime imports. The fix turned `module.Name` reads into a
from-import, which fails at load time when the cycle reaches the name before the module defines it.
