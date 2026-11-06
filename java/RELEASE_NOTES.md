# Rusty Java 1.3.0-beta.7

**Auto Mine joins the Minecraft-style 44-tool interface.**

Choose one of eleven ore presets or an aimed Custom block, set a search radius and block goal, and explicitly start Auto Mine. The bounded planner weighs walking and mining time, validates supported steps, can open conservative access tunnels, and displays an optional green route preview. Ordinary game controls handle movement, tool selection and breaking; client-predicted air does not count until normal server acknowledgments settle it.

Manual input, menus, focus loss, damage, low health/food, unsuitable or worn tools, a full inventory, unsafe/changed terrain, crossing entities, world changes or denied/stalled actions stop the mode. Stops never restart themselves. Auto Mine is off and unbound by default, with individual defaults and the global restore control. Existing X-Ray, Finder, tracer filters, visual HUD editing and other tools are retained.

Fabric and NeoForge for **Minecraft26.3 / Java25**. Use one matching loader jar. Fabric requires Loader0.20.1+ and Fabric API0.165.2+26.3; NeoForge targets26.3.0.101. Multiplayer Auto Mine requires the existing **view + movement + assistance** grants from the current server. It cannot discover server-hidden ore, and conservative routing is not an invulnerability guarantee.

[Auto Mine guide](https://github.com/Justeck/Rusty/blob/main/docs/AUTO_MINE.md) · [Validation](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md) · [Horizon-native release](https://github.com/Justeck/Rusty/releases/tag/v1.3.0).
