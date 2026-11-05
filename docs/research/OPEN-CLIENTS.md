# Open client and compact utility research for Rusty

> Dated reference-mechanism research and design context. Current executable acceptance is recorded only in [Rusty validation](../VALIDATION.md).

> Publication note: authored research summary copied from the private task workspace on 2026-11-03. Raw third-party source, binaries, documentation receipts and assets are excluded. Links to task-local inventories now point to the combined matrix, whose 884 rows include both research scopes. Original upstream source URLs and the original dated observations are retained. See [research scope](README.md) for current migration status.


Research date: 2026-11-03. Source study only. No Java client was built, installed, executed, or connected to a server. No Rusty code, installed application, account, or user state was changed. REA's routing instruction directs complete public-source analysis to ordinary repository tools; binary providers were unnecessary for these two clients.

## Main findings

The best transfers are shared product patterns and a few compact utilities: searchable modules and settings, explicit favorites, grouped ESP filters and styles, world-scoped waypoints, item-component previews, deliberately paced inventory actions, bounded light overlays, and small configurable HUD widgets. Implement these independently against Horizon's received world state and supported native hooks. Java/Fabric mixins and Meteor's Baritone dependency are not native Rust components and are not proposed dependencies.

Wurst and Meteor are both GPLv3 projects. The native Apache Rusty upgrade should use independently written Rust and original assets. The source trees and factual inventories are research references, not material to bundle into the Apache product. Wurst's and Meteor's README licensing statements explicitly require the same license when using their code. This report does not classify an independent implementation as a derivative legal conclusion. See the exact primary [Wurst license][WLicense], [Wurst statement][WReadme], [Meteor license][MLicense], and [Meteor statement][MReadme].

## Frozen identities and complete declaration inventory

| Reference | Exact commit | Commit date | Scope inventoried |
|---|---|---|---|
| Wurst7 | `4e4a957e87ffd79d120835fb47fa9c139006abb8` | 2026-11-03T13:25:38+01:00 | HackList, CmdList, OtfList, their concrete feature classes |
| Meteor Client | `e3a2e77967629d216e127b5ceecf71fb15f9d2e5` | 2026-10-27T21:28:59+01:00 | Modules, Commands, Hud and concrete feature classes |

| Client | Module classes | Commands | Other feature classes | HUD classes | Total rows |
|---|---:|---:|---:|---:|---:|
| Wurst | 156 | 52 | 18 | — | 226 |
| Meteor | 170 | 39 | — | 15 | 224 |
| Total | 326 | 91 | 18 | 15 | 450 |

**170 Meteor module classes is not 170 default-available modules.** `AntiAnchor` exists in source but is absent from the default module registration list. `Excavator` and `InfinityMiner` register only when Baritone is available. Other environment or module runtime checks have not all been audited. All 156 Wurst module classes have textual registry matches. All command/other-feature/HUD rows have textual registry matches. The Meteor text HUD is constructed through the `MeteorTextHud` factory, and the inventory resolves that indirection. Sources: [Wurst registry][WHackList], [Meteor registry][MModules], [Meteor HUD factory][MTextHud].

The published [combined matrix](FEATURE-MATRIX.json) and [TSV](FEATURE-MATRIX.tsv) preserve the 450 open-client name/kind/source records alongside the documented-client research. The original detailed declaration inventories, descriptions, setting declarations and extraction receipts remain private task inputs. Class and registry declarations are not independently observed behavior. The inventory scope is concrete feature classes and their registrations, not every Java mixin, extension, addon, wiki or historical version. Helper/inherited settings were not exhaustively expanded.

Captured Wurst/Meteor rows remain in the [combined TSV](FEATURE-MATRIX.tsv). Each entry retains source identity and research depth. This source inventory is separate from native runtime acceptance. No reference-client implementation snapshot is bundled.

| Wurst module category | Classes | Meteor module category | Classes |
|---|---:|---|---:|
| Render | 37 | Render | 38 |
| Movement | 33 | Movement | 31 |
| Blocks | 26 | Player | 31 |
| Combat | 24 | Combat | 30 |
| Other | 11 | World | 24 |
| Items | 9 | Misc | 16 |
| Chat | 7 | | |
| Fun | 7 | | |
| No category (GUI entry modules) | 2 | | |

## Focused source findings and independently designed transfers

