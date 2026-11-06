# Area Clear

Select two opposite block corners to clear an inclusive box using ordinary player movement and mining. The Horizon edition is native Rust; Fabric and NeoForge provide matching controls through their existing runtime adapters.

## Select and start

1. Stand on safe ground and aim at the first corner block. Open **Right Shift → Area Clear** and click **Set first corner**.
2. Close the menu, move to a safe viewpoint and aim at the opposite corner. Open Area Clear and click **Set second corner**.
3. Review **Selection details**: inclusive dimensions, cell count and corner coordinates. The green box preview shows the boundary. **Clear selection** discards both corners.
4. Return to safe ground near the box with suitable hotbar tools and free inventory space. Explicitly enable **Area Clear**. The menu closes before controls are acquired.
5. Let it work, or stop it with its assigned shortcut, manual input or by opening a menu. It never resumes automatically.

Both corners may be the same block. Reversed corners are normalized. Each axis is limited to **32 blocks**, for at most **32,768 cells**. Corners are temporary: disconnects and world/dimension changes discard them. Active runs are never saved in profiles. Area Clear and Auto Mine cannot run together. Its shortcut starts **unbound** and can be assigned or cleared in the menu.

## Safety preferences

| Setting | Default | Purpose |
|---|---|---|
| Enabled | Off | Requires an explicit start after selection |
| Minimum health | 12 of 20 | Stops below this health, or on observed damage |
| Minimum food | 10 of 20 | Stops below this food level |
| Tool reserve | 10% | Refuses tools at or below the reserve |
| Selection and route preview | On | Shows the box and bounded planned actions |

Each preference has its own Default control. The global Restore defaults stops automation and clears its temporary selection. Changing safety preferences during a run stops it so they can be reviewed before restarting.

## What it clears

Ordinary breakable blocks with suitable available hotbar tools, including ordinary partial shapes such as carpet. It reviews received terrain incrementally and works on the highest remaining eligible layer. Weighted routes account for ordinary walking and estimated breaking time; it does not claim a globally optimal excavation order.

Every mining action stays inside the box. It never digs an access tunnel outside it and does not mine directly above or beneath the player's column. Other layers remain protected as potential support until their turn. Thin shapes are never treated as full supporting cubes. Outside partial shapes are treated conservatively so neighboring support is not deliberately removed beneath an attachment or half of a tall block.

Containers and other block entities, unbreakable blocks, fluids, hazardous or unstable blocks, unknown cells and unsuitable tools are protected. A survey containing any fluid, hazard, unstable or unknown cell stops the entire operation before planning excavation; deal with those hazards manually first. Containers and ordinary protected leftovers can remain while other safe blocks are cleared. It stops on unsafe or unreachable leftovers and reports them. **Area cleared** is used only after a fresh survey finds every selected cell is received, settled air; a protected leftover is not success. Natural block updates, external players and future world changes remain outside its control.

## Limits and multiplayer

Use it in a disposable area first. Conservative checks reduce risk; they cannot guarantee survival, preserve every indirect world update or replace a backup. Build a safe access route yourself when needed. Large/tall selections may require repositioning and a new explicit start. Each run is limited to 30 minutes. The bounded route search may decline a route that a player could complete manually. No flight, teleportation, placement or instant area removal is used.

Native Horizon uses received terrain and normal controls. Java multiplayer retains Rusty's current-server **view + movement + assistance** opt-in policy. Server permissions, protection plugins, adventure rules, latency and authoritative rejections remain effective. The mode cannot expose withheld blocks or bypass server anti-X-ray. Menus, focus loss, manual controls, damage, low health/food, tool/inventory problems, stale terrain and denied or stalled actions stop it without automatic retry.
