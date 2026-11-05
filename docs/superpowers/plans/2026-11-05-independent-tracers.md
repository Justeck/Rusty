# Independent entity tracer targets implementation plan

**Goal:** Give Entity Tracers independent Players, Mobs and Dropped Items controls in the native, Fabric and NeoForge editions.

**Architecture:** Retain the existing separate module switch and bounded received-entity rendering. Store tracer target flags separately from ESP flags and route each renderer to its own selection. Preserve old native player/mob coverage and old Java shared choices when loading existing profiles; fresh settings select players only. Keep Finder block tracers unchanged.

**Tech stack:** Native Rust/Serde/Horizon API409; Java25/Minecraft26.3/Fabric/NeoForge.

**Spec:** User request: “have tracers also separate like entity esp where you can choose what it shows tracers of”.

- [ ] Demonstrate missing independent settings with failing native and Java acceptance checks.
- [ ] Add saved independent filters, migration and default controls. Cover all selection combinations and profile round trips.
- [ ] Filter native received mobs and dropped items with bounded work; independently filter Java boxes and lines in a shared entity pass.
- [ ] Exercise real menus, dropped-item and mob positive/negative controls in owned offline worlds; inspect actual screenshots. Preserve personal world/preferences.
- [ ] Build through the official launcher, publish immutable native and Java releases after checks, install the accepted native release and update GitHub/Modrinth documentation and artifacts.

**Review focus:** Empty target selection draws nothing; disabled tracer module draws nothing; items must not be treated as mobs; toggling ESP targets never changes tracers; old saved choices and explicit new false values survive import.
