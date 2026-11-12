[`wall-clock-duration`](../src/rules/logic/wall_clock_duration.md) reports durations measured with `time.time()`, which
jumps when the system clock is set, and its fix measures them with `time.perf_counter()`.
