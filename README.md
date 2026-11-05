<p align="center"><img src="docs/assets/rusty-wordmark.png" width="760" alt="Rusty — original copper wordmark"></p>
<p align="center"><strong>Keep your bearings.</strong></p>
<p align="center">42 configurable tools · Horizon · Minecraft Java 26.3 · Native Rust</p>

<p align="center"><a href="https://github.com/Justeck/Rusty/actions/workflows/build.yml"><img src="https://github.com/Justeck/Rusty/actions/workflows/build.yml/badge.svg" alt="Native checks"></a> <img src="https://img.shields.io/badge/runtime-Rust-ce8b62" alt="Rust runtime"> <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-777777" alt="Apache-2.0"></a></p>

**Rusty is a native Rust toolkit for Horizon:** a familiar Minecraft interface, textured world views, independent HUD layouts, local navigation and ordinary player assistance. Press **Right Shift** to open the menu. Search by module, setting or description, then enable the tools you want.

**Tested in the game:** 124 core checks, 10 interface checks, 3 HUD-column checks, owned Java and native multiplayer fixtures, short performance measurements and a ten-minute stability run. The official launcher Build/Play path is checked separately. [Validation](docs/VALIDATION.md) records exact identities, scope and limits.

<p align="center"><img src="docs/screenshots/rusty-1.2-classic.png" width="960" alt="Rusty native Minecraft-style module browser"></p>

<p align="center"><a href="https://github.com/Justeck/Rusty/releases/latest">Download</a> · <a href="docs/INSTALLATION.md">Installation</a> · <a href="docs/MODULES.md">Modules</a> · <a href="docs/screenshots/README.md">Screenshots</a> · <a href="docs/VALIDATION.md">Validation</a></p>

## Install

In Horizon Launcher, select **Horizon → Mods → Add a Native Mod from GitHub** and enter:

```text
https://github.com/Justeck/Rusty
```

Review the selected immutable source revision, enable Rusty, select **Build → Performance**, and wait for **Up to date** before **Play**. The supported base is Horizon **6475f62**, native API **359**, Minecraft Java **26.3**. Every enabled native mod must support the selected host.

**Right Shift** opens Rusty. **Options → Mods → Rusty** remains the recovery path. See [Installation](docs/INSTALLATION.md), [Usage](docs/USAGE.md) and [Modules](docs/MODULES.md).

## Familiar controls, less friction

- Minecraft-style buttons and clear name/category spacing across Classic, Graphite and Copper.
- Live **Enabled**, Favorites and Recent collections; instant search and contextual **?** help.
- Rebind or **Clear** a shortcut, including the menu shortcut. Native settings remain available for recovery.
- Independent menu size, with sliders, step controls and reset.
- Position, size and reset controls for **18 HUD overlays**, saved locally. Default columns fit expanded contents and leave room for the minimap.
- Original block-texture frames in X-ray: **25% by default**, thin textured edges at minimum, ordinary terrain at maximum. **X** toggles by default; optional Hold mode deactivates on release.

<p align="center"><img src="docs/media/rusty-1.2-ui-tour.gif" width="960" alt="Ten-second Rusty screenshot tour"></p>

A **ten-second still-image tour** of the real native interface and renderer. [MP4](docs/media/rusty-1.2-ui-tour.mp4) · [Original captures and provenance](docs/screenshots/README.md). This is presentation media, not continuous gameplay or a frame-rate recording.

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

The copper R and RUSTY wordmark are original project branding, distinct from the official Rust mark. The graphite/copper artwork set includes a transparent emblem, wordmark and repository cover; see the [brand guide](docs/assets/BRAND.md). [Artwork provenance](docs/assets/branding.json) records the generated originals. Game resources, accounts and player worlds are not distributed.
