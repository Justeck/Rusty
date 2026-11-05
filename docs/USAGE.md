# Usage and settings

This guide describes Rusty **1.2.4**. The [module guide](MODULES.md) covers all 42 registered tools, ordinary-action requirements and data limits. See [validation](VALIDATION.md) for exact artifacts and actual test evidence, and [installation](INSTALLATION.md) for the pinned native host.

## The live ClickGUI

Press **Right Shift** in the world. Rusty uses Horizon's native mod-owned screen over the continuing world and owns menu input while open. The rebound menu shortcut or **Escape** closes it; Escape first cancels key capture, help or a category popup when active. The input contract requires a fresh gameplay press after closing, rather than replaying a menu click as attack/use. Actual focus and input behavior is part of native acceptance.

- Click search or press **Ctrl+F**. Search names, aliases, descriptions and setting labels; all query words must match the module or one matching setting. Queries are bounded to **128 characters**.
- Browse populated categories, **All**, **Enabled**, **Favorites** or **Recent**. Enabled follows live activation, including temporary tools. Empty categories are hidden.
- Select a row for details. **ON/OFF** toggles it; **+ / \*** changes favorite state. Zoom is a held-key action marked **KEY**.
- Scroll the list or detail pane under the pointer. Tab/Shift+Tab and Enter provide focus navigation.
- Drag sliders, use minus/plus steps, or right-click a slider to reset it. Context help explains limits.
- Cycle **Classic**, **Graphite** and **Copper**. Set **Motion: Low** to remove the **120 ms** opening fade; **Motion: Full** restores it.
- Select a binding, then press a key or mouse button. **Escape** leaves it unchanged. The bottom **Menu** control rebinds Right Shift itself.
- Use **Clear** beside a binding to leave it unbound. Right-click its binding control to restore the default. The native **Options → Mods → Rusty** screen remains available, and `/rusty menu` recovers an unbound menu.
- Adjust **Menu size** independently from Minecraft's GUI scale: 50–300%, with step buttons and a 100% reset. **100% is now the former 70% physical size**, and is the new default. Existing saved physical sizes stay unchanged and display their equivalent new percentage. Small windows fit the requested size to the available area.
- Screen HUD modules expose **HUD X (%)**, **HUD Y (%)** and **HUD size** controls, plus a per-panel reset. Position is a percentage of the space remaining after the panel fits. HUD size is independent from menu size; world-space indicators remain anchored to their targets.

Favorites, recent selections, menu size and HUD placement persist locally and survive feature-profile imports; recent items are capped at **16**. **Horizon Options → Mods → Keys** remains the binding fallback. Check for conflicts there when a shortcut behaves unexpectedly.

Search supports **Ctrl+A**, **Ctrl+C**, **Ctrl+X** and **Ctrl+V** for selection and clipboard editing. For a denser desktop layout, choose **GUI Scale 2** in the game's video settings; Rusty follows the game's GUI scale and supports independent scrolling in both panes.

**Advanced / Profiles** opens the full native editor with six tabs: **Overview**, **Xray**, **Targets**, **Visuals**, **Camera** and **Profiles**. It remains available through **Options → Mods**. **Apply** or **Close** in that editor saves validated settings.

## X-ray and terrain

Press **X** to toggle X-ray. In **Xray**, switch activation to **Hold** for press-and-hold use. Rebind controls through **Horizon Options → Mods → Keys**.

With **Textured Context** enabled, ordinary exposed faces keep frames made from their original block textures. **Texture Visibility** (search for **block textures**, or use **Block textures** in the detailed native editor) describes visible face area, rather than linear border width:

| Value | Appearance |
|---|---|
| 0% | Thin original-texture edges remain visible. |
| 25% | Default texture frames with an open center. |
| 100% | Normal native terrain, including ordinary lighting and terrain range. |

Below 100%, **Terrain radius** controls nearby frames from 1 to 48 blocks; the default is 18. **Terrain opacity** defaults to 255, fully opaque. Disable Terrain context to remove ordinary terrain frames. Target outlines and tracers have separate settings. At 100%, independently enabled Finder or visual overlays can still be visible.

Drag a numeric slider, click minus/plus, or focus it and use arrow keys for precise steps. Keyboard navigation also supports Tab and Enter.

## Target rules

**Targets** starts with the usual ores. Add any other client-known block by searching the registry, selecting the held block, or selecting the aimed-at block. Presets can add ores, storage blocks, or lava.

Each block ID has one rule. Set its label, group, RGB color, outline box, tracer, horizontal radius from **1 to 128 blocks**, inclusive Y bounds, and block-state property filters. The block must be enabled and match all applicable filters. Wider selections increase work and can reach result limits. A profile supports up to 256 distinct block rules.

Per-rule radius limits scanned results and their boxes/tracers. Native X-ray terrain uses the block/state/height rules across received chunks within the game's render distance.

## Block Finder

X-Ray changes terrain only. Enable Finder separately for target boxes, tracer lines and target readouts. The line/box controls live under Finder; X-ray activation never changes Finder or overwrites saved line preferences. An independently enabled Finder remains active when X-ray is switched off.

Finder is **off by default**. Press **V** or enable it in the menu for bounded scans of received world data. Adjust scan radius, refresh interval, counts, and maximum targets to fit the selection. A manual scan is also available.

