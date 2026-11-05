<p align="center"><img src="docs/assets/rusty-wordmark.png" width="760" alt="Rusty — original copper wordmark"></p>
<p align="center"><strong>Keep your bearings.</strong></p>
<p align="center">43 configurable tools · Horizon · Fabric · NeoForge · Minecraft 26.3</p>

<p align="center"><a href="https://github.com/Justeck/Rusty/actions/workflows/build.yml"><img src="https://github.com/Justeck/Rusty/actions/workflows/build.yml/badge.svg" alt="Native checks"></a> <a href="https://github.com/Justeck/Rusty/actions/workflows/java.yml"><img src="https://github.com/Justeck/Rusty/actions/workflows/java.yml/badge.svg" alt="Java edition checks"></a> <img src="https://img.shields.io/badge/Horizon_runtime-Rust-ce8b62" alt="Horizon runtime: Rust"> <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-777777" alt="Apache-2.0"></a></p>

**Rusty is a native Rust toolkit for Horizon, with Fabric and NeoForge counterparts:** a familiar Minecraft interface, textured world views, independent HUD layouts, local navigation and ordinary player assistance. Press **Right Shift** to open the menu. Search by module, setting or description, then enable the tools you want.

**Earlier native 1.2.0 acceptance:** 124 core checks, 10 interface checks, 3 HUD-column checks, owned Java and native multiplayer fixtures, short performance measurements and a ten-minute stability run. The current 1.2.5 defaults and active-list update has separate fresh build, game and launcher checks. [Validation](docs/VALIDATION.md) records exact identities, scope and limits.

<p align="center"><img src="docs/screenshots/rusty-1.2.1-classic.png" width="960" alt="Rusty native Minecraft-style module browser"></p>

<p align="center"><a href="https://github.com/Justeck/Rusty/releases/latest">Download</a> · <a href="docs/INSTALLATION.md">Installation</a> · <a href="docs/MODULES.md">Modules</a> · <a href="docs/screenshots/README.md">Screenshots</a> · <a href="docs/VALIDATION.md">Validation</a></p>

## Editions

| Edition | Game/runtime | Installation |
|---|---|---|
| **Horizon native 1.2.5** | Horizon 38609b7 · API 409 · Minecraft 26.3 · Rust | Native GitHub input below |
| **Fabric 1.3.0-beta.4** | Minecraft 26.3 · Java 25 · Fabric Loader 0.20.1+ and Fabric API | [Java guide](java/README.md) |
| **NeoForge 1.3.0-beta.4** | Minecraft 26.3 · Java 25 · NeoForge 26.3.0.101 | [Java guide](java/README.md) |

