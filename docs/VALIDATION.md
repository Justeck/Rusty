# Rusty validation

## 1.2.5 defaults and Enabled Mods list

Added **Enabled Mods**, an independent, opt-in Interface feature: active Rusty tools appear alphabetically in top-left rows, excluding the list itself. Its nineteenth HUD panel supports independent placement and size. **Default** controls restore individual preferences, shortcuts and target fields. **Restore defaults** asks for confirmation and restores all Rusty settings, including keys, appearance, compact 100% menu size, favorites/recent and HUD layout; it stops active runtime tools and keeps saved waypoint/profile files.

The official Horizon launcher **Performance** build completed on **38609b7 / API 409 / Minecraft 26.3**, using its supported Rust **1.99.1** toolchain. Selected build: `native-38609b7-2bbd64b7ea25`. Executable SHA-256: `596e84c91e8a43996b19a018ec20568677776a5f72334f35c93eb64adde48090`.

Fresh checks passed: **146 Rust library + 18 native-tool tests**, **14 Python tests**, formatting, strict Clippy, exact SDK compilation and structural package verification. The launcher-built executable passed **9 defaults/list game checks**, **11 interface/layout checks** and **5 rendered X-ray/Finder regressions**. The defaults suite exercises actual buttons for booleans, numeric values, shortcuts and HUD positions; target-editor recovery from invalid color text; Cancel; confirmed full reset with active X-ray/Freecam; preserved saved profile and real waypoint bytes; and active-list update/removal. The interface suite covers all 19 HUD controls, menu sizing and owned-process restart persistence.

Both Java editions passed fresh builds, package checks, **692 shared assertions** and **65 actual game checks per loader**. See [Java validation](JAVA_VALIDATION.md) and the [exact receipt](evidence/defaults-list-1.2.5.json) for identities and named checks.

![Actual native per-setting Default controls](screenshots/rusty-1.2.5-default-controls.png)

![Confirmed full reset in the native menu](screenshots/rusty-1.2.5-restore-confirmation.png)

![Native top-left active-list gameplay](screenshots/rusty-1.2.5-enabled-mods.png)

These fresh game checks use owned offline fixtures. They do not establish fresh external-server, arbitrary-modpack, endurance or alternate-backend compatibility. The build uses the official launcher CLI; no fresh GUI Build/Play claim is made. Existing user games/worlds remain untouched, and the reset tests do not reset installed personal preferences. Historical results below retain their own version and scope.


## 1.2.4 compact menu baseline

The former 70% physical menu size is now the **100% default/reset**. Displayed scaling spans **50–300%** (stored physical scales 0.35–2.1). Stored values stay in their original physical units, preserving every previously supported saved size without a lossy schema migration. HUD size and feature-profile preservation remain independent. Java adds a 100% reset and applies changes after slider release or keyboard adjustment; the existing Apply size button remains available.

![Original native capture of the new compact 100% menu, filtered to Fullbright](screenshots/rusty-1.2.4-compact-menu.png)

The official Horizon launcher **Performance** build completed on **38609b7 / native API 409 / Minecraft 26.3**, using its supported Rust **1.99.1** toolchain. It selected `native-38609b7-e07ca05ff764`; executable SHA-256 is `57077237e5406d16bc95e423684a598b2a7cb821506ccd14d2f530f71f0b302c`.

Fresh checks passed: **142 Rust library + 18 native-tool tests**, **14 Python tests**, formatting, strict Clippy, structural packaging, **11 actual native interface checks** and **5 rendered X-ray/Finder regression checks**. The interface suite checks the fresh compact default, both new endpoints, actual hit alignment, reset, all 18 HUD controls, profile preservation and process-restart persistence. Original default/50%/300% menu frames were inspected. The matching Java beta passed **621 shared assertions** and **53 game checks per loader**, including actual screen-transform/widget bounds at all three menu sizes.

