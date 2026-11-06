# Rusty in the game

**Original game screenshots, grouped by edition.** Open a gallery below, then click any image for its full-resolution file. Game captures are separate from generated brand artwork and from automated test assertions.

| Edition | Collection |
|---|---|
| **Horizon native** | 10 original captures below: menus, textured terrain, HUD placement, lighting and profiles |
| **Fabric / NeoForge** | [39-scene Java tour](java/tour/README.md), plus [4 original loader acceptance captures](java/README.md) |

## Visual HUD editing — native 1.2.7 / Java beta.6

These are original, visually reviewed captures from owned test worlds. The native
editor shows the real overlay with four resize handles; the next frame shows the
confirmed placement in ordinary gameplay. Opening the editor does not enable a
feature. The Java editor follows the same flow with Minecraft widgets.

![Native drag and corner resize editor](rusty-1.2.7-hud-editor.png)

![Confirmed native HUD arrangement in normal view](rusty-1.2.7-hud-confirmed.png)

![Java visual HUD editor in the clear world view](java/rusty-visual-hud-editor.png)

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

## Defaults and active list — Java 1.3.0-beta.4

Original, unretouched Fabric 26.3 frames from the owned disposable game test. Each was opened and inspected before publication.

![Individual Default buttons and confirmed full-reset entry in Minecraft widgets](java/rusty-java-default-controls.png)

![Fullbright and No Fog in separate top-left active-list rows](java/rusty-java-enabled-mods.png)

The first capture shows the actual module menu, with per-control Default buttons, shortcut defaults/clear and Restore defaults. The second shows gameplay with Enabled Mods on, listing only Fullbright and No Fog; Status HUD is off. These images document the interface and list; broader behavior is bounded by the linked acceptance receipt.

## Defaults and active list — native 1.2.5

Original frames from the actual launcher-built executable in an owned offline world. Opened and inspected before publication; unretouched.

![Native per-setting Default controls](rusty-1.2.5-default-controls.png)

![Full-reset confirmation](rusty-1.2.5-restore-confirmation.png)

![Active tool names in separate top-left rows](rusty-1.2.5-enabled-mods.png)

Exact file hashes, build identity and game scope are in [the acceptance receipt](../evidence/defaults-list-1.2.5.json).

## Independent entity targets — native 1.2.6

Actual frames from the accepted Windows Performance build, captured in a disposable offline world and visually inspected before publication.

![Independent tracer target controls](rusty-1.2.6-tracer-settings.png)

The saved target choices remain visible while the tracer feature is disabled. Each target has its own Default control.

| Tracers include both entities; ESP selects mobs only | Tracers off; the mob ESP box stays on |
|---|---|
| ![Diamond and pig tracers with a pig-only ESP box](rusty-1.2.6-entity-tracers.png) | ![ESP box without any tracer lines](rusty-1.2.6-esp-without-tracers.png) |

[Exact build, original-file hashes and acceptance scope](../evidence/tracer-targets-1.2.6.json).