| Area | Observed source behavior | Rusty design implication (inference/proposal) | Primary source |
|---|---|---|---|
| Search and discoverability | Wurst searches feature names, search tags, and translated descriptions; orders matches by first occurrence in those fields; keeps preference counts to order the unfiltered list. Meteor has separate module-title and setting-title results, a configurable result cap, Ctrl/Cmd+F, and a favorites window. | One search field should match native module name, aliases, description, and setting labels. Show why a result matched. Explicit favorites and recent actions are understandable without replicating either layout or ranking code. | [Wurst Navigator][WNavigator], [Meteor module browser][MModulesScreen] |
| Settings | Meteor's typed settings hold default value, validity checks, visibility predicates, change callbacks, activation callbacks, parse/save/load hooks; settings are grouped by purpose. | Make Rust settings the single source of truth for UI, persistence, reset, validation and command interaction. Hide controls only when irrelevant; preserve their values. Display units and constraints. | [Meteor Setting][MSetting], [Meteor StorageESP][MStorage] |
| Storage overlays | Wurst's ChestESP rebuilds groups from loaded block entities and renderable entities on updates, removes listeners and clears groups when disabled, and renders boxes/lines by group. Meteor StorageESP exposes selected storage types, tracer/shape modes, fill opacity and shader width; tracks interacted block positions. | A shared received-storage index can drive boxes, tracers and filter counts. Use per-type colors, max distance, fill/outline and optional opened-state marking. Clear state on world changes. A position alone cannot prove storage contents or ownership. | [Wurst ChestESP][WChest], [Meteor StorageESP][MStorage] |
| X-ray lifecycle | Wurst snapshots visible block identifiers on enable, uses binary search for membership, invalidates chunk extraction on toggle, can restrict to blocks adjacent to non-opaque blocks, and restores brightness depending on Fullbright ownership. Settings explicitly warn that restart is needed there. | Prefer a versioned native filter snapshot with queued mesh invalidation; explain pending rebuilds. Coordinate brightness ownership between modules and restore actual prior state. Batch settings changes before invalidating chunks. The code demonstrates lifecycle concerns, not Horizon implementation compatibility. | [Wurst XRay][WXray], [Wurst Fullbright][WFullbright] |
| Waypoints | Meteor loads per-world files on join, clears session state on leave, stores positions, dimension, name, icon, color, visibility/range, UUID, near action and cross-dimension display preference. Overworld/Nether translation is explicit, not arbitrary teleportation. | Add/edit/delete locally stored marks keyed by stable world/server identity and dimension; death mark and coordinate copy; distance label and visibility toggle. A compass direction label can be an initial native implementation. | [Meteor Waypoints][MWaypoints], [Meteor Waypoint][MWaypoint] |
| Profiles | Meteor selectively saves/loads HUD, macros, modules and waypoints, and declares a server load-on-join list. | Keep module/visual profiles separate from world observation caches. Preview what changes before applying a profile, version the schema and persist atomically. Automatic server matching should be opt-in. | [Meteor Profile][MProfile] |
| Registry/session lifecycle | Meteor rebuilds its command dispatcher on each join because registry-backed argument objects otherwise keep stale server-specific registry identity. | Tag native registry caches and lookup results with a connection/world generation; replace them on reconnect instead of preserving borrowed IDs across worlds. | [Meteor Commands][MCommands] |
| Item previews | Meteor BetterTooltips reads item components for containers, bundles, books, map IDs and more. Ender chest preview uses received-memory data and explicitly renders an unknown-inventory message when absent. | Add an item-details view, durability/count summaries, and a bounded container grid only when contents are present in the received item data. Mark cached contents with origin/time. Never display an unknown container as empty. | [Meteor BetterTooltips][MTooltips] |
| Inventory sorting | Meteor InventorySorter rejects unsupported inventory partitions, hotbar and preview screens; collects the origin partition, plans stack consolidation, then orders remaining items by item ID/count/damage. It executes a move action with tick delay. Nested candidate selection is quadratic in slot count. | Independently design a deterministic stable comparator; maintain component-sensitive stack identity, protected/locked slots, and an immutable action plan. Send only supported normal menu actions, pace them, compare the live menu/session/slot revision before each action, and cancel on close or unexpected update. The source pattern is not proof of server acceptance. | [Meteor InventorySorter][MSorter], [Meteor InventoryTweaks][MInvTweaks] |
| Light overlay | Meteor pools cross objects, computes spawn classifications on ticks via a shared block iterator, then draws only the prepared list, choosing depth-tested or through-block renderer. Its defaults are horizontal radius 8, vertical radius 4 and light threshold 0. | Prefer a bounded surface overlay with received block/sky light, capped scan work and explicit unloaded/unknown handling. Call it a light/spawn-risk aid because server rules, entity checks and changed mechanics can invalidate a universal spawn guarantee. | [Meteor LightOverlay][MLight], [Meteor BlockIterator][MBlockIterator] |
| Shared scans | Meteor BlockIterator merges callbacks into one cuboid scan using the maximum requested horizontal/vertical radius, uses a mutable coordinate and pooled callback objects, then clears registrations each tick. It still evaluates callbacks for each candidate position. | Consolidate native region scans only where they reduce repeated lookups. Add strict radius/work caps, scan counters, incremental dirty updates, cached visible results and scheduling. The shared maximum cuboid can be more expensive for sparse consumers; do not copy it mechanically. | [Meteor BlockIterator][MBlockIterator] |
| HUD | Meteor's registered HUD includes armor, inventory, item, compass, player radar/model, potion timers, map, active modules, module info, lag, combat and keyboard plus text. Text presets include FPS/TPS/ping/speed/position/time and configurable update delay. | Start with a compact native status strip: coordinates/dimension, direction, selected target/count and active module state; optional durability and timing widgets. Expose refresh rate and avoid recompiling/reformatting unchanged data every frame. | [Meteor HUD factory][MTextHud], [Meteor text implementation][MTextImpl] |