The [exact receipt](evidence/menu-scale-1.2.4.json) identifies the tested source, binary, package and named checks. These fresh fixtures used isolated offline worlds. Older server/performance/endurance results below remain historical. The native build used the official launcher CLI; no fresh GUI Build/Play test is claimed for this patch. Existing running user games and personal saves were left untouched.


The [published-release installation receipt](evidence/public-release-1.2.4.json) verifies the repository URL and fixed 1.2.4 URL through Horizon's official installer. The public native package is byte-identical to the tested package. A final Performance build exited 0 in **13.609 seconds**, reported **Up to date**, reused the accepted executable and left the sole `horizon` instance selected. The existing Rusty preference file remained byte-identical, including its physical menu scale. Both Modrinth runtime downloads match the tested/GitHub jars byte-for-byte; the project remains **Under review**. Existing running user games were left untouched and need a restart to load the patch.

## 1.2.3 independent X-ray and Finder

X-ray now changes terrain only. Finder separately owns target boxes, tracer lines and target readouts. Saved line preferences remain intact; enabling or disabling X-ray does not change Finder. Original-texture terrain frames remain part of X-ray.

The official Windows launcher **Performance** build completed successfully on **Horizon 38609b7 / API 409 / Minecraft 26.3**, using its supported Rust **1.99.1** toolchain. Compilation took **13m 38s**; the launcher reported **26m 4s** including staging/installation. The sole `horizon` instance selected `native-38609b7-62e1f0c09f3c`. The accepted executable SHA-256 is `3aefe2228d77e803b1b6a4a169a579d65c622031476aa7986aaa5b431c23d0f8`.

Fresh actual game checks on that executable passed: **5 X-ray/Finder independence checks**, **9 feature checks** and **10 interface checks**. The rendered regression keeps the HUD/line layer visible and proves both directions: X-ray alone is unchanged with saved lines/boxes on or off, while explicitly enabled Finder visibly renders tracers with X-ray both on and off. Hotkey toggles preserve line preferences and received block data. Original paired frames were reviewed. Drifting clouds initially contaminated one comparison despite frozen world time; clouds were disabled only in the disposable fixture, without relaxing thresholds. Failed receipts remain separate.

**141 Rust library + 18 native-tool tests**, **14 Python tests**, formatting, strict lint, structural packaging and pinned-SDK CI checks passed. The [exact receipt](evidence/xray-independence-1.2.3.json) records source, executable, package and named game checks. The matching Java beta passed **49 checks per loader** and **609 shared core assertions**.

These fresh checks use isolated offline worlds. Earlier server/performance/endurance results below remain historical. The build used the official launcher CLI; this patch does not claim a fresh GUI Build/Play test. No existing user game or public-server character was controlled.


The [published-release installation receipt](evidence/public-release-1.2.3.json) verifies both the fixed 1.2.3 URL and repository URL through the official installer. The downloaded native archive is byte-identical to the tested package. A final Performance build exited 0 in **15.065 seconds**, reusing the same accepted executable and leaving the sole `horizon` instance selected. The existing Rusty preference file stayed byte-identical. Both Java runtime downloads on Modrinth match their GitHub release files byte-for-byte; Modrinth still shows **Under review**. The existing running user game was left untouched and needs a restart to load the patch.

## 1.2.2 current Nightly compatibility patch

Horizon Nightly **38609b7 / native API 409 / Minecraft 26.3** replaces the old held-key set with a device-aware container. Rusty's two obsolete collection calls prevented compilation. The camera integration now removes its movement mappings individually and restores physically held movement keys on exit. No upstream production source is changed.

Fresh checks on the new identity: **141 Rust library + 18 native-tool tests**, **14 Python tests**, formatting, strict Clippy, structural packaging, Linux exact-SDK compilation and official package verification passed. The actual launcher **Rebuild on latest Nightly → trust-build** buttons were pressed in **Performance** mode. Windows compilation succeeded in **11m 33s**; total build/installation took **20m 28s**. Horizon showed **Up to date**, selected `native-38609b7-ba8dab22119b`, and GUI **Play → title → Options → Mods → Rusty → clean Quit** passed.

