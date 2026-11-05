Types in a public signature count as public API: a class or alias that an exported name's annotations or bases
refer to is no longer reported as unused in `__all__`, and missing-from-all lists it there.