Scan radius is **1–128 blocks**, automatic interval **1–60 nominal ticks** at 50 ms per step, and retained target limit **1–4096**. Drawing budgets can be tighter than the retained-result limit.

Counts describe available results within these limits. They are not a complete census of a world or unexplored terrain. Dense selections may show partial results until later refreshes, or remain limited by the configured maximum. The partial indicator also covers terrain-context geometry budgets, so it can appear before the maximum target count is reached.

## Camera and visuals

Press **U** to toggle the detached camera. Set speed in **Camera**. Noclip is enabled by default; turn it off for camera collision. Detaching the view does not move the body to the camera or suspend normal body simulation, including gravity. Camera movement does not request new chunks around the camera.

Press **B** for Fullbright. Hold **C** to zoom, at **4×** by default. **Visuals** and **Camera** also expose optional fog/weather adjustments, entity boxes, coordinates, auto-sprint, and a stable hurt camera. These options have independent controls.

## Profiles

**Advanced / Profiles** can export/import JSON through the clipboard and save/load presets in Rusty's data folder. Names use **1–64 letters, numbers, dashes or underscores**.

Feature-profile imports preserve this player's theme, reduced-motion choice, favorites and recent selections. Horizon keybindings are separate and are not replaced; waypoints remain in their own local data file. Imports are capped at **4 MiB before deserialization**, file reads are bounded, and supported ranges, nested settings and duplicate block IDs are validated before application.

Export a copy before replacing a configuration you want to keep. Named presets make it possible to maintain different selections without editing source.

## Compass, waypoints and breadcrumbs

Navigation and utility modules are off by default. **Compass** shows cardinal heading and yaw. **Waypoints** display the selected local marker's name, distance and bearing. Markers are stored by local-world identity or remote server address, then by dimension; the same coordinates in another scope do not select that marker.

Use **Save Current Position** in Waypoints to save the current body position, or these local commands:

| Command | Result |
|---|---|
| `/rusty waypoint add NAME` | Save the current position, sanitize the name and select the marker. |
| `/rusty waypoint list` | List IDs and markers in this scope. |
| `/rusty waypoint select ID` | Select a marker in this scope. |
| `/rusty waypoint select none` | Clear the selection. |
| `/rusty waypoint delete ID` | Delete the marker and clear selection if necessary. |
| `/rusty waypoint copy ID` | Copy coordinates to the local clipboard. |

Names are sanitized and capped at **48 characters**. Storage permits **128 markers per scope**, **64 scopes** and a **4 MiB** navigation file. Positions must be finite and within supported bounds. Save failures are reported; corrupt existing files are preserved rather than replaced silently. Coordinates remain local unless explicitly copied/shared. These commands do not teleport the player.

**Breadcrumbs** retain at most **512 observed body positions** in the current session. **Point Spacing** is **0.5–16 blocks**, default **2**. **Break Distance** is **16–128**, default **64**, and must exceed point spacing. Teleports/corrections and scope changes break the trail; missing loaded terrain stops continuous recording. Disabling breadcrumbs clears the ephemeral trail, which is not persisted with waypoints.

## Equipment, environment and container readouts

**Equipment HUD** shows four armor slots, selected main hand and offhand. Durability uses known world-registry values and supplied stack overrides/removals. Unknown and unbreakable values receive explicit labels. The warning defaults to **20%**, configurable **1–50%**, and compares the actual ratio rather than the rounded displayed percentage. It performs no equipment swap, repair or item use.

**Environment HUD** shows the body's biome and block/sky light at its feet, each channel **0–15**. Absent chunks, out-of-height positions, unready light and unknown biomes stay unavailable. Unavailable light is not displayed as measured darkness. These values do not guarantee whether a mob can spawn.

**Container Preview** lists supplied top-level contents of the selected main-hand item. It examines at most **64 supplied entries**, displays **5 rows by default**, configurable **1–9**, and reports omitted/invalid entries. Nested containers are listed as items without expansion. It does not open world chests, query another inventory or request server data.

The SDK represents absent container components and empty decoded lists identically, so the readout says **“empty or not provided”** rather than claiming known emptiness. A text list appears only for supplied entries. Labels and rows are bounded, and omitted entries receive a truncation message. Screen panels fit by scaling when necessary. Utilities hide with F1 and while another GUI owns gameplay input.

## Client-data boundary

Views and searches depend on data already delivered to the client. Rusty cannot bypass server anti-X-ray, recover blocks the server withheld, or replace missing world data. Server rules and enforcement remain applicable.

The source registry contains **42 modules**. The [research matrix](research/README.md) contains **884 reference occurrences** with source/documentation/historical boundaries; they are comparison records, not implemented module counts or proof of reference parity. See the [backlog](research/BACKLOG.md) for remaining capability and test gates.

## Default HUD stacking

Default overlays form left, center and right columns. Each column uses the visible panels' actual sizes, so expanded Target Info, Finder counts and container contents receive space. The right column reserves room above the default minimap. Columns fit within the screen when needed. Changing a panel’s position or size gives it an independent saved placement; Reset returns that panel to automatic default placement. Custom placements can intentionally overlap, so leave space for expanded content.

Automatic spacing covers Rusty's default panels and default minimap. Chat, host notifications and other mods can still cover an overlay; use the individual HUD controls to arrange a layout around the information you normally keep visible.
