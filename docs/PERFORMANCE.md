# Rusty performance observations

The native architecture uses bounded section/target/geometry caches, finite per-frame scan work, shared HUD placement and context invalidation. These mechanisms do not establish a measured speedup by themselves.

Fresh Rusty performance/endurance measurements are pending the accepted native executable. Report frame-cap conditions, process CPU, native CPU/GPU timing, queues, cache work and memory with their real scopes. Whole-process private commit and Rust requested-allocation counters have different meanings. A bounded quiet tail cannot establish leak-free behavior or multi-hour stability.

[Validation](VALIDATION.md) · [Development](DEVELOPMENT.md)
