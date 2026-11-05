# Impact, Future, and RusherHack: public documentation research

> Dated reference-mechanism research and design context. Current executable acceptance is recorded only in [Rusty validation](../VALIDATION.md).

> Publication note: authored research summary copied from the private task workspace on 2026-11-03. Raw third-party source, binaries, documentation receipts and assets are excluded. Links to task-local inventories now point to the combined matrix, whose 884 rows include both research scopes. Original upstream source URLs and the original dated observations are retained. See [research scope](README.md) for current migration status.


Observed 2026-11-03. This report is one part of the Rusty upgrade investigation. It covers official public documentation and a small, explicitly identified public API-source sample. It does not report execution of any reference client, decompilation of a proprietary client, comparative benchmarks, or a completed feature port.

## Inventory coverage and evidence boundaries

The machine-readable inventory is [documented-features.tsv](FEATURE-MATRIX.tsv), with the same records and metadata in [documented-features.json](FEATURE-MATRIX.json). There are **434 client/feature occurrences**, not 434 distinct capabilities:

| Client | Captured occurrences | Coverage boundary |
|---|---:|---|
| Impact | 117 modules, with 540 nested setting-name paths | All eight categories in the official feature page's embedded `features` object. Mode names are captured separately. The page can lag releases. |
| RusherHack | 166 modules, 35 commands, 32 HUD elements | Entire official landing inventory. Selected per-feature pages were read through the web tool. Direct HTTP retrieval of those pages returned 403; all settings were therefore not extracted. |
| Future | 22 Combat modules; 59 additional named modules/subsystems mentioned in official changelogs; 3 expressly removed modules | The publicly rendered default category is Combat. The page names seven categories, but the other six inventories were not exposed by the accessible extraction. Changelog evidence is historical evidence and is marked accordingly. |

