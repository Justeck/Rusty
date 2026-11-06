package dev.justeck.rusty.core;

import java.util.List;
import java.util.Locale;

public final class Catalog {
  private Catalog() {}

  public static List<String> enabledNames(java.util.function.Predicate<String> active) {
    return ALL.stream()
        .filter(m -> !m.id().equals("enabled_mods") && active.test(m.id()))
        .map(Module::name)
        .sorted()
        .toList();
  }

  public record Module(String id, String name, String category, String description) {
    public boolean matches(String query) {
      return (name + " " + category + " " + description)
          .toLowerCase(Locale.ROOT)
          .contains(query.toLowerCase(Locale.ROOT));
    }
  }

  public static final List<Module> ALL =
      List.of(
          new Module(
              "auto_mine",
              "Auto Mine",
              "World",
              "Explicit resource mining with conservative received-terrain routes, normal controls,"
                  + " quotas and safety stops."),
          new Module(
              "xray",
              "X-Ray",
              "Render",
              "Reveal selected received blocks while retaining textured world context."),
          new Module(
              "finder",
              "Finder",
              "World",
              "Independently enable target boxes and tracer lines for selected loaded blocks."),
          new Module(
              "entity_esp",
              "Entity ESP",
              "Render",
              "Highlight received entities with selectable player, mob and item filters."),
          new Module(
              "fullbright",
              "Fullbright",
              "Render",
              "Brighten the local world view for visibility in dark areas."),
          new Module(
              "no_fog",
              "No Fog",
              "Render",
              "Remove local distance fog without revealing unreceived terrain."),
          new Module(
              "clear_weather",
              "Clear Weather",
              "Render",
              "Hide local weather effects while server weather remains authoritative."),
          new Module(
              "zoom", "Zoom", "Camera", "Magnify the camera view while holding the zoom key."),
          new Module(
              "coordinates",
              "Coordinates",
              "Navigation",
              "Show current coordinates and dimension in the local HUD."),
          new Module(
              "auto_sprint",
              "Auto Sprint",
              "Movement",
              "Request normal sprinting while walking forward."),
          new Module(
              "no_hurt_camera",
              "No Hurt Camera",
              "Player",
              "Suppress local camera shake from damage."),
          new Module(
              "freecam",
              "Freecam",
              "Camera",
              "Detach the local camera while the player body continues normal simulation."),
          new Module(
              "status_hud",
              "Status HUD",
              "Interface",
              "Show active Rusty state and optional target counts."),
          new Module(
              "enabled_mods",
              "Enabled Mods",
              "Interface",
              "List active Rusty features vertically in the top-left. Move or resize it in HUD"
                  + " layout."),
          new Module(
              "compass",
              "Compass",
              "Navigation",
              "Show your cardinal direction and yaw using the current player rotation."),
          new Module(
              "waypoints",
              "Waypoints",
              "Navigation",
              "Save local markers, scoped to each world/server and dimension, with distance and"
                  + " bearing HUD."),
          new Module(
              "breadcrumbs",
              "Breadcrumbs",
              "Navigation",
              "Trace observed body movement with a bounded trail; world changes and teleports break"
                  + " the path."),
          new Module(
              "equipment_hud",
              "Equipment HUD",
              "Inventory",
              "Show received armor and held-item remaining durability."),
          new Module(
              "environment_hud",
              "Environment HUD",
              "World",
              "Show the current received biome and local block and sky light; missing data stays"
                  + " unavailable."),
          new Module(
              "container_preview",
              "Container Preview",
              "Inventory",
              "List embedded contents supplied with the held item, without opening it or querying"
                  + " world chests."),
          new Module(
              "chunk_borders",
              "Chunk Borders",
              "World",
              "Show the current chunk boundaries and section heights."),
          new Module(
              "light_overlay",
              "Light Risk",
              "World",
              "Mark dark and skylit full-block floors using received light. Not a complete spawn"
                  + " predictor."),
          new Module(
              "storage_esp",
              "Storage ESP",
              "Render",
              "Outline received nearby storage block entities without reading hidden contents."),
          new Module(
              "trajectories",
              "Trajectories",
              "Render",
              "Estimate bow, pearl, snowball and egg paths against received block geometry and"
                  + " fluid drag. Entity collisions, spread and future movement are not"
                  + " predicted."),
          new Module(
              "target_info",
              "Target Info",
              "World",
              "Inspect the received block and properties at the ordinary crosshair."),
          new Module(
              "portal_coordinates",
              "Portal Coordinates",
              "Navigation",
              "Display the corresponding Overworld or Nether XZ coordinates; no portal or"
                  + " destination scan."),
          new Module(
              "speed_hud",
              "Speed HUD",
              "Interface",
              "Show observed body speed, resetting on discontinuities."),
          new Module(
              "clock_hud",
              "World Clock",
              "Interface",
              "Show received world time as a 24-hour clock."),
          new Module(
              "entity_tracers",
              "Entity Tracers",
              "Render",
              "Choose independent player, mob and dropped-item tracers within the search radius."),
          new Module(
              "safe_walk",
              "Safe Walk",
              "Movement",
              "Request normal crouching near a ledge while grounded; respects jumping and collision"
                  + " shapes."),
          new Module(
              "auto_jump",
              "Auto Jump",
              "Movement",
              "Request a normal jump over a received obstacle with clear headroom."),
          new Module(
              "auto_tool",
              "Auto Tool",
              "Inventory",
              "Choose a suitable hotbar tool while you mine; preserve drops and protect low"
                  + " durability."),
          new Module(
              "trigger_bot",
              "Trigger Bot",
              "Combat",
              "Attack a living crosshair target through the ordinary client action at full cooldown"
                  + " and normal reach."),
          new Module(
              "auto_eat",
              "Auto Eat",
              "Player",
              "Eat ordinary hotbar food below a hunger threshold; menus and manual actions cancel."
                  + " Toggle off/on to retry an unchanged cancelled or rejected stack."),
          new Module(
              "armor_assist",
              "Armor Assist",
              "Inventory",
              "Fill empty armor slots with ordinary clicks; preserve worn gear and skip binding"
                  + " curses. Toggle off/on to rearm an identical attempted source."),
          new Module(
              "refill",
              "Auto Refill",
              "Inventory",
              "Refill observed depletion with an exact matching inventory stack. Never clear a slot"
                  + " or blindly retry a correction. Toggle off/on to rearm identical attempted"
                  + " stacks."),
          new Module(
              "build_planner",
              "Build Planner",
              "Building",
              "Preview a bounded cuboid plan and explicitly place one reachable held block at a"
                  + " time through normal client actions."),
          new Module(
              "auto_fish",
              "Auto Fish",
              "Player",
              "Reel an owned fishing hook after a received bite. Cast once yourself to begin;"
                  + " respects normal use cooldown."),
          new Module(
              "auto_totem",
              "Auto Totem",
              "Inventory",
              "Move a hotbar totem into an empty offhand with a normal swap; preserve other offhand"
                  + " items. Toggle off/on to rearm an identical attempted source."),
          new Module(
              "death_markers",
              "Death Markers",
              "Navigation",
              "Save one waypoint after an observed death in the current world and dimension; keep"
                  + " your selected marker unchanged."),
          new Module(
              "waypoint_walk",
              "Waypoint Walk",
              "Navigation",
              "Explicitly walk to the selected waypoint through loaded terrain. Manual input or"
                  + " opening a menu stops walking."),
          new Module(
              "minimap",
              "Minimap",
              "Navigation",
              "Draw a bounded surface-color map of received chunks, with optional rotation and"
                  + " round framing. Underground layers are not provided."),
          new Module(
              "fps_hud",
              "Frame Rate",
              "Interface",
              "Show the current frame rate reported by Minecraft."),
          new Module(
              "clicks_hud",
              "Click Counter",
              "Interface",
              "Count ordinary mouse presses received over the last second."));
}
