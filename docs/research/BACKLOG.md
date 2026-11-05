# Historical planning backlog and acceptance gates

> Dated reference-mechanism research and design context. Current executable acceptance is recorded only in [Rusty validation](../VALIDATION.md).

This **2026-11-03 planning snapshot** is retained for provenance. It is superseded for implemented features by the [module guide](../MODULES.md), [continued implementation analysis](implementation-analysis.md), completion plan and exact [validation record](../VALIDATION.md). Proposed rows below are not claims that current implementations remain absent, or that a feature name is tested.

Research/design date: 2026-11-03. These are proposed work items and explicit gates, not a delivery checklist marked complete. Consult the [matrix](FEATURE-MATRIX.tsv) for every captured reference occurrence and the implementation plan for current engineering sequencing. A feature becomes a runnable module only after its implementation and acceptance exist; unsupported names remain documentation.

## Foundation and SDK gate

The current Rusty host pin is in compatibility.json. Use the module guide for actual capabilities and fresh validation for accepted behavior; this dated planning snapshot is not an implemented-feature count.

P0 requires exact-pin mixin application, a successful native build, artifact identity, a separate real-game launch and all baseline regressions. Compilation alone is insufficient. Before building new behavior, establish a module metadata catalogue, shared settings validation/persistence, explicit input ownership, bounded render/scan work and world/session generations for caches.

The native over-world screen should own UI input while single-player simulation continues. Screen hooks have read-only game access; queue bounded operations and apply mutable actions through the appropriate frame/update path. A visual overlay must not itself call unsupported private methods, move inventory slots or fabricate authoritative events.

## Category priorities

| Category | Priority and proposed modules/work | Native capability prerequisite and authority boundary | Concrete test gate |
|---|---|---|---|
| HUD | P0 search/category/details overlay; direct toggles; favorites/recent actions; keybind editor; profiles. P1 direction, equipment and timing widgets. | One shared metadata/settings model; native screen/input contract; read-only item state for equipment. User-facing modules must be implemented, not placeholder toggles. | Right Shift opens/closes without repeats or action leakage; Escape nesting; text/IME, wheel and controller focus; F1, resize/scale; world ticks continue; preferences survive restart; failed profile input preserves valid state. |
| Rendering | P0 preserve X-ray, terrain frames, finder and camera. P1 storage/type presets, light-risk overlay, richer target readouts, fill/outline/range controls. | Read only client-received blocks/entities/light; share bounded scans and render snapshots. Anti-X-ray replacement and unloaded world data remain unknown. | Correct known/unknown/empty distinctions; radius/height/filter edges; live block/entity removal; dense caps and partial labels; reconnect/dimension clear; depth/render-state restoration; measured scan/render budgets. |
| Navigation | P1 world/server/dimension-scoped waypoints, compass, coordinate copy, bounded breadcrumbs and confirmed death markers. P2 native path planner only after collision and movement validation. | Stable world identity and received geometry. Coordinate translation is a display operation, not teleportation or knowledge of unloaded terrain. | Identical positions in separate worlds remain separate; dimension changes; mark edits/deletes; clipboard bounds; ring-buffer memory; death/reconnect; blocked and unknown routes; cancellation without a stale movement key. |
| Inventory | P1 read-only equipment/durability and item-component container previews. P2 protected slots, sorting/refill/tool selection after normal action primitives are validated. | Received item components; distinguish unknown from known-empty containers. Mutation needs menu/session identity, revision-aware ordinary actions and server acknowledgements. | Never claim unopened chest/player inventory knowledge; nested preview depth cap; component-distinct items; damaged/unbreakable gear; cursor-held stack; full inventory; delayed rejection, forced close, disconnect; no item loss/duplication. |
| Utility | P1 contextual help, session diagnostics, local profiles and telemetry. P2 notification/macro utilities with explicit scope. | Bounded local state, parsers and storage; separate private world observations from reusable profiles. No external service is silently required. | Invalid/oversized inputs, failed writes, migration/reset/import, stale caches, output escaping, bounded log growth; macros cancel at screen/world transitions. |
| Movement | P0 preserve auto-sprint and camera suspension. P2 edge/input aids; P3 step/flight/elytra/liquid features after native semantics are established. | Input/velocity/collision hooks and authoritative correction handling. A client request cannot guarantee a server-accepted position or flying permission. | Local controlled course with ledges, liquids, vehicles and lag; real server correction; focus loss and overlay-open; death/reconnect; no sticky input. Test each mode independently. |
| Build | P2 placement/mining preview and shared selection primitives. Scaffold, schematic assistance, repeated mining and bulk operations wait for those primitives. | Reach/raycast/collision/support-face checks, materials, authoritative block updates and safe cancellation. Received geometry may change during execution. | Replaceable/non-full blocks, world height/border, missing materials, server rejection, obstruction, user cancellation and reconnect; no stale plan sends later actions. |
| Automation | P2 food/fishing/farming helpers only after eligible-item and normal-action semantics. P3 multi-step mining/path/farm pipelines. | Shared input, rotation and inventory ownership; action pacing and server-confirmed state. Java Baritone/Fabric cannot be assumed available in native Rust. | User interruption and competing modules; item use in progress; inventory changes; empty resources; stalled server; target unload; bounded retries/timeouts; stop before an invalid action. |
| Combat | P3 target/rotation/attack/healing/equipment investigation; no count-driven registration. | Available authoritative interaction APIs, line-of-sight/reach, current cooldowns and target identity. Historical anti-cheat claims are not current guarantees. | Controlled disposable targets; friend/team exclusions; target despawn/death; server rejection/cooldown; yaw/pitch bounds; interaction conflicts; damage estimates clearly labeled when server data is missing. |