Sources: [Impact features](https://impactclient.net/features), [RusherHack features](https://rusherhack.org/features/), [Future homepage and changelog](https://futureclient.net/).

For RusherHack and Future's changelog-only entries, `category` is an **analyst comparison taxonomy**. It is not asserted to be the reference client's own category. Empty `settings` arrays mean settings were not extracted; they do not mean a feature has no settings. Impact's `source_category` is the actual embedded category. Removed Future modules must not be counted as current availability.

The inventories are broad enough to guide architecture and backlog work. They are **not an exhaustive inventory of every current paid-client build, setting, command, plugin, or hidden implementation**. No matching full proprietary source release was established. No purchases, account logins, license bypasses, or third-party client execution were performed.

Local raw HTML/text/API-tree snapshots are under `documented-snapshots/`; their SHA-256 values are in `documented-snapshots/manifest.json`. These research receipts contain third-party text and source and should remain local rather than be copied into Rusty's published source distribution. Publish the original comparison, factual name inventory, and source links if needed.

## Version and compatibility observations

Impact's current public homepage lists Minecraft through **1.16.5** and redirects users seeking newer versions toward Future. Its feature data does not establish Minecraft 26.3 or Horizon compatibility. Its public changelog also adds Randomize and Capes outside the captured 117-entry feature object, demonstrating that the feature page and changelog are different inventories. [Impact homepage](https://impactclient.net/), [Impact changelog](https://impactclient.net/changelog).

Future's homepage lists **v3.0 for Minecraft 1.21.11**, dated March 15, 2026; its complete v3.0 changelog points to a forum page that could not be read in this investigation. Its own older changelogs describe compatibility failures and removed functionality, so old server-specific claims cannot be treated as current capabilities. [Future releases and changelog](https://futureclient.net/), [v3.0 forum link](https://www.futureclient.net/forum/showthread.php?tid=60319).

RusherHack's public changelog lists **v2.0.8**, June 2, 2026, with Minecraft 1.21.11 support and 26.1/26.2 support in progress. The same entry says support for older 1.20.1–1.21.4 versions will no longer receive feature updates. This is documentation, not proof of support for Rusty's Minecraft 26.3/Horizon target. [RusherHack changelog](https://rusherhack.org/changelog.html).

## Comparison by capability family

| Family | Evidence-supported reference ideas | Practical independent Horizon design | Required acceptance evidence |
|---|---|---|---|
| X-ray and block search | All three document block visibility/search features; Impact exposes result limits and refresh controls; RusherHack has block/entity/item selectors and alerts. | Preserve Rusty's known-block registry filters and finite scan limits. Share one incremental scanner among finder, storage and spawn overlays; invalidate on world/chunk changes. Display partial-result state. | Known test blocks at radius/height boundaries; unloaded chunks; anti-X-ray server; reload and dense selection; scan timing and result cap. |
| Entity/storage ESP | Impact offers box, shader, skeleton and 2D styles. RusherHack documents entity and storage type filters and several render modes. | Start with stable native box/tracer passes and typed filters, using one entity snapshot per frame. Add more costly effects only after measuring their render integration. | Visible and occluded targets, removed entities, dimensions, third-person/freecam, resource reload and GPU timing. |
| Camera and visibility | Freecam, zoom and no-render controls recur; reference changelogs document camera and input conflicts. | Keep server player state separate from detached camera state; explicit depth policy and render-state restoration. Handle zoom as a release-safe held action. | Player continues normal simulation, death/respawn, GUI focus, F1, view changes, resize and restoring every modified visual state. |
| Movement | Sprint, edge safety, step, flight, elytra and liquid movement recur, with strongly version/server-specific behavior. | Implement input assistance through available native hooks. Label actual game-authorized flight separately from any speculative movement request. Centralize input ownership. | Authoritative position/velocity comparison on a controlled server; collision, reconnect, lag and rejection. |
| Inventory | Armor/totem/tool selection, refill, container transfer, crafting and cleaning recur. RusherHack exposes delays and screen scope. | First require validated inventory access and server-confirmed transactions. Use a serial transaction queue with revision checks, cancellation, and recovery from rejection. | Slot revision changes, server rejection, high latency, close/open container, death, full inventory and competing modules. |
| Building and automation | Scaffold, mining, farming and pathing recur; many wrappers depend on Baritone. | Add shared placement/mining primitives and visible previews before destructive automation. Time budgets, world snapshot checks, user cancel and explicit progress belong in infrastructure. | Reach, support-face/collision correctness, wrong block replacement, block updates, missing materials and server rejection. |
| Navigation | Impact wraps Baritone settings; RusherHack documents waypoint persistence and pathing integration. | Native waypoints and breadcrumb storage are useful before a full planner. Native A* requires collision costs, loaded-world boundaries and executable movement validation. | Dimension/server scoping, path cancellation, unknown terrain, blocked route, falling/liquids and repeated replanning. |
| Combat | Target filters, aim/aura, crystal planning, healing and equipment are widely documented. | Require authoritative interaction APIs, cooldown and line-of-sight semantics before implementation; use shared selection/rotation arbitration and simulation fixtures. | Controlled-world targets, friends/team exclusions, target disappearance, cooldowns, reach checks and server acceptance. |
| HUD/config/keybinds | Module lists, position, time, metrics, bind lists, profiles and themes are common. | Build from declarative module metadata, one shared settings model, stable IDs and versioned profile migrations. Search names, descriptions and setting labels. | Input capture, bind conflicts, restart persistence, invalid profiles, scale/resize, keyboard-only operation and visible HUD bounds. |

Representative primary sources for the table: [RusherHack ESP](https://rusherhack.org/features/modules/ESP.html), [Search](https://rusherhack.org/features/modules/Search.html), [AutoSorter](https://rusherhack.org/features/modules/AutoSorter.html), [InventoryCleaner](https://rusherhack.org/features/modules/InventoryCleaner.html), [Waypoints](https://rusherhack.org/features/modules/Waypoints.html), [Baritone integration](https://rusherhack.org/features/modules/Baritone.html). The third and fourth columns are proposed Rusty engineering decisions and acceptance criteria, not claims that the references implement these mechanisms.

## UI and configuration findings

Impact's documented ClickGUI controls include scale, grid snapping, opacity, blur, tooltips and typed-query highlighting. Its changelog records search typing activating binds and an opening bind entering the search field. Those are useful concrete regression cases for a native overlay. Its historical preset commands and macro GUI also show that profile/macro management need discoverable controls. [Impact features](https://impactclient.net/features), [Impact changelog](https://impactclient.net/changelog).

Future's public changes document per-panel scrolling, a search bar, keyboard number entry/arrow adjustments and color editing. These are transferable interaction ideas; the implementation and actual latency remain unknown. For Rusty, a single search-focused list with a details pane is more manageable than opening hundreds of floating controls. Favorites/recent use are independent design proposals, not observed reference features. [Future changelog](https://futureclient.net/).

RusherHack documents a configurable ClickGUI shortcut (default **P**), a separate window manager, HUD customization and command-based bind management. Theme controls expose monitor scaling, tooltip delay/wrapping, focus dimming and optional blur. Its changelog documents atomic config saves where supported and module settings that persist across profiles. [FAQ](https://rusherhack.org/faq.html), [ClickGUI](https://rusherhack.org/features/modules/ClickGUI.html), [Theme](https://rusherhack.org/features/modules/Theme.html), [Windows](https://rusherhack.org/features/modules/Windows.html), [bind command](https://rusherhack.org/features/commands/bind.html), [changelog](https://rusherhack.org/changelog.html).

For Rusty, the highest-value UI work is a Right Shift overlay that captures input reliably, closes with Escape or the rebound shortcut, releases held actions on focus loss, keeps scrolling inside the hovered pane, and searches setting descriptions as well as module names. Use the existing native settings as a fallback with the same underlying state. A binding editor should show conflicts and distinguish toggle, hold and one-shot actions. A profile operation should validate before applying and write atomically. These are native design recommendations, not verified implementation status.

## Source-observed API patterns and license boundaries

**Impact ClientAPI** is an archived public project, not established as the current closed client's implementation. GitHub reports commit `9d52943cd85b650a2782d99fadd3a2e0df8fef24` from November 16, 2018. The actual checked license file is Apache-2.0. The inspected key handler dispatches press/release actions through a keybind registry, handles toggle clicks separately, and checks cancellation for key-down dispatch. Its module annotation carries names, descriptions and default binds. These demonstrate a public API pattern only. [Pinned key handler](https://github.com/ImpactDevelopment/ClientAPI/blob/9d52943cd85b650a2782d99fadd3a2e0df8fef24/src/main/java/clientapi/event/handle/KeybindEventHandler.java), [pinned module metadata](https://github.com/ImpactDevelopment/ClientAPI/blob/9d52943cd85b650a2782d99fadd3a2e0df8fef24/src/main/java/clientapi/module/annotation/Mod.java), [actual license](https://github.com/ImpactDevelopment/ClientAPI/blob/9d52943cd85b650a2782d99fadd3a2e0df8fef24/LICENSE).

**RusherHack example-plugin** was inspected at commit `986db6c411a4f2312672cbd3e8f790050c83d38f`, March 28, 2026. Its example source demonstrates typed settings, conditional visibility, change callbacks, nested settings, lifecycle methods and separate 2D/3D render/update subscriptions. That sample does not reveal internal aura, scanning, networking or prediction algorithms. Crucially, its actual LICENSE says **All rights reserved**; it is not an MIT/Apache template merely because it is public. No code/assets from it should be adopted into Rusty. [Pinned example module](https://github.com/RusherDevelopment/example-plugin/blob/986db6c411a4f2312672cbd3e8f790050c83d38f/src/main/java/org/example/ExampleModule.java), [actual license](https://github.com/RusherDevelopment/example-plugin/blob/986db6c411a4f2312672cbd3e8f790050c83d38f/LICENSE).

RusherDevelopment's public plugin collection links many third-party additions, but explicitly does not establish that they are verified, endorsed or safe. Its static metadata API is a discovery aid, not a license grant or a correctness audit. The registry was not folded into the core-client counts; every candidate would need its own repository, commit and actual license review. [Official-organization plugin collection](https://github.com/RusherDevelopment/rusherhack-plugins), [metadata API description](https://github.com/RusherDevelopment/rusherhack-plugins/blob/main/API.md).

The public ImpactServer repository describes website behavior and has no detected license in the metadata examined; that does not license the Impact client. RusherHack documentation/assets repositories similarly had no detected root license in the checked trees. The proprietary reference clients and their art/UI assets are not cleared for copying. Implement behavior independently and retain attribution/license notices for any separately approved open-source material.

## Authority limits and performance evidence

Rusty's current README already states the correct boundary: it operates on client-received data; a detached camera does not move the authoritative player or request new distant chunks. Therefore X-ray cannot reconstruct blocks a server omits or substitutes, ESP cannot reveal unsent entities, and cached data may be stale. Tooling must show unknown/partial states rather than inventing world knowledge. See `work/rusty-upgrade-repo/README.md` and `docs/VALIDATION.md` for the separate Rusty evidence record.

Movement, combat reach/cooldowns, item use, inventory changes and block interactions remain subject to server acceptance. Java mixins, Fabric events, Baritone and ViaFabricPlus are not native Horizon APIs. Their feature names do not establish an equivalent Rust hook. The references' dated anti-cheat-specific claims should be treated as historical compatibility notes, not promises of bypassing a current server.

No FPS, CPU, memory, input-latency or feature-reliability benchmark was performed on the references. Marketing claims of superior performance remain claims. Changelog reports of less allocation or faster code do not supply reproducible comparative results. Useful native priorities are bounded scanning, cached metadata/search indices, shared snapshots, batched drawing, explicit frame budgets and measurable acceptance tests; superiority can only be claimed after a controlled baseline comparison.

## Recommended backlog order

1. Declarative metadata/settings registry, searchable live overlay, focus-safe hotkeys, favorites/recent state, atomic versioned profiles and reliable fallback settings.
2. HUD telemetry and native local utilities: active modules, position/direction, FPS, memory if supported, breadcrumbs, waypoints, crosshair and block/entity readouts.
3. Shared bounded world/entity scanning and rendering services; storage filters, target search, tracers, visibility controls and partial-result diagnostics.
4. Native capability audit for inventory, input, interaction and network hooks; transaction/rotation/input ownership before competing automation modules.
5. Incremental placement/mining/pathing/automation after previews, cancellation and authoritative acknowledgements are tested. Keep unsupported features in a documented backlog, outside the runnable module count.
6. Real controlled singleplayer/multiplayer validation and performance comparison before stable packaging or claims about reference parity.

This order addresses the references' recurring strengths while avoiding a large list of labels backed by absent or unverified behavior.
