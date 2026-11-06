# Area Clear implementation plan

> Use superpowers:executing-plans for implementation and one fresh final review. The user's standing instruction is autonomous execution without repeated design approvals.

**Goal:** Select two opposite block corners and explicitly clear the inclusive cuboid with ordinary, guarded movement and mining.

**Architecture:** Extend the existing exclusive mining controller with an Area Clear mode. Share normal input ownership and live safety checks; keep independent persistent safety preferences and transient world/dimension-scoped corner selection. Pure Rust and Java region helpers normalize and bound coordinates, survey received cells, protect non-target cells and select the highest eligible layer.

**Tech stack:** Native Rust on Horizon 66f3981; equivalent Java25 Fabric/NeoForge runtime adapters. No server packet manipulation or new dependency.

## Design decisions

- Two explicit Set Corner buttons capture the aimed block. Clear Selection is available. Show corners, dimensions, volume and a twelve-edge box preview. Keybind is clearable and unbound by default.
- Inclusive axes, maximum 32 blocks per axis / 32,768 blocks. Reject overflow, world-border and host-height violations. Corners are not saved in profiles and clear on connection/world/dimension change.
- Start is explicit after selection review; automatic activation is never persisted. Auto Mine and Area Clear share one owner and cannot run together. Manual input, menus, focus, damage, low health/food, inventory/tool limits and stale/denied actions stop without resuming.
- Strict excavation boundary: no access-tunnel mining outside the selected box. Work on the highest remaining eligible layer first. Require safe standing support, headroom and conservative normal steps; never mine directly beneath/above the player.
- Preserve containers/block entities, unbreakable blocks, fluids and hazardous/unstable blocks. Ordinary partial-shape blocks may be mined but never treated as full supporting cubes. Unknown/pending cells stop progress. Completion requires a fresh survey finding only air; protected or unreachable leftovers are reported accurately.
- Area scanning is incremental and bounded, and the route grid covers the selection plus the player and a small safety margin, rather than the whole vertical world.
- Retain native-only client operation and Java's existing view+movement+assistance server grants. Tests use only owned disposable worlds and a separate authorized loopback server.

## Tasks and interfaces

1. **Pure region contract** — `mixins/rusty/mining.rs`, `tests/area_clear.rs`, `java/core/.../Mining.java`, `MiningTest.java`: `Region::new/contains/volume/position/edges`, bounded survey, position-dependent layer restriction and separate clear settings. Tests first: reversed/negative corners, oversized/overflow rejection, outside-box protection, highest-layer choice, unknown/protected leftovers, non-solid support and boundary-only route actions.
2. **Native adapter and menu** — extend `mining_native.rs` with scoped selection, area survey phase and shared exclusive run mode. Catalog/clickgui/control/runtime expose corner capture, selection summary, start/stop, safety defaults and preview. Test actual area removal, surrounding-block preservation, containers/water, no support mining, manual/menu/world resets, tool/server rejection and Auto Mine regression.
3. **Java counterparts** — extend `RustyMining`, settings/catalog/menu/activation/overlay and permission policy. Port pure regressions; owned game acceptance on both loaders verifies matching behavior and independent settings.
4. **Review and delivery** — fresh read-only final review, fix important findings with reproducing tests, run full appropriate checks, exact SDK and official Performance build, native singleplayer/separate-server and Java game checks. Publish immutable native1.3.1 / Java beta.8, verify public artifacts, install through Horizon, actual GUI Build/Play, preserve settings and one instance, update GitHub/Modrinth and superseded download cleanup.

## Review focus

Boundary enforcement with identical block states inside/outside; position-sensitive caching; top-down floor/head clearance; safe cancellation and no saved activation; unknown/protected leftovers never reported as completion; selection scope across same-named dimensions/servers; non-solid blocks never becoming support; final server acknowledgment and tool durability.

Pre-flight: Region filtering is applied after state caching, shared controller run identity drives enabled-list/keybindings, and completion bypasses Auto Mine's resource quota only for Area Clear. These interfaces must remain consistent in all adapters.