These are static observations and design recommendations. No FPS, memory, latency, anticheat acceptance or interoperability measurement was run. The comparison does not establish that Meteor/Wurst's algorithms are faster than current Rusty.

## Additional public source references

The [curated utility license summary](UTILITY-LICENSES.md) records the additional frozen references and review boundaries. Full retrieved source, path inventories and raw license bodies stay in private research receipts and are not bundled into Rusty.

| Project | Frozen commit / primary license | Evidence and useful idea | Boundary |
|---|---|---|---|
| Inventory Profiles Next, historical GitHub source | `a0b16d955128421a6c6f07ef3238d974ebd58785`; [AGPLv3][IPNLicense] | Slot locks/keeper, inventory profiles, replacement/refill hooks, sorting buttons and integration hints are present in the tree. [Locked-slot keeper][IPNLocks] and [auto-refill handler][IPNRefill] inspected. Good specification references for protected slots, loadouts and precise refill eligibility. | [README][IPNReadme] says development moved to Codeberg. This is an archived snapshot, not confirmed current code. The Codeberg page was blocked by browser robots and the attempted public API path returned 404. Current source head remains unresolved; no claim of current behavior. Independently design Rust, no AGPL code adoption. |
| ShulkerBoxTooltip | `68e7f8e280f13f59a18a95f378b4f57b606db5fa`; [MIT][SBTLicense] | [Generic provider][SBTGeneric] handles the container item component. [Shulker provider][SBTShulker] preserves uncertainty for unresolved loot data. [Ender provider][SBTEnder] distinguishes optional server sync from a client cache. | Valuable for item grid previews and unknown/cached states. Server integration is optional in that project and unavailable merely by adding a native client feature. Native Rusty needs its own supported item-component access. |
| LightOverlay | `1eb139be02aeefbe6c116c4cd32b0e3417a28443`; [Apache-2.0][LOLicense] | [Renderer][LORender] traverses a chunk-keyed position/light map, rejects chunks outside range, checks reach/frustum, draws crosses or numerical light levels. | Architecture evidence only. The renderer's cache use was inspected; its ticker's invalidation and background update policy were not audited. No performance result. Compatible license does not imply Java rendering code is useful in native Rust. |
| Item Scroller | `a0384231358af8b5b76b562dd85bd3de427aa09f`; [LGPLv3][ISLicense] | [README][ISReadme] describes moving inventory items by wheel over slots. Native wheel-to-transfer with explicit modifier and direction is a compact interaction idea. | README/metadata-level coverage in this pass; movement implementation and server correctness not traced. |
| MiniHUD | `f3d17d5e24d14379b8e83a0f36848d940246004b`; [LGPLv3][MHLicense] | [README][MHReadme] and renderer inventory show compact information HUD and overlays for light, grids, region boundaries, ranges and other diagnostics. | Mainline/default branch is a pinned reference, not a current-version acceptance claim. Some overlays require world knowledge unavailable to a vanilla remote client. Only received information is proposed for Rusty. |

