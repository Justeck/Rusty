# Auto Mine

Auto Mine is an explicitly started resource-mining mode. It finds a conservative route through terrain already received by the client, walks normally, and breaks the selected resource with an appropriate hotbar tool. X-Ray and Finder can remain off.

## Start and stop

1. Open **Right Shift → Auto Mine**.
2. Choose a **Resource**, **Search Radius**, and **Block Goal**. The resource button cycles through coal, iron, copper, gold, redstone, lapis, diamond, emerald, quartz, nether gold, ancient debris, and Custom. Overworld ore presets include their deepslate variants.
3. For Custom, aim at a block before opening the menu, then choose **Use Aimed Block**. Containers, unsafe blocks, unsuitable tools, and unbreakable blocks remain protected regardless of the selected ID.
4. Put suitable tools in your hotbar, release movement/attack/use, and enable Auto Mine. The menu closes before the mode starts.

Auto Mine has **no key assigned by default**. Assign an unused key in its settings for explicit start/stop. Movement and attack keys are unsuitable shortcuts because manual gameplay takes priority. Opening a menu, moving the mouse, or pressing a gameplay control stops the mode. A stop never becomes an automatic restart.

The native edition also supports `/rusty mine start`, `/rusty mine stop`, and `/rusty mine status`. Each setting has a Default control, and Restore defaults stops an active run.

## Defaults

| Setting | Default | Range / behavior |
|---|---|---|
| Resource | Diamond | Eleven ore presets and Custom |
| Search radius | 24 blocks | Horizontal radius 8–32; received vertical terrain only |
| Block goal | 16 | 1–256 observed, server-settled resource blocks |
| Minimum health | 12 / 20 | Stop below the threshold or after any observed damage |
| Minimum food | 10 / 20 | Stop below the threshold |
| Tool reserve | 10% | Do not use a tool at or below this remaining durability |
| Access tunnels | On | May remove selected resources and a small whitelist of ordinary terrain |
| Route preview | On | Bounded green route lines; independent from Finder tracers |

The goal counts resource **blocks**, rather than predicted clicks or a promised number of drops. Fortune, collection conditions, and server rules determine the actual item yield. A full inventory stops the run.

## Routing and safety

The planner weighs walking time and estimated mining time. It can choose a walking detour instead of a slow tunnel, or a short tunnel through safe ordinary terrain. “Quickest” means the lowest estimated cost within this bounded model, rather than a globally fastest real-world route. Tool effects, latency, dynamic terrain, entities and server rules can change the actual duration.

Routes require known support and room for a two-block-tall player. The mode rejects unknown or unloaded cells, fluids, dangerous or unstable blocks, partial collision shapes, unsupported steps and drops greater than one block. It does not bridge, sprint, parkour, or mine vertically beneath or directly above the player. Head clearance is opened before foot clearance. Containers and interactive block entities are excluded.

Every action is checked again against live terrain. Client-predicted openings remain unavailable to navigation until the game's normal server-acknowledgment bookkeeping settles them. A resource is counted only after that prediction is settled and the block is still air. Rejected breaks and missing confirmations terminate within a bounded wait.

After a resource break, the mode can step into its confirmed opening to collect drops when support, clearance and the remaining route permit it. Reaching the block goal clears further mining actions; an eligible collection step remains guarded before the run ends. Collection does not justify an unsupported step or changing the standing position assumed by a remaining clearance.

Manual input, menus, focus loss, damage, low health/food, unsuitable or worn tools, a full inventory, a changed world/dimension, missing chunks, an obstructing entity, or a stalled action stop the mode. Conflicting automatic tools are suspended while Auto Mine owns the controls; their preferences are preserved. A running session is never saved in a profile or resumed on connection.

## Multiplayer and limits

The Horizon edition uses ordinary client walking, selection and block-breaking controls. No server component or special resource requests are added. Fabric and NeoForge retain Rusty's existing permission policy: Auto Mine requires the current server's **view**, **movement**, and **assistance** grants; singleplayer is allowed.

Server-side anti-X-Ray can hide or substitute ore states. Auto Mine uses only what the client has received and cannot locate information withheld by the server. It may stop with no reachable resource even when ore exists elsewhere. There is no blind exploration or unlimited tunnelling.

The grid is capped at two million cells, vertical span at 512 blocks, and route search at 32,768 expansions. Scanning, distance preparation and search are spread over client ticks. Missing routes and exceeded limits produce a stop, never a partial route executed as a successful plan. Nonstandard dimensions and terrain can be declined conservatively.

Safety is risk reduction, not invulnerability. The mode cannot identify whether an ordinary stone or dirt state was naturally generated or player-placed. Damage and unforeseen world changes can still occur before the client observes them. See [validation](VALIDATION.md) for the actual tested environments and limits.
