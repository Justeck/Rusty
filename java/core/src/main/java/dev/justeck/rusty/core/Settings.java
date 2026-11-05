package dev.justeck.rusty.core;

import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;

public final class Settings {
  // Saved physical sizes are unchanged; the compact baseline is displayed as 100%.
  public static final double DEFAULT_MENU_SCALE = .7, MIN_MENU_SCALE = .35, MAX_MENU_SCALE = 2.1;
  public double textureVisibility = 25, menuScale = DEFAULT_MENU_SCALE;

  public static double menuPercent(double scale) {
    return scale / DEFAULT_MENU_SCALE * 100;
  }

  public static double fittedMenuScale(double width, double height, double requested) {
    double scale = bound(requested, MIN_MENU_SCALE, MAX_MENU_SCALE, DEFAULT_MENU_SCALE);
    return Math.max(.001, Math.min(scale, Math.min(width / 390, height / 250)));
  }

  public int scanRadius = 64;
  public boolean holdXray;
  public int schema = 1, maxTargets = 1024, scanBudget = 8192, minY = -64, maxY = 319;
  public int entityRange = 64, tracerLimit = 32, fishDelay = 4;
  public double hudScale = 1, zoomFactor = 4, cameraSpeed = .6;
  public boolean context = true, tracers = true, outlines = true, reducedMotion, triggerPlayers;
  public boolean espPlayers = true, espMobs, espItems, mapRound = true, mapRotate;
  public boolean tracerPlayers = true, tracerMobs, tracerItems;

  public boolean entitySelected(boolean tracer, boolean player, boolean living, boolean item) {
    if (player) return tracer ? tracerPlayers : espPlayers;
    if (living) return tracer ? tracerMobs : espMobs;
    return item && (tracer ? tracerItems : espItems);
  }

  /** Old profiles shared the ESP choices. Copy only missing tracer preferences once. */
  public void inheritLegacyTracerTargets(Set<String> fields) {
    if (!fields.contains("tracerPlayers")) tracerPlayers = espPlayers;
    if (!fields.contains("tracerMobs")) tracerMobs = espMobs;
    if (!fields.contains("tracerItems")) tracerItems = espItems;
  }

  public int mapSize = 96, plannerWidth = 3, plannerDepth = 3, plannerHeight = 1, eatBelow = 16;
  public boolean plannerHollow, plannerFloor = true, plannerRoof = true;
  public String theme = "Classic";
  public Set<String> enabled = new LinkedHashSet<>(Set.of("status_hud"));
  public Set<String> favorites = new LinkedHashSet<>();
  public java.util.List<String> recent = new java.util.ArrayList<>();
  public Map<String, Integer> bindings = new LinkedHashMap<>();
  public Map<String, double[]> overlays = new LinkedHashMap<>();
  public Map<String, Double> overlayScales = new LinkedHashMap<>();
  public Set<String> targets = defaultTargets();

  public static Set<String> defaultTargets() {
    var result = new LinkedHashSet<String>();
    for (String material :
        new String[] {
          "coal", "iron", "copper", "gold", "redstone", "lapis", "diamond", "emerald"
        }) {
      result.add("minecraft:" + material + "_ore");
      result.add("minecraft:deepslate_" + material + "_ore");
    }
    result.add("minecraft:nether_gold_ore");
    result.add("minecraft:nether_quartz_ore");
    result.add("minecraft:ancient_debris");
    return result;
  }

  /** Restore one public preference from a fresh default settings object. */
  public boolean restoreField(String name) {
    try {
      var field = Settings.class.getField(name);
      if (java.lang.reflect.Modifier.isStatic(field.getModifiers())) return false;
      field.set(this, field.get(new Settings()));
      if (name.equals("minY")) maxY = Math.max(maxY, minY);
      if (name.equals("maxY")) minY = Math.min(minY, maxY);
      validate();
      return true;
    } catch (ReflectiveOperationException e) {
      return false;
    }
  }

  public void validate() {
    menuScale = bound(menuScale, MIN_MENU_SCALE, MAX_MENU_SCALE, DEFAULT_MENU_SCALE);
    hudScale = bound(hudScale, .5, 2, 1);
    textureVisibility = bound(textureVisibility, 0, 100, 25);
    zoomFactor = bound(zoomFactor, 1.5, 12, 4);
    cameraSpeed = bound(cameraSpeed, .05, 4, .6);
    scanRadius = Math.clamp(scanRadius, 8, 128);
    scanBudget = Math.clamp(scanBudget, 4096, 32768);
    maxTargets = Math.clamp(maxTargets, 16, 4096);
    entityRange = Math.clamp(entityRange, 8, 128);
    tracerLimit = Math.clamp(tracerLimit, 0, 128);
    minY = Math.clamp(minY, -2048, 2048);
    maxY = Math.clamp(maxY, minY, 2048);
    fishDelay = Math.clamp(fishDelay, 2, 20);
    eatBelow = Math.clamp(eatBelow, 1, 19);
    mapSize = Math.clamp(mapSize, 48, 192);
    plannerWidth = Math.clamp(plannerWidth, 1, 16);
    plannerDepth = Math.clamp(plannerDepth, 1, 16);
    plannerHeight = Math.clamp(plannerHeight, 1, 16);
    if (targets == null) targets = defaultTargets();
    targets.removeIf(
        s -> s == null || s.length() > 256 || !s.matches("[a-z0-9_.-]+:[a-z0-9/._-]+"));
    targets = new LinkedHashSet<>(targets.stream().limit(256).toList());
    if (enabled == null) enabled = new LinkedHashSet<>();
    if (favorites == null) favorites = new LinkedHashSet<>();
    if (bindings == null) bindings = new LinkedHashMap<>();
    var known = new LinkedHashSet<String>();
    Catalog.ALL.forEach(m -> known.add(m.id()));
    enabled.retainAll(known);
    favorites.retainAll(known);
    if (recent == null) recent = new java.util.ArrayList<>();
    recent =
        new java.util.ArrayList<>(
            recent.stream().filter(known::contains).distinct().limit(12).toList());
    bindings.keySet().removeIf(k -> !known.contains(k) && !k.equals("menu"));
    bindings.replaceAll((k, v) -> v == null || v < 0 || v >= 512 ? -1 : v);
    if (overlays == null) overlays = new LinkedHashMap<>();
    if (overlayScales == null) overlayScales = new LinkedHashMap<>();
    overlayScales.replaceAll((k, v) -> v == null ? 1 : bound(v, .5, 2, 1));
    overlays.entrySet().removeIf(e -> e.getValue() == null || e.getValue().length != 2);
    overlays
        .values()
        .forEach(v -> {
          v[0] = bound(v[0], 0, 1, .02);
          v[1] = bound(v[1], 0, 1, .02);
        });
    if (theme == null || !Set.of("Classic", "Graphite", "Copper").contains(theme))
      theme = "Classic";
  }

  private static double bound(double value, double min, double max, double fallback) {
    return Double.isFinite(value) ? Math.clamp(value, min, max) : fallback;
  }

  public static double textureEdge(double percent) {
    return Math.max(1.0 / 256, (1 - Math.sqrt(1 - bound(percent, 0, 100, 25) / 100)) / 2);
  }
}
