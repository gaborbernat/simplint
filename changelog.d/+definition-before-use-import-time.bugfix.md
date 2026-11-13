[`definition-before-use`](../src/rules/layout/definition_before_use.md) no longer keeps in place the helpers of a base
class or of a function whose default or decorator reads a constant, as neither runs its body while the module loads, and
lets such a function move as long as the names its statement reads stay bound above it.
