# Rusty in the game

**53 original screenshots, grouped by edition.** Open a gallery below, then click any image for its full-resolution file. Game captures are separate from generated brand artwork and from automated test assertions.

| Edition | Collection |
|---|---|
| **Horizon native** | 10 original captures below: menus, textured terrain, HUD placement, lighting and profiles |
| **Fabric / NeoForge** | [39-scene Java tour](java/tour/README.md), plus [4 original loader acceptance captures](java/README.md) |

## Java edition: the expanded tour

[![Textured terrain with Finder](java/tour/rusty-gallery-03-texture-default-25.png)](java/tour/README.md)

The new tour covers **terrain and block selection**, **world tools**, **lighting comparisons**, and **interface/settings**. It shows real Fabric 1.3.0-beta.1 on Minecraft 26.3; Fabric and NeoForge share this UI. [Explore all 39 scenes →](java/tour/README.md) · [Original-file manifest](java/tour/manifest.json).

The courtyard and dark chamber are owned test fixtures. Features are enabled for illustration; only Status HUD starts enabled by default. Restricted Java tools require server permission in multiplayer. The tour is presentation evidence, not a replacement for [Java validation](../JAVA_VALIDATION.md).

## Horizon native: original captures

These are fresh, unedited native game captures. The accepted executable, original capture names, dimensions and exact file hashes are recorded in [media provenance](../evidence/media.json). Generated brand artwork lives separately in [assets](../assets/branding.json). No game photograph is generated, retouched or relabeled from a previous product.

| Ordinary mine | Textured X-ray |
|---|---|
| ![Ordinary mine](rusty-1.2-normal.png) | ![Original block-texture frames and visible ores](rusty-1.2-textured-xray.png) |

| Minecraft-style controls | Independent HUD layout |
|---|---|
| ![Native browser](rusty-1.2-classic.png) | ![Independently positioned overlays](rusty-1.2-hud-layout.png) |

| Live Enabled collection | Texture coverage settings |
|---|---|
| ![Enabled collection](rusty-1.2-enabled.png) | ![Texture coverage control](rusty-1.2-texture-settings.png) |

The ordinary, [standalone Fullbright](rusty-1.2-fullbright.png) and [restored lighting](rusty-1.2-restored.png) photographs use the same camera. The scripted pixel comparison checks brightening and restoration in that bounded scene. [Detailed profiles](rusty-1.2-profiles.png) use the same native Minecraft-style controls.

The authored mine and interface fixtures are separate disposable worlds. Menu size and HUD layout photographs document their recorded fixture settings. Terrain context and target coverage have finite budgets; a showcase image is not a completeness assertion.

## Screenshot tour

![Ten-second still-image tour](../media/rusty-1.2-ui-tour.gif)

[MP4 version](../media/rusty-1.2-ui-tour.mp4). Five actual screenshots, two seconds each, resized and encoded using FFmpeg. Duration and frame count are checked. This is a timed still-image tour, not continuous gameplay or a performance demonstration. The original PNGs remain byte-identical to their captured files.

`rusty-1.2.1-classic.png` is an unretouched capture of the actual launcher-installed 1.2.1 binary in a disposable offline UI fixture. It shows the standalone copper R menu badge; the preceding 1.2 gallery remains historical evidence.
