# Java edition validation — 1.3.0-beta.1

Horizon native 1.2.0 has its own [accepted report](VALIDATION.md). This report applies to the separate Minecraft 26.3 Fabric/NeoForge counterparts. No personal world, public-server character, account inventory or launcher installation was used as a Java test fixture.

## Toolchain and artifacts

- Minecraft 26.3, Java 25 (Temurin 25.0.5+8), Gradle 9.8.2 with distribution checksum.
- Fabric Loader 0.20.1, Fabric API 0.165.2+26.3, Loom 1.19.3.
- NeoForge 26.3.0.101, ModDevGradle 2.0.148.
- Both loader runtime/source jars build. `java/verify_artifacts.py` checks version/manifests, source completeness, notices, and absence of QA/Minecraft/loader classes in installable packages. See [checksums](../java/SHA256SUMS).
- Shared core: **555 behavioral assertions**, run as both a JUnit acceptance entry and a plain Java executable. These cover permission lifecycle, hold/toggle/focus/quick taps, config limits, cache ownership, actual settings-file replacement and nonoverlapping HUD packing.
- The QA mod is a separate development source set. It is excluded from runtime and source jars. Its guarded fixture refuses personal game paths/worlds and non-loopback remote addresses.

## Actual game checks

Both loaders launched through their supported development toolchains and joined an exact, copied disposable `RustyLab` world. Each completed **44 in-game checks**, including:

- Real Minecraft widgets and recovery access with a cleared menu binding; actual local settings/profile files; independent 18-panel HUD bounds, default packing and hotbar clearance.
- Original-texture quad generation, exposed and buried received ores, removal/restoration, custom target selection, target caps/partial state, minimum/maximum context and off/on restoration.
- Detached camera movement and rendering while preserving body position/rotation; chunk unload/reload, dimension transitions and reattachment in the new dimension.
- A harvest-correct tool replacing a faster ineligible golden pickaxe; manual hotbar ownership; ordinary eating and slot restoration; empty-offhand totem transfer, shield preservation, empty-slot armor, exact-stack refill and an ordinary reachable living-target attack.
- A synthetic authoritative inventory correction remains stopped for 25 ticks; explicit off/on rearm then retries the retained observed refill candidate. This exercises correction-state handling; it is not a measurement of every possible server rejection sequence.
- All registered adapters enabled in a received world for smoke/crash coverage, with bounded overlay output and a 30-second final rendering observation. A batch smoke check is not per-module functional proof.

**Physical input:** short X taps were observed to activate and restore textured terrain in both loader clients. The polling-only prototype missed a brief tap; the released implementation retains press events with parity and clears them at world/connection boundaries. Hold/release/focus behavior is covered by shared-controller checks; long-held physical-key automation is not claimed. Windows automation did not establish a modifier-only Right Shift press; menu default 229 is the official SDL Right Shift scancode, with Options recovery independently checked.

## Dedicated server checks

Separate official Minecraft 26.3 server processes bound to loopback were used; these are actual TCP client/server tests, not an integrated-world substitute.

| Server/client | Observed result |
|---|---|
| Unmodified vanilla server → Fabric client | Joins without Rusty on the server; restricted requested tools stay inactive; ordinary information overlays render. |
| Unmodified vanilla server → NeoForge client | Same behavior. |
| Optional Fabric server component → Fabric client | Authoritative `view` grant enables real received target scanning; `combat` stays denied; live revocation disables the view. |
| Optional NeoForge server component → NeoForge client | Same behavior. |

Each permissioned-client run also rejects malformed payload versions, masks and lengths and accepts the valid two-byte payload. The receiver captures its actual listener; stale listeners cannot relabel a grant as belonging to a replacement connection. Permissions are not stored in profiles. Reconnect resets are covered by core ownership checks and source review; no claim of every public-server/proxy topology is made.

An early permissioned-server run failed because a grant could arrive before client ownership initialization, or the server could mark an unavailable channel as delivered. The client now retains only a current-listener grant until initialization; the server retries until the channel actually accepts delivery. Both matching loader/server pairs passed after those corrections. Fabric's final receiver is per connection, registered before channel advertisement; NeoForge uses its payload context listener.

## Performance and review

Work is capped: received target index 4096 maximum; scanner 4096–32768 cells and 32 palette checks per tick; overlay snapshot 768 primitives; entity inspection 512; storage inspection 512 and 128 matches; minimap 32 of 576 cells per tick; A* 4096 nodes/128-block route; breadcrumbs 512 samples. Dense/unknown areas can be partial. Texture frames emit more geometry than ordinary full faces and may reduce frame rate.

The accepted raw receipts retain sampled Minecraft FPS, Java heap usage and counts during the fixture. This is a short observation on one Windows/Intel Iris Xe machine, not a cross-hardware benchmark, frame-time distribution, long endurance test or proof of no leaks. No superiority over other clients is claimed. Horizon's separate native ten-minute run does not establish Java endurance.

An independent whole-branch review identified and rechecked harvest eligibility, Freecam/minimap world state, explicit action rearming, refill-candidate ownership, storage inspection limits and minimum scan budget. Follow-up review covered actual receiver identity, successful channel delivery and queued-key lifecycle. Material findings were corrected before release.

## Explicit limits

- Only Minecraft 26.3 and the pinned loaders are validated. Earlier versions, snapshots, third-party rendering replacements/shaders, mixed-loader server transports and arbitrary public servers are unverified.
- Auto Fish's received-bite path, full pathfinding/build-placement scenarios, every movement edge and every individual cosmetic/overlay option have source/smoke coverage rather than exhaustive end-to-end action cases. Do not read 42 catalog entries as 42 individually certified features.
- Ordinary server corrections, reach, cooldowns, inventory and health remain authoritative. Restricted Java features require real server opt-in; the unmodified-server tests prove denial, not unrestricted X-ray there.
- Received surface minimap, approximate block-only trajectories, sampled light warnings and manual-cast reeling have the edition-specific limits in the [Java guide](../java/README.md).
- Remeshing completes through the game's normal asynchronous chunk rebuilds; a toggle changes effective state immediately but full terrain rebuilding need not finish in the same frame.
- Server anti-X-ray may replace or withhold block information. Rusty does not obtain it. No personal or public server was used to test bypasses.

## Evidence

Sanitized [game receipts](evidence/java) include actual check names, times, counts, short performance samples and SHA256 references to preserved raw receipts. They are readouts, not generated success summaries. Original local logs/source snapshots are retained privately; account/device paths are not published. [Original Java screenshots](screenshots/java) are captured with Minecraft's F2 function, without retouching. Build Actions provide independent compilation/package checks, not live graphics proof.