The installed executable (SHA-256 `a3dd935963a3acecfc518c274fb2d58572d3657d7a7a3e94cc4a53f00cd5c363`) passed **9 fresh feature checks** and **10 interface checks**, including X-ray/off/100% restoration, received block updates, Finder, lighting, held zoom, moving/turning Freecam with stationary body XZ/rotation, waypoint scopes and dimension/reopen behavior, keybind clear/recovery, menu-size endpoints, three palettes, all 18 HUD controls, profile preservation and actual process-restart persistence. Original X-ray, Freecam and native browser frames were inspected. All 115 native payloads in the release package match the source installation used by the launcher build.

[1.2.2 acceptance receipt](evidence/native-update-1.2.2.json) keeps exact inputs and named checks. The package contains 116 files, including its root manifest; archive SHA-256 is `13a4d22ad1985e08eada3a36777a89a5c58f71e961b32a597adecfd3595f05b9`. Public-tag installation is a separate subsequent gate. Fresh 1.2.2 tests did **not** join a remote server, rerun the earlier exhaustive module/performance/endurance suites, or validate arbitrary graphics configurations. No personal world or public-server character was used. The Fabric/NeoForge beta was not changed.

## 1.2.0 full native acceptance (historical)

Fresh checks identify the actual Rusty executable and source inputs. No historical game result is relabeled as a new test. [Build evidence](evidence/native-build.json), [game acceptance](evidence/game-acceptance.json), [named checks](evidence/game-checks.json), [launcher UI acceptance](evidence/launcher-ui.json) and [media provenance](evidence/media.json) keep their separate scopes.

| Gate | Result |
|---|---|
| Rust library/release-tool tests | 141 + 18 passed |
| Python tooling | 13 tooling tests passed; actual game-fixture behavior is covered by the fresh game checks below |
| Formatting and strict lint | Passed locally and in Linux CI |
| Native SDK compilation/package | Pinned SDK check and official package verification passed; Linux/Windows native archives byte-identical |
| Official Windows native build | Performance build completed successfully, waited to exit 0 |
| Core game behavior | 124 checks across 13 suites passed, including an owned official Java 26.3 server and a separate native Survival guest |
| UI/overlay behavior | 10 UI checks and 3 HUD-column checks passed, including clear/rebind, size endpoints, 18 independent HUD controls and actual process restart |
| Visual review | Original interface, help/binding, texture-context, profile and HUD frames reviewed at their captured scales |
| Short performance observation | 3 disabled + 9 enabled samples; stationary aggregation-cache reuse passed |
| Bounded stability | Ten-minute run completed; no panic/fatal marker observed; owned process stopped |
| Launcher build and actual GUI Play | Performance build through the official CLI; launcher 0.2.42 showed green Up to date, then GUI Play → title → Rusty settings → clean Quit. No Build button was offered for the already-built mod set. |
| Published release installation | Both repository and exact v1.2.0 release URLs installed successfully through launcher 0.2.42. All 115 payloads and archive bytes match the published/tested package. Supported cached Performance build exited 0; GUI Up to date → Play → title → clean Quit passed. |

[Release installation receipt](evidence/release-installation.json) records both accepted inputs, exact immutable tag/commit/asset, unchanged preferences and the executable shared by all game checks. [Published package](evidence/published-package.json), [release/workflow identities](evidence/published-release-api.json) and [official version check](evidence/official-version-check.json) are separate receipts.

## Exact native identity

