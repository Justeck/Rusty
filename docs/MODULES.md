# Module guide

Rusty exposes implemented native modules rather than placeholder entries for the reference clients' much larger catalogs. Search names, aliases, descriptions and individual setting labels with **Right Shift**, then **Ctrl+F**. Every module has a configurable shortcut; most start unbound. New action and awareness modules start disabled. Test receipts and the supported binary belong in [Validation](VALIDATION.md), not in this feature inventory.

## Auto Mine

Explicitly mine a chosen resource through a conservative weighted route, with normal movement and mining, suitable hotbar tools, a block goal and live safety stops. Includes eleven ore presets, aimed Custom selection, optional access tunnels and a green route preview. Off and unbound by default; stops never resume themselves. Predicted openings wait for normal server acknowledgment. [Controls, defaults and limitations](AUTO_MINE.md).

## Seeing received world data

| Module | Operation and limits |
|---|---|
| X-Ray | Select known block IDs with optional states, heights, radius, color, label and group. X toggles by default; optional Hold mode ends on release. It never requests hidden server blocks. |
| Finder | Bounded scans of received sections, independent of X-ray. Finder owns target boxes, tracer lines and target readouts; X-Ray alone does not render them. Results and refresh have explicit budgets; dense selections are marked partial. |
| Entity ESP | Boxes for received Players, Mobs and Items, using its own target choices. |
| Entity Tracers | Independently enable lines to received Players, Mobs and Dropped Items. Any combination or none; individual defaults and saved profile choices. Bounded range and draw work, without discovering unloaded entities. |
| Storage ESP | Received live block-entity positions, including ordinary updates and removal. It cannot reveal unopened inventory contents. |
| Light Risk | Marks known floor/headroom samples using ready received light. Red means observed darkness; amber distinguishes skylight. It is a conservative visual aid, not a complete mob-spawn predictor. |
| Chunk Borders | Geometry aligned to the player's current chunk, including vertical section guides. |
| Trajectories | Read-only shadow simulation using Horizon's native arrow/thrown-projectile physics, current received collisions and fluids. Main-hand bow needs ammunition and charge. Pearl, snowball and egg variants are supported. Random spread and future entity movement are not predicted; unknown terrain and target-budget limits are reported. |
| Target Info | Compact received properties for the normally reachable crosshair block. |
| Minimap | Horizon's cached native surface/cave renderer, with size, scale, rotation and round/square controls. It covers received terrain only. |

**Block textures** uses the original installed block texture around an open face center. Its minimum retains a thin textured edge, the default covers approximately 25% of the face, and its maximum restores ordinary native terrain and lighting. Terrain context also has independent radius and opacity controls. No texture-pack assets are bundled.

## Visibility and camera

| Module | Operation and limits |
|---|---|
| Fullbright | Client lighting override; X-ray's automatic brightness is separately reconciled. |
| No Fog | Local environmental fog presentation. |
| Clear Weather | Local weather presentation, preserving the authoritative weather state. |
| Zoom | Hold its shortcut, C by default; adjustable factor. The menu labels this **KEY**, not a persisted toggle. |
| No Hurt Camera | Suppress the local hurt tilt without changing damage. |
| Freecam | Detached view with speed and optional collision. The server-side body still exists and simulates normally. It does not load chunks around the detached camera. |

## Ordinary player actions

| Module | Operation and limits |
|---|---|
| Auto Sprint | Requests ordinary forward sprint input before native movement physics. |
| Safe Walk | Requests ordinary crouching near an unsupported edge while grounded; respects jumping and received collision shapes. It does not grant invulnerability or prevent every fall. |
| Auto Jump | Requests an ordinary jump over a received obstacle when headroom is clear. |
| Auto Tool | Chooses an appropriate hotbar tool while mining, using the host's tool/drop rules and a remaining-durability threshold. Manual selection takes ownership. No inventory moves. |
| Trigger Bot | Ordinary crosshair attack, normal reach, visible living target and full cooldown. Mobs only by default; players require an explicit option. No silent rotation, team/friend filtering or reach extension. |
| Auto Eat | Safe ordinary food from the hotbar below a chosen hunger threshold. Component identity, user input, menus and timeouts cancel ownership. The status HUD shows a paused state after interruption. It never opens a targeted container. |
| Auto Fish | Cast once yourself. Observe this client's hook and a received bite, wait a configurable delay plus the normal use cooldown, then issue one ordinary reel request. Optional recast waits for hook removal. User actions and menus cancel the sequence. |
| Armor Assist | Fill empty armor parts with eligible known inventory gear using ordinary quick moves. Preserve existing equipment, exclude binding curses and overridden equippable/attribute components. Received equipment updates rearm successful parts; a predicted click is not an acknowledgment. |
| Auto Refill | Remember nonempty hotbar stacks, then replenish an observed depleted slot from an exact component-matching main-inventory stack. Do not fill initially empty slots. A request consumes its intent and cannot blindly retry a rejected correction. Menus and user actions pause ownership. |
| Auto Totem | Ordinary hand swap from a normal hotbar totem into an empty offhand, waiting for received hand state before restoring the previous selection. Existing offhand items are preserved. It does not search a chest or guarantee survival. |

