# Rusty 1.2.6

**Choose tracer targets independently of Entity ESP.**

- **Entity Tracers → Players / Mobs / Dropped Items:** select any combination, or turn all three off. Each choice has its own Default button and is saved in profiles.
- **Separate views:** draw boxes around mobs while tracing dropped items, or choose another combination. Changing Entity ESP preferences does not change tracer targets. Disabling tracers leaves ESP enabled.
- **Predictable upgrades:** existing native profiles keep their previous player-and-mob tracer coverage. Fresh settings select players only; Entity Tracers remains disabled until enabled. Existing explicit choices survive reload and import.
- **Bounded client rendering:** use only received entities inside the search radius, with a 128-line cap. Finder still owns block tracers; enabling X-ray does not enable either tracer feature.

Horizon-native Rust for **Horizon 38609b7 / API 409 / Minecraft 26.3**. Paste `https://github.com/Justeck/Rusty/releases/tag/v1.2.6` into Horizon's native GitHub input; enable Rusty and complete a Performance build if requested. Open **Right Shift → Entity Tracers** for the new controls. Package: `rusty-1.2.6.hnmod`, with `SHA256SUMS`.

[Validation and exact scope](https://github.com/Justeck/Rusty/blob/main/docs/VALIDATION.md). Fabric and NeoForge counterparts are **1.3.0-beta.5**; Java jars are separate from Horizon's native installer. Apache-2.0 and applicable notices are preserved.
