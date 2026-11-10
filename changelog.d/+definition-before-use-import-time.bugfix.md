[`definition-before-use`](../src/rules/layout/definition_before_use.md) keeps a helper in place when a `def` inside a
module-level block, such as `if __name__ == "__main__":`, calls it while the module loads, and when a definition reads
an import written below it. The fix used to move those helpers above the code that needs them, which broke the module at
import time.
