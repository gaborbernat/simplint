[`super-forwarding-method`](../src/rules/logic/super_forwarding_method.md) looks up the bindings and the classes of a
module in indexes built once per file, instead of scanning the class bodies for each method. A class of 64,000
forwarding methods takes 0.32 seconds instead of 35.5.
