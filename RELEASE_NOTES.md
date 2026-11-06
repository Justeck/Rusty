# Rusty 1.4.0 — Schematic Builder

Build Planner now turns a local schematic into an explicit, controlled construction plan.

- **Load schematic** opens the native file picker for modern `.schem`, `.litematic` and structure `.nbt` files. Import previews without starting construction; set an origin, rotate or mirror the plan.
- **Materials / Copy materials** provides item quantities, stack totals, inventory availability and exact block-state counts. Coupled doors/beds and partial blocks use their ordinary item costs.
- **Start construction** uses bounded safe walking, normal inventory sourcing and exact ordinary placement checks. Real received settlement determines progress. Occupied, unsupported or inaccessible work remains explicit.
- **Export first / second corner → Save selection** writes a states-only Sponge `.schem` from settled client-received data. Entities, container contents and sign text are not recreated.
- Resettable construction health/food defaults, manual/menu/damage/scope stops, no automatic restart, and a retained final outcome summary keep the work visible and under player control.

The existing box preview/one-placement action, Area Clear and Auto Mine remain available. Construction adds no demolition, flight, extended reach or unrequested scaffolding. Normal server authority applies, and bounded planning cannot guarantee access to every structure or invulnerability.

Horizon-native Rust for **Horizon 66f3981 / API 411 / Minecraft 26.3**. Install `https://github.com/Justeck/Rusty/releases/tag/v1.4.0`, enable Rusty and complete a **Performance** build if requested. Package: `rusty-1.4.0.hnmod` with `SHA256SUMS`.

[Schematic guide](https://github.com/Justeck/Rusty/blob/main/docs/SCHEMATICS.md) · [Validation](https://github.com/Justeck/Rusty/blob/main/docs/VALIDATION.md). Fabric/NeoForge counterpart: **1.4.0-beta.1**, retaining its multiplayer capability policy. Required licenses and notices remain included.