## Baseline module-specific regression gates

The following are the 12 current module families, not additional new features. Each needs fresh acceptance after a host/mixin migration.

| Existing module | Required check |
|---|---|
| `xray` | Toggle and hold; target add/remove/filter; 0%, intermediate and 100% terrain texture behavior; rebuild completion; disable returns ordinary native terrain/lighting. |
| `finder` | Known placed targets, explicit count/result cap, scan range/height limits, refresh and changed-block invalidation; dense/unloaded regions show partial or unknown state. |
| `entity_esp` | Known entities enter/leave range, despawn and change dimension; correctly aligned geometry through camera changes; disabled state draws nothing. |
| `fullbright` | Enable/disable, ownership with X-ray, resource reload and world changes; restore the actual prior brightness. |
| `no_fog` | Fog changes only while enabled; reload, biome and dimension transitions restore the expected host state. |
| `clear_weather` | Visual suppression restores on disable and does not claim to alter authoritative weather or gameplay conditions. |
| `zoom` | Hold/release, focus loss, overlay and other screens, rebound shortcut; no latched zoom after reconnect or window activation. |
| `coordinates` | Player position/dimension readout, formatting and camera-versus-body labeling; never attribute detached-camera position to the server player. |
| `auto_sprint` | Normal forward input, blocked/using states and server correction; never runs movement beneath settings, on focus loss or after disconnect. |
| `no_hurt_camera` | Controlled local damage confirms camera effect only; damage/knockback and server health remain authoritative. |
| `freecam` | Body remains simulated; no distant chunk loading claim; active-use opening policy, focus/menu suspension, speed/collision, perspective restore, death and world changes. |
| `status_hud` | Correct active state, readable bounds at small/high scale, F1 policy, refresh behavior, profile change, no player and disconnected/title states. |

## Evidence required before release claims

Record pure tests, mixin application, native compilation, package parsing, launch, visual/input checks, controlled feature fixtures and multiplayer acceptance as separate results. Include artifact/commit identity, host/mixin pins, test world/server conditions and unresolved failures. Keep screenshots and timing samples tied to the final tested build.

Performance comparison needs an original installed baseline and repeatable within-build feature-off/on scenarios. An engine or mod-stack change limits causal claims across builds. Report FPS distribution, CPU/GPU frame timing, memory, scan work/result caps and input response where measurable. Do not infer better performance from fewer lines of Rust, advertised reference speed or a passing unit test.

A stable release requires all applicable final-binary and CI checks plus real game acceptance. A branch or review PR with pending gates must say so. This folder does not establish that the ambitious complete client upgrade or reference parity is finished.
