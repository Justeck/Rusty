# Rusty Java 1.4.0-beta.1 — Schematic Builder

Fabric and NeoForge for **Minecraft 26.3 / Java 25**.

Build Planner gains a native OS file chooser, bounded modern schematic import, exact-state preview, origin/rotation/mirror controls, a copyable materials list, ordinary walking/inventory-sourcing construction and two-corner states-only `.schem` export. Loading never starts construction. Slabs/layers and coupled blocks use normal item semantics, normal published view and received exact-state settlement.

Manual input, menus, damage, health/food thresholds, corrections and scope changes stop construction without automatic restart. Unsupported, occupied or inaccessible work remains explicit; no demolition, flight, reach extension or unrequested scaffolding is added. Entity/block-entity payloads are not reconstructed.

**Construction result / details** separates initial matches, newly completed cells, placement actions and final leftovers. Relevant terrain and cache changes invalidate unfinished verification; persistent churn stays unverified. Build permission is checked at Start and throughout the run. Cancellation remains responsive at the save-commit boundary, and stopped readback cannot restart construction.

Multiplayer restricted construction retains explicit current-server **view + movement + assistance** permission requirements. The server controls inventory, movement and placement. The bounded planner cannot promise every possible structure or invulnerability.

[Schematic guide](https://github.com/Justeck/Rusty/blob/main/docs/SCHEMATICS.md) · [Java validation](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md) · [Horizon native](https://github.com/Justeck/Rusty/releases/tag/v1.4.0). Install only the jar for the selected Java loader; Horizon uses the separate native package. Required licenses remain embedded.
