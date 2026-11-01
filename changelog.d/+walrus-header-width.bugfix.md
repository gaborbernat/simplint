`walrus-candidate` (SMP100) no longer reports an assignment when folding it into the `if` test would make the header too
wide for one line, which `ruff format` then split over several. The header is measured joined on one line at its block's
indentation, so the result does not depend on how the code is laid out.