The Java counterparts share Minecraft-style controls and a 43-module catalog, with edition-specific behavior and validation. Their restricted multiplayer tools require authoritative server opt-in; ordinary information tools work on unmodified servers. [Permissions and differences](java/README.md#multiplayer-permissions) · [Java validation](docs/JAVA_VALIDATION.md). The stable native release remains the GitHub `latest` so Horizon's installer keeps resolving its native package.

The Fabric and NeoForge beta has been [submitted to Modrinth](https://modrinth.com/mod/rusty-toolkit) with explicit AI code, asset and text disclosures. It is **under review**, with approval unconfirmed. GitHub remains the available download source. [Submission record](docs/evidence/modrinth-submission.json).

## Install

In Horizon Launcher, select **Horizon → Mods → Add a Native Mod from GitHub** and enter:

```text
https://github.com/Justeck/Rusty
```

Review the selected immutable source revision and enable Rusty. If Horizon offers **Build**, choose **Performance** and wait for **Up to date** before **Play**. An identical native mod set can reuse its verified build. The supported base is Horizon **38609b7**, native API **409**, Minecraft Java **26.3**. Every enabled native mod must support the selected host.

The fixed release input is [`https://github.com/Justeck/Rusty/releases/tag/v1.2.5`](https://github.com/Justeck/Rusty/releases/tag/v1.2.5). The [1.2.5 acceptance receipt](docs/evidence/defaults-list-1.2.5.json) identifies the native source payloads and tested executable. [Validation](docs/VALIDATION.md) records the actual Build/Play checks and distinguishes release installation from source-build acceptance.

**X-Ray changes terrain only.** Target boxes, tracer lines and Finder target readouts require **Finder** to be enabled separately. X-ray toggles preserve Finder and line preferences.

**Menu size now defaults to 100% at the former 70% physical size.** Scale from 50% to 300%, or reset to 100%. Existing saved physical sizes are preserved; HUD scaling remains independent.

**Enabled Mods** adds a vertical, alphabetically ordered list of active Rusty tools in the top-left. It is opt-in and has its own position and size controls.

**Default** buttons restore individual preferences or shortcuts. **Restore defaults** in the menu header restores all Rusty settings after confirmation, including shortcuts, appearance and HUD layout. Active tools stop; saved waypoint and named profile files are kept.

**Right Shift** opens Rusty. **Options → Mods → Rusty** remains the recovery path. See [Installation](docs/INSTALLATION.md), [Usage](docs/USAGE.md) and [Modules](docs/MODULES.md).

## Familiar controls, less friction

- Minecraft-style buttons and clear name/category spacing across Classic, Graphite and Copper.
- Live **Enabled**, Favorites and Recent collections; instant search and contextual **?** help.
- Rebind or **Clear** a shortcut, including the menu shortcut. Native settings remain available for recovery.
- Individual **Default** controls and a confirmed **Restore defaults** for the whole mod.
- Independent menu size, with sliders, step controls and reset.
- Position, size and reset controls for **19 HUD overlays**, saved locally. Default columns fit expanded contents and leave room for the minimap.
- Original block-texture frames in X-ray: **25% by default**, thin textured edges at minimum, ordinary terrain at maximum. **X** toggles by default; optional Hold mode deactivates on release.

<p align="center"><img src="docs/media/rusty-1.2-ui-tour.gif" width="960" alt="Ten-second Rusty screenshot tour"></p>

A **ten-second still-image tour** of the real native interface and renderer. [MP4](docs/media/rusty-1.2-ui-tour.mp4) · [Original captures and provenance](docs/screenshots/README.md). This is presentation media, not continuous gameplay or a frame-rate recording.

## See Rusty in use

**53 original game captures**, with separate Horizon and Java galleries. The new Java tour contains 39 scenes: texture coverage from minimum to 100%, ore/custom-block filters, Finder, outlines, navigation, Freecam, lighting restoration, HUD placement and the full menu workflow. Each image has a caption and an original-file hash.

| Textured view — Java edition | Compact Minecraft-style controls — Java edition |
|---|---|
| [![25% retained original textures](docs/screenshots/java/tour/rusty-gallery-03-texture-default-25.png)](docs/screenshots/java/tour/README.md#terrain-and-block-selection) | [![Compact tool browser](docs/screenshots/java/tour/rusty-gallery-36-compact-menu.png)](docs/screenshots/java/tour/README.md#interface-and-settings) |

| Detached camera — Java edition | Independent HUD layout — Java edition |
|---|---|
| [![Camera above the owned test courtyard](docs/screenshots/java/tour/rusty-gallery-18-detached-camera.png)](docs/screenshots/java/tour/README.md#world-tools) | [![All 18 panels in the layout editor](docs/screenshots/java/tour/rusty-gallery-30-hud-layout-editor.png)](docs/screenshots/java/tour/README.md#interface-and-settings) |

[**Browse the full screenshot collection →**](docs/screenshots/README.md) · [**39-scene Java tour →**](docs/screenshots/java/tour/README.md) · [**Modrinth gallery →**](https://modrinth.com/mod/rusty-toolkit/gallery)

The Java tour uses an authored, disposable singleplayer fixture. Restricted Java tools need explicit server opt-in in multiplayer. A screenshot illustrates an interface or view; it is not a new functional-test claim. Modrinth remains under review, and its gallery excludes generated artwork.

## Choose your tools

| Area | Modules |
|---|---|
| World views | X-Ray, Finder, Entity ESP, Entity Tracers, Storage ESP, Chunk Borders, Light Risk, Trajectories, Target Info, Minimap |
| Visibility and camera | Fullbright, No Fog, Clear Weather, Zoom, No Hurt Camera, Freecam |
| Assistance and building | Auto Sprint, Safe Walk, Auto Jump, Auto Tool, Trigger Bot, Auto Eat, Auto Fish, Auto Totem, Armor Assist, Auto Refill, Build Planner |
| Navigation | Compass, Waypoints, Breadcrumbs, Death Markers, Waypoint Walk, Portal Coordinates |
| Information | Coordinates, Status HUD, Equipment HUD, Environment HUD, Container Preview, Speed HUD, World Clock, Frame Rate, Click Counter |

The catalog contains **42 actual modules**. **Status HUD** starts enabled; other modules start off. Zoom works while its shortcut is held. Gameplay automation and inventory actions are opt-in. [Modules](docs/MODULES.md) explains each tool's bounded behavior.

Rusty uses data the client receives. Servers may withhold or replace block information, and retain authority over movement, reach, inventory and health. Finite work budgets, unknown chunks and unsupported data remain explicit limits. Freecam leaves the player body in ordinary simulation.

## Build and verify

```sh
cargo test --locked --features native-tools
cargo run --locked --features native-tools --bin rusty-native -- package
```

Runtime and optional native packaging tools are Rust. Python is development-only game-test orchestration. [Development](docs/DEVELOPMENT.md) covers exact pins, SDK checks and real game fixtures.

[Architecture](docs/ARCHITECTURE.md) · [Validation](docs/VALIDATION.md) · [Performance](docs/PERFORMANCE.md) · [Research](docs/research/README.md) · [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License and attribution

Rusty is **Apache-2.0**, with applicable **MIT** component notices. The native X-ray foundation includes work by **1_st_Cat / RealCat1st**; the camera includes MIT code by **umbx-dot**; Horizon is by **Ludovic Marie and contributors**. Preserve [LICENSE](LICENSE), [NOTICE](NOTICE), [THIRD-PARTY.md](THIRD-PARTY.md), [LEGAL.md](LEGAL.md) and component notices.

The integrated copper [R badge]USTY wordmark are original project branding, distinct from the official Rust mark. The graphite/copper artwork set includes a transparent emblem, wordmark and repository cover; see the [brand guide](docs/assets/BRAND.md). [Artwork provenance](docs/assets/branding.json) records the generated originals. Game resources, accounts and player worlds are not distributed.
