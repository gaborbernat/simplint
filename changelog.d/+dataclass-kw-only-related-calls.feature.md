`dataclass-kw-only` (SMP703) reports a dataclass once, at its definition, and lists its positional calls as related
locations instead of reporting each call. The fix is offered only when every such call is in the same file; calls in
other files need converting by hand.