Sodium was checked as an additional public rendering reference, not used as a proposed component. Its inspected head `6b2732bcf044e906b0f505c1ab37324869060fe2` has a [PolyForm Shield 1.0.0 license][SodiumLicense], not an assumed MIT/Apache/LGPL license. Treat current public source as license-specific source-available research. No Sodium algorithms were traced in this pass, and its performance claims were not adopted as Rusty evidence.

One IPN behavior should specifically **not** be adopted by default: the inspected locked-slot keeper's no-free-unlocked-slot branch can throw a newly picked-up item out of the inventory. For Rusty, protected slots should initially constrain only user-requested organization actions. Any pickup rearrangement or item dropping requires its own clearly exposed behavior; a slot-lock label alone should not imply that policy.

## Practical implementation queue, with acceptance conditions

The following is a proposed priority order, subject to the main investigation's SDK capability audit. It is not a statement that the native hooks already exist.

1. **Search, favorites and consistent settings.** One descriptor catalogue for names/aliases, settings, defaults, constraints and capability availability. Acceptance: every shipped module and setting is searchable; favorites survive restart; keyboard focus and small-window scrolling remain usable.
2. **ESP and x-ray visual controls.** Target/type filters, independent fill and outline, distance caps, exposed-only selection if useful, and visible rebuild status. Acceptance: live block/entity changes update; disabled modules leave no stale overlays or brightness changes; unload/reconnect clears world data.
3. **Waypoints and death markers.** Private local marks, dimension/world separation, direction/distance and copy coordinates. Acceptance: same coordinates in two worlds remain separate; dimension changes never mislabel a mark; reconnect loads only the appropriate world data.
4. **Item details and container previews.** Preview only supplied item components; clearly distinguish unknown, known empty, and cached contents. Acceptance: no unopened ordinary chest-content claim; nested previews have depth/size limits; item names/metadata never execute as commands.
5. **Light overlay and compact HUD.** Bounded scan and render data; depth control; configurable refresh and maximum radius. Acceptance: measurable per-tick and per-frame budgets on the supported native preview, unknown chunks skipped/marked, no whole-world rescan on every frame.
6. **Inventory sorting, protected slots and refill.** Stage only after supported authoritative menu APIs are confirmed. Acceptance: no item loss/duplication across delayed server updates, moving containers, full inventory, damaged or component-distinct stacks, cursor-held stack, disconnect and forced menu close. Stop/cancel on mismatch rather than replaying a stale plan.
7. **Breadcrumbs, last-seen storage and session observation history.** Small bounded ring buffers keyed by world/dimension, with timestamps and observation state. Acceptance: bounded memory, stale labels and explicit clear/export; no unreceived chunk or remote-player inventory inference.

Avoid treating the existence of a combat/movement/packet module in the inventory as proof of a portable feature. Modules that alter movement, attack timing, inventory or packet state remain server-authoritative and environment-dependent. A rendering client cannot recover block states the server withheld or substituted. An optional server protocol or privileged command must be disclosed as such; no client-side feature can establish permission or bypass missing data simply by displaying a toggle.

## Coverage and unresolved questions

- Answered: exact identities/licenses of both principal clients; exhaustive declaration inventory for their module/command/other-feature/HUD roots; default registry anomaly; focused UI, config, visual, scan, waypoint, preview and inventory design patterns; primary source/licensing checks for compact utility references.
- Partial: additional utility mods were sampled rather than exhaustively inventoried; IPN's archived GitHub architecture was inspected, current Codeberg head was not acquired; helper-defined/inherited settings are not exhaustively expanded; runtime preconditions within all 450 features are not audited.
- Unresolved here: Horizon native API availability, current Rusty feature overlap and correctness, real server acceptance, visual QA, performance budgets and regression results. Those require the main Rusty/Horizon code audit and an explicitly scoped native validation pass.