- Supported public game: Minecraft 26.3, Horizon `6475f62f3c834b7489e1816bf47f47ea5eb2b879`, native API 359.
- Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Accepted executable SHA-256: `fb177cd9216a567bb4982bfacedc47d0b07e6af6cdc43cfd2591050209ca29ea`.
- Staged input receipt SHA-256: `1c5d75b9a97e0f3f9e3ebf83de45e6d4a4aa41804ddf4b51391c83365d4f7646`.
- Native archive SHA-256: `7fb9ade0e05595fad6ddc019c0fdb0807a802f5667091df9eeb9a58ea908db0d`.
- Independent Linux [CI package receipt](evidence/ci-package.json) identifies its exact source/run/job/artifact; later documentation and test-observation changes leave the 115 native payloads unchanged.

## Corrected measurement and limits

One initial native multiplayer run timed out after a restored block. Its test had frozen the first zero-queue target count as its baseline. An unchanged-binary diagnostic passed all five lifecycle checks and captured a temporary 493→495 target transition during ordinary cache eviction/rebuilding. The first failing run did not record its exact baseline; it is not reconstructed or asserted. The stronger fixture waits across more than two scan intervals, distinct rendered frames and ready renderer queues, keeping exact original-minus-one/original counts, nearest targets, received block checks and host/guest source hashes. The failed raw receipt and diagnostic remain distinct from the corrected acceptance. [Settling observations](evidence/multiplayer-settling.json) record the corrected measurements.

A separate runner argument error was rejected before a performance game launched; the invocation was corrected without changing the fixture or executable. Acceptance records keep this separate from game behavior.

Owned server tests are separate from a player's public server. No personal world, authenticated account, public-server movement/building, inventory or combat was used in these fresh fixtures. Arbitrary server plugins, network latency, other hardware/backends and resource packs remain unverified. Servers can withhold or replace block data and retain authority over movement, reach, inventory and health. No anti-X-ray bypass, reference-client parity/superiority, leak-free or multi-hour claim is made.

UI 50% demonstrates contained compact layout; 200% keeps the outer menu/fixed controls inside the frame while detail content scrolls. Independent HUD placements are tested and visually reviewed in their recorded fixtures; not every possible user arrangement is validated. Screenshot tours are presentation media rather than continuous gameplay.

[Installation](INSTALLATION.md) · [Development](DEVELOPMENT.md) · [Performance](PERFORMANCE.md)

## 1.2.1 branding patch

The copper badge is now the R in the integrated Rusty wordmark; native and Java menu headers use the badge alone. [Patch evidence](evidence/branding-1.2.1.json) records the separate scope.

Fresh native checks: 141 Rust + 18 packaging tests, 13 Python tooling tests and strict lint passed. The launcher’s actual **Build** and trust-build buttons were pressed in Performance mode; it finished in **14 minutes 39 seconds**, selected `native-6475f62-fe330eeed81e` and showed **Up to date**. The installed executable passed **10 UI checks**, including clear/rebind, menu size endpoints, three palettes, all 18 HUD controls, profile preservation and restart persistence. The standalone badge was visually inspected in original frames.

Horizon allocates its native image atlas slot only when `native-mods.json` beside the executable lists native mods. A preliminary bare-target fixture omitted that metadata and showed no badge; the integration run therefore uses the launcher-installed executable and its metadata. No production-source workaround is needed.

Both Java loaders passed **45 fresh checks** with the R-only menu header and direct framebuffer captures. Full native functional, server and performance results above remain explicitly the earlier 1.2.0 acceptance, rather than being relabelled as new exhaustive tests.

The [published-release installation receipt](evidence/public-release-1.2.1.json) verifies both the repository input and fixed 1.2.1 release input through Horizon’s official GitHub installer. The installed archive is byte-identical to the tested native package; the accepted executable remains selected. The post-install native build exited 0 and reused the Performance build in 9.639 seconds. The final launcher view shows 1.2.1 from `Justeck/Rusty @ v1.2.1` and green **Up to date**. Play, the normal title, Options → Mods → Rusty settings, and clean Quit Game were observed on the identical accepted executable.
