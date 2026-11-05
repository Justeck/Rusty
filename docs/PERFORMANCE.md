# Rusty performance observations

Measurements use the same accepted executable, fresh vanilla worlds, an offscreen window requested at 960×540, render distance 4 and a 60 FPS cap. Each condition contains three five-second samples. The table reports the mean of those sample averages; it is not a maximum-throughput benchmark.

| Condition | Mean FPS | Process CPU seconds per wall second | Mean resident MiB |
|---|---:|---:|---:|
| Mod disabled | 60.00 | 0.188 | 989.6 |
| Rusty normal | 60.00 | 0.171 | 999.1 |
| Rusty xray | 60.00 | 0.203 | 1029.4 |
| Rusty menu | 60.00 | 0.201 | 1032.3 |

The stationary X-ray sample observed **797 additional aggregation-cache reuses**. Queue validation and bounded section rebuilding continue independently. The source uses finite section, target, context and geometry budgets, avoiding unbounded scan work.

These short, capped samples do not establish that Rusty is faster than the host or another client. Process CPU time includes all process threads; resident memory includes host worlds, renderer resources and allocator reservations. The native menu keeps singleplayer simulation running. [Measurement summaries](evidence/performance.json) preserve CPU/GPU/phase statistics and original receipt hashes.

## Bounded stability run

The accepted executable completed **600.0 measured seconds**, including recorded module/input and world-context operations. No panic/fatal marker was observed, and the owned process stopped cleanly. [Endurance evidence](evidence/endurance.json) preserves samples, operations and memory observations.

Ten minutes cannot prove leak-free behavior, multi-hour stability, other hardware, arbitrary resource packs or server-specific behavior. Target counts update incrementally and can briefly change between cache discovery intervals; zero queued work is an instantaneous observation, not an atomic inventory of every received ore. The multiplayer fixture therefore establishes settled exact counts across more than two scan intervals, distinct rendered frames and ready renderer queues.

[Validation](VALIDATION.md) · [Development](DEVELOPMENT.md)