<!-- pinned source links -->
[WLicense]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/LICENSE.txt
[WReadme]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/README.md
[WHackList]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/src/main/java/net/wurstclient/hack/HackList.java
[WNavigator]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/src/main/java/net/wurstclient/navigator/Navigator.java
[WChest]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/src/main/java/net/wurstclient/hacks/ChestEspHack.java
[WXray]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/src/main/java/net/wurstclient/hacks/XRayHack.java
[WFullbright]: https://github.com/Wurst-Imperium/Wurst7/blob/4e4a957e87ffd79d120835fb47fa9c139006abb8/src/main/java/net/wurstclient/hacks/FullbrightHack.java
[MLicense]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/LICENSE
[MReadme]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/README.md
[MModules]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/modules/Modules.java
[MTextHud]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/hud/elements/MeteorTextHud.java
[MModulesScreen]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/gui/screens/ModulesScreen.java
[MSetting]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/settings/Setting.java
[MStorage]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/modules/render/StorageESP.java
[MWaypoints]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/waypoints/Waypoints.java
[MWaypoint]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/waypoints/Waypoint.java
[MProfile]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/profiles/Profile.java
[MTooltips]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/modules/render/BetterTooltips.java
[MSorter]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/utils/player/InventorySorter.java
[MInvTweaks]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/modules/misc/InventoryTweaks.java
[MLight]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/modules/render/LightOverlay.java
[MBlockIterator]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/utils/world/BlockIterator.java
[MTextImpl]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/systems/hud/elements/TextHud.java
[IPNLicense]: https://github.com/blackd/Inventory-Profiles/blob/a0b16d955128421a6c6f07ef3238d974ebd58785/LICENSE
[IPNReadme]: https://github.com/blackd/Inventory-Profiles/blob/a0b16d955128421a6c6f07ef3238d974ebd58785/README.md
[IPNLocks]: https://github.com/blackd/Inventory-Profiles/blob/a0b16d955128421a6c6f07ef3238d974ebd58785/platforms/shared-1.20.5+/src/main/java/org/anti_ad/mc/ipnext/event/LockedSlotKeeper.kt
[IPNRefill]: https://github.com/blackd/Inventory-Profiles/blob/a0b16d955128421a6c6f07ef3238d974ebd58785/platforms/shared-1.20.5+/src/main/java/org/anti_ad/mc/ipnext/event/autorefill/AutoRefillHandler.kt
[SBTLicense]: https://github.com/MisterPeModder/ShulkerBoxTooltip/blob/68e7f8e280f13f59a18a95f378b4f57b606db5fa/LICENSE
[SBTGeneric]: https://github.com/MisterPeModder/ShulkerBoxTooltip/blob/68e7f8e280f13f59a18a95f378b4f57b606db5fa/common/src/main/java/com/misterpemodder/shulkerboxtooltip/impl/provider/GenericContainerPreviewProvider.java
[SBTShulker]: https://github.com/MisterPeModder/ShulkerBoxTooltip/blob/68e7f8e280f13f59a18a95f378b4f57b606db5fa/common/src/main/java/com/misterpemodder/shulkerboxtooltip/impl/provider/ShulkerBoxPreviewProvider.java
[SBTEnder]: https://github.com/MisterPeModder/ShulkerBoxTooltip/blob/68e7f8e280f13f59a18a95f378b4f57b606db5fa/common/src/main/java/com/misterpemodder/shulkerboxtooltip/impl/provider/EnderChestPreviewProvider.java
[LOLicense]: https://github.com/shedaniel/LightOverlay/blob/1eb139be02aeefbe6c116c4cd32b0e3417a28443/LICENSE
[LORender]: https://github.com/shedaniel/LightOverlay/blob/1eb139be02aeefbe6c116c4cd32b0e3417a28443/common/src/main/java/me/shedaniel/lightoverlay/common/LightOverlayRenderer.java
[ISLicense]: https://github.com/maruohon/itemscroller/blob/a0384231358af8b5b76b562dd85bd3de427aa09f/LICENSE.txt
[ISReadme]: https://github.com/maruohon/itemscroller/blob/a0384231358af8b5b76b562dd85bd3de427aa09f/README.md
[MHLicense]: https://github.com/maruohon/minihud/blob/f3d17d5e24d14379b8e83a0f36848d940246004b/LICENSE.txt
[MHReadme]: https://github.com/maruohon/minihud/blob/f3d17d5e24d14379b8e83a0f36848d940246004b/README.md
[SodiumLicense]: https://github.com/CaffeineMC/sodium/blob/6b2732bcf044e906b0f505c1ab37324869060fe2/LICENSE.md
[MCommands]: https://github.com/MeteorDevelopment/meteor-client/blob/e3a2e77967629d216e127b5ceecf71fb15f9d2e5/src/main/java/meteordevelopment/meteorclient/commands/Commands.java
