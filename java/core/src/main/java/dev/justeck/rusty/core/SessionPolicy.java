package dev.justeck.rusty.core;

import java.util.Set;

public final class SessionPolicy {
  private String identity;
  private boolean local;
  private Set<String> grants = Set.of();

  public static String required(String module) {
    return switch (module) {
      case "xray", "finder", "entity_esp", "entity_tracers", "storage_esp", "freecam" -> "view";
      case "auto_mine", "area_clear" -> "view + movement + assistance";
      case "trigger_bot" -> "combat";
      case "auto_sprint", "safe_walk", "auto_jump", "waypoint_walk" -> "movement";
      case "auto_tool",
          "auto_eat",
          "auto_fish",
          "auto_totem",
          "armor_assist",
          "refill",
          "build_planner" ->
          "assistance";
      default -> "";
    };
  }

  public void connect(String identity, boolean local) {
    if (identity == null || identity.isBlank())
      throw new IllegalArgumentException("connection identity required");
    this.identity = identity;
    this.local = local;
    grants = Set.of();
  }

  public boolean allows(String module) {
    if (module.equals("auto_mine") || module.equals("area_clear"))
      return identity != null
          && (local || grants.containsAll(Set.of("view", "movement", "assistance")));
    String permission = required(module);
    return permission.isEmpty() || (identity != null && (local || grants.contains(permission)));
  }

  /** Invoke only from the current connection's registered server payload receiver. */
  public boolean grant(String identity, Set<String> capabilities) {
    if (this.identity == null
        || !this.identity.equals(identity)
        || capabilities == null
        || !Set.of("view", "combat", "movement", "assistance").containsAll(capabilities))
      return false;
    grants = Set.copyOf(capabilities);
    return true;
  }

  public void disconnect() {
    identity = null;
    local = false;
    grants = Set.of();
  }
}
