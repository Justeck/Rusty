<!-- Submitted for manual review with explicit AI disclosures; not confirmed approved. No generated project-page artwork uploaded. -->

**AI disclosure:** Rusty was developed predominantly with an AI coding assistant. The in-game interface includes AI-generated copper branding, and this description was drafted with AI assistance. No generated project-page images are submitted. This is disclosed for Modrinth’s manual review, including its restrictions on primarily AI-produced projects.

# Keep your bearings.

Rusty brings textured world views, local navigation, movable information panels and optional player assistance together in Minecraft's own style. Choose the tools you need without a large dashboard covering the game.

**Fabric and NeoForge · Minecraft 26.3 · Java 25 · 1.3.0-beta.1**

## A familiar menu

Press **Right Shift**, or open **Options → Rusty**. Search the catalog, browse Enabled/Favorites/Recent and categories, and click a small **?** for help. Rebind any shortcut or clear it entirely. Resize the menu independently of the HUD; drag and individually scale any of 18 overlays.

## Terrain with context

Selected received blocks stay visible while the surrounding blocks keep frames of their original textures. The texture slider starts at 25%: slide toward thin textured edges for visibility or all the way to 100% for ordinary terrain. Typical ores are preselected; add any registered block ID. **X** toggles the view, with optional Hold mode. Finder adds bounded target boxes and pointers.

## Your own toolkit

- **Visibility and camera:** Fullbright, No Fog, Clear Weather, Zoom, No Hurt Camera and detached Freecam.
- **World awareness:** target information, chunk borders, sampled floor-light warnings, received entity/storage outlines and approximate projectile paths.
- **Navigation:** coordinates, compass, local waypoints, death markers, breadcrumbs, portal coordinates and a received-surface minimap.
- **Information:** equipment durability, biome/light, embedded held-container contents, clock, speed, FPS and click counters.
- **Optional assistance:** ordinary sprint/crouch/jump requests, tool/food/armor/refill/totem assistance, owned-hook bite reeling, explicit waypoint walking, one-action build planning and full-cooldown crosshair attacks.

**Status HUD starts enabled; other modules start off.** Inventory and movement changes use the game's ordinary action paths and remain subject to the server's decisions. Freecam leaves your body in normal simulation.

## Multiplayer: explicit permission for restricted tools

Ordinary information and presentation tools work on unmodified servers. X-ray/Finder, through-block entity/storage views, Freecam and automatic combat/movement/inventory tools require an explicit capability from the server. **Without it, they remain inactive.** Singleplayer grants capabilities locally. A profile cannot grant multiplayer permission.

A server owner can optionally install the matching Rusty server jar and edit `config/rusty-server-permissions.json`. Its default grants are empty. Capabilities (`view`, `combat`, `movement`, `assistance`) can be assigned per authenticated UUID or server-wide; updates and revocation are applied live. [Server setup and exact module groups](https://github.com/Justeck/Rusty/blob/main/java/README.md#multiplayer-permissions).

Rusty uses only information already received by the client. It does not defeat server anti-X-ray, recover withheld ores, bypass reach/cooldowns or alter server authority. Follow your server's rules.

## Install

Choose **one** loader version for Minecraft 26.3:

- **Fabric:** Loader 0.20.1+ and Fabric API 0.165.2+26.3 are required.
- **NeoForge:** 26.3.0.101.
- **Java:** 25.

Put the matching Rusty jar into that Java installation's `mods` directory. Do not install both loader builds together. [Full guide](https://github.com/Justeck/Rusty/blob/main/java/README.md).

**Using Horizon?** Its native Rust package is a separate edition hosted on [GitHub](https://github.com/Justeck/Rusty/releases/tag/v1.2.1). These Java jars are not Horizon-native mods; Modrinth currently has no Horizon loader category.

## Evidence and limits

The project keeps builds, shared behavioral checks, actual in-game fixtures and unverified cases separate. [Read the Java validation report](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md). This beta targets one exact game version. Compatibility with arbitrary rendering replacements, shaders and modpacks is unverified.

The minimap is a received surface-color map. Trajectories do not predict entities, spread or future movement; Light Risk is not a complete spawn predictor. Auto Fish reels after a received bite and needs a manual cast. Identical rejected inventory sources require an explicit off/on rearm. No claims of complete parity or better performance than unrelated clients are made.

[Source](https://github.com/Justeck/Rusty) · [Report an issue](https://github.com/Justeck/Rusty/issues) · [License and attribution](https://github.com/Justeck/Rusty/blob/main/THIRD-PARTY.md)

Apache-2.0 with applicable project notices. Original copper branding; no Minecraft assets or third-party mod binaries bundled.