Auto Refill uses received slot changes and exact component identities. When a server accepts a predicted refill without echoing the positive slot, a fresh ordinary **Use** or **Attack** press can authorize one further refill request. Holding a button does not renew that permission. The permission expires after ten seconds and is cancelled by menu/focus or hotbar changes; an explicit off/on starts a new baseline. Renewed input is user intent, not a server acknowledgment: a late empty correction after that new input cannot always be distinguished from depletion. Without renewed input, the helper does not repeatedly retry an unconfirmed request. Loot picked up into the target before replenishment is preserved; the helper does not overwrite that new item to force a refill. These inventory helpers operate in idle Survival after ordinary controls are released, preserve a held cursor and other open server menus, and do not replace worn armor.

The server can reject or correct ordinary actions. These modules use the normal client action paths and require no server-side Rusty installation. They do not circumvent server rules or turn server-authoritative movement, reach, health or inventory into client-owned state.

## Build Planner

Anchor a cuboid preview to a received block face. Width, depth and height each range from 1 to 16; choose solid or hollow sides with independent floor and roof. The plan has at most 4096 cells and draws at most 128 nearest cell outlines. Matching material, missing, occupied and unknown cells are distinguished. Passive preview reads received facts and changes no block or inventory.

**Place Next** explicitly requests one missing reachable placement using the held block. It visibly aims at a normal reachable support, checks the host's dry placement result, sends one ordinary main-hand action and waits for settled server state. Manual input, menus, focus loss, item/context changes and a timeout end ownership. There is no continuous building loop, offhand fallback, inventory selection, multi-block-item support or automatic retry. The server remains authoritative; use a separate test world for learning the action.

## Navigation, information and local tools

| Module | Operation and limits |
|---|---|
| Coordinates | Body coordinates and received dimension. |
| Compass | Local heading display. |
| Waypoints | Validated local markers scoped to verified world/server identity and dimension. Save the current position in the menu; manage through `/rusty waypoint`. |
| Breadcrumbs | Session trail of observed body movement, capped at 512 samples; teleports and world changes break the trail. |
| Death Markers | One local marker after an observed alive-to-dead transition in the current scope; preserves the selected waypoint. Enabling while already dead does not invent a death position. |
| Waypoint Walk | Explicit, temporary route to the selected marker through loaded terrain, using the host's bounded A*. It refuses another active task, has no straight-line fallback and cancels on user input, menu, focus loss, death, Freecam or scope change. Closing a menu never restarts it. |
| Portal Coordinates | Conventional Overworld/Nether 8:1 coordinate conversion, not a portal location or link prediction. |
| Speed HUD | Observed body displacement over time; resets across discontinuities. |
| World Clock | Received world time, without changing the server clock. |
| Frame Rate | The native frame-rate observation. A cap can hide maximum throughput. |
| Click Counter | Bounded recent local click observations; not server attack acceptance. |
| Equipment HUD | Read-only held/armor durability from known registry/components; unknown values stay unavailable. |
| Environment HUD | Received biome and ready block/sky light. |
| Container Preview | Bounded top-level contents supplied in a held item's components. No chest queries or nested expansion. |
| Status HUD | Compact X-ray/Finder results and enabled assistance indicators. Independently configurable from Coordinates. |

## Menu, profiles and shortcuts

Classic, Graphite and Copper themes use Minecraft's native text and controls. The two panes scroll independently; categories, favorites and recent selections reduce browsing. Sliders support drag, step controls and reset. Small **?** controls open contextual descriptions. **Reduced Motion** removes the 120 ms opening fade.

**Advanced / Profiles** retains detailed target editing and local JSON presets. Feature-profile import preserves local theme, reduced motion, favorites and recent selections; Horizon stores keybindings separately. Imports and file reads are bounded. World/server marker data stays local. **Options → Mods** remains available as the native entry point.

See [Usage](USAGE.md), [Installation](INSTALLATION.md), [Performance](PERFORMANCE.md), and the [reference implementation analysis](research/implementation-analysis.md). The research inventory includes commands, HUD elements, aliases and historical records; it is not an implemented-module count or a claim of parity with those clients.

## Enabled Mods

**Interface → Enabled Mods** is an opt-in screen overlay that lists active Rusty tools alphabetically, one name per row. It excludes its own name. Its default anchor is top-left; independent HUD placement and size controls let you move it. Runtime-only tools appear while active. This is the active Rusty feature list, not the loader’s dependency/mod inventory.
