package dev.justeck.rusty.core;

import java.util.Set;

/** Dependency-free behavioral acceptance, also invoked by Gradle check. */
public final class CoreAcceptance {
  private static int checks;

  private static void check(boolean value, String name) {
    if (!value) throw new AssertionError(name);
    checks++;
  }

  public static void main(String[] args) {
    var tracerOnly = new OverlayBuffer<String>();
    var tracersAndEsp = new OverlayBuffer<String>();
    var saturated = new OverlayBuffer<String>();
    for (int i = 0; i < 1000; i++) saturated.add("finder:" + i);
    for (int i = 0; i < 400; i++) {
      tracerOnly.addEntityTracer("endpoint:" + i);
      tracersAndEsp.add("box:" + i);
      tracersAndEsp.addEntityTracer("endpoint:" + i);
      saturated.addEntityTracer("endpoint:" + i);
    }
    var endpoints = tracerOnly.snapshot().stream().filter(v -> v.startsWith("endpoint:")).toList();
    check(
        endpoints.equals(
            tracersAndEsp.snapshot().stream().filter(v -> v.startsWith("endpoint:")).toList()),
        "ESP saturation must not evict tracer endpoints");
    check(
        endpoints.equals(
            saturated.snapshot().stream().filter(v -> v.startsWith("endpoint:")).toList()),
        "Finder saturation must not evict tracer endpoints");
    check(
        endpoints.size() == 128
            && endpoints.getFirst().equals("endpoint:0")
            && endpoints.getLast().equals("endpoint:127"),
        "entity tracers retain a stable independent 128-line cap");
    check(
        saturated.snapshot().size() == 896,
        "combined geometry remains bounded at 768 main entries plus 128 entity tracers");
    var savedSnapshot = tracerOnly.snapshot();
    tracerOnly.add("later box");
    check(savedSnapshot.size() == 128, "published geometry snapshots do not change after capture");
    check(
        new Settings().restoreField("tracerPlayers"), "tracers have an independent player setting");
    check(new Settings().restoreField("tracerMobs"), "tracers have an independent mob setting");
    check(new Settings().restoreField("tracerItems"), "tracers have an independent item setting");
    for (int esp = 0; esp < 8; esp++) {
      for (int tracer = 0; tracer < 8; tracer++) {
        var filters = new Settings();
        filters.espPlayers = (esp & 1) != 0;
        filters.espMobs = (esp & 2) != 0;
        filters.espItems = (esp & 4) != 0;
        filters.tracerPlayers = (tracer & 1) != 0;
        filters.tracerMobs = (tracer & 2) != 0;
        filters.tracerItems = (tracer & 4) != 0;
        for (boolean lines : new boolean[] {false, true}) {
          int wanted = lines ? tracer : esp;
          check(
              filters.entitySelected(lines, true, true, false) == ((wanted & 1) != 0),
              "independent player selection");
          check(
              filters.entitySelected(lines, false, true, false) == ((wanted & 2) != 0),
              "independent mob selection");
          check(
              filters.entitySelected(lines, false, false, true) == ((wanted & 4) != 0),
              "independent item selection");
          check(
              !filters.entitySelected(lines, false, false, false),
              "other entity types stay excluded");
        }
      }
    }
    var migrated = new Settings();
    migrated.espPlayers = false;
    migrated.espMobs = true;
    migrated.espItems = true;
    migrated.inheritLegacyTracerTargets(Set.of());
    check(
        !migrated.tracerPlayers && migrated.tracerMobs && migrated.tracerItems,
        "old shared choices migrate once");
    migrated.espPlayers = true;
    migrated.tracerMobs = false;
    migrated.inheritLegacyTracerTargets(Set.of("tracerPlayers", "tracerMobs", "tracerItems"));
    check(
        !migrated.tracerPlayers && !migrated.tracerMobs && migrated.tracerItems,
        "explicit new choices survive reload");
    check(
        migrated.restoreField("tracerMobs") && !migrated.tracerMobs && migrated.espMobs,
        "tracer Default preserves ESP");
    check(
        Catalog.ALL.stream()
            .anyMatch(m ->
                m.id().equals("enabled_mods")
                    && m.name().equals("Enabled Mods")
                    && m.category().equals("Interface")),
        "Enabled Mods is available as an independent interface feature");
    var priority =
        HudLayout.fit(
            java.util.List.of(
                new HudLayout.Panel("equipment_hud", 160, 100),
                new HudLayout.Panel("enabled_mods", 140, 40)),
            640,
            360,
            1);
    check(
        priority.panels().get("enabled_mods").x() == 8
            && priority.panels().get("enabled_mods").y() == 8,
        "Enabled Mods gets the top-left anchor before taller panels");
    check(
        Catalog.enabledNames(id -> Set.of("enabled_mods", "no_fog", "fullbright").contains(id))
            .equals(java.util.List.of("Fullbright", "No Fog")),
        "active list is alphabetical and excludes itself");
    check(
        Catalog.enabledNames(id -> id.equals("no_fog")).equals(java.util.List.of("No Fog")),
        "disabled names disappear");
    check(
        Catalog.enabledNames(id -> id.equals("enabled_mods")).isEmpty(),
        "list excludes itself when no other modules are active");
    var restore = new Settings();
    restore.scanRadius = 128;
    restore.context = false;
    restore.theme = "Copper";
    restore.targets.clear();
    check(
        restore.restoreField("scanRadius")
            && restore.scanRadius == 64
            && !restore.context
            && restore.theme.equals("Copper"),
        "one numeric default preserves other preferences");
    check(
        restore.restoreField("context") && restore.context && restore.theme.equals("Copper"),
        "true boolean default restores independently");
    check(
        restore.restoreField("targets") && restore.targets.equals(Settings.defaultTargets()),
        "target defaults use a fresh selection");
    restore.targets.clear();
    check(!Settings.defaultTargets().isEmpty(), "mutable default selections are not shared");
    check(
        !restore.restoreField("unknown")
            && !restore.restoreField("DEFAULT_MENU_SCALE")
            && restore.theme.equals("Copper"),
        "unknown and static fields cannot reset settings");
    restore.menuScale = 1.4;
    check(
        restore.restoreField("menuScale") && restore.menuScale == .7,
        "menu default retains compact 100 percent baseline");
    var state = new SessionPolicy();
    check(!state.allows("xray"), "no connection starts denied");
    state.connect("first", false);
    check(!state.allows("xray"), "remote without opt-in denied");
    check(state.allows("coordinates"), "ordinary information is client-side");
    check(!state.grant("stale", Set.of("view")), "wrong connection cannot grant");
    check(state.grant("first", Set.of("view")), "authoritative current connection grant");
    check(state.allows("xray") && !state.allows("trigger_bot"), "capabilities are distinct");
    state.connect("second", false);
    check(!state.allows("xray"), "reconnect clears previous permission");
    state.connect("local", true);
    check(state.allows("xray") && state.allows("trigger_bot"), "singleplayer grants locally");
    state.disconnect();
    check(!state.allows("xray"), "disconnect revokes");

    var input = new Activation();
    check(input.sample(true, false, true), "toggle press activates");
    check(input.sample(true, false, true), "held toggle does not retrigger");
    check(input.sample(false, false, true), "release preserves toggle");
    check(!input.sample(true, false, true), "second press deactivates");
    input.reset();
    check(input.sample(true, true, true), "hold activates");
    check(!input.sample(false, true, true), "hold release deactivates");
    check(!input.sample(true, true, false), "menu focus cancels hold");
    check(!input.sample(true, false, true), "held key after focus does not synthesize press");
    input.sample(false, false, true);
    check(input.sample(true, false, true), "fresh press works after focus");
    input.contextChanged(true, true);
    check(
        input.sample(true, false, true),
        "connection reset preserves a requested toggle without a synthetic press");
    input.sample(false, false, true);
    input.contextChanged(true, false);
    check(input.sample(false, false, true), "saved enabled tools survive joining a world");
    input.contextChanged(false, true);
    check(
        !input.sample(true, false, true),
        "a key held through connection change is not a new toggle press");

    var cfg = new Settings();
    check(cfg.menuScale == .7, "100 percent menu default is the former 70 percent physical size");
    for (double[] pair : new double[][] {{.35, 50}, {.7, 100}, {2.1, 300}})
      check(
          Math.abs(Settings.menuPercent(pair[0]) - pair[1]) < 1e-9,
          "percent uses compact baseline");
    check(
        Settings.fittedMenuScale(1920, 1080, .35) == .35,
        "new minimum is not clamped to old minimum");
    check(Settings.fittedMenuScale(320, 200, 2.1) <= .8, "large request fits a small window");
    for (double scale : new double[] {.35, .5, .7, 1, 2, 2.1}) {
      var savedMenu = new Settings();
      savedMenu.menuScale = scale;
      savedMenu.validate();
      check(
          savedMenu.menuScale == scale,
          "menu bounds include old physical sizes and new zoom endpoints");
    }
    check(cfg.textureVisibility == 25 && !cfg.holdXray, "readable texture default and toggle");
    check(
        cfg.targets.contains("minecraft:deepslate_diamond_ore")
            && cfg.targets.contains("minecraft:ancient_debris"),
        "typical ores selected");
    cfg.menuScale = Double.NaN;
    cfg.scanRadius = 100000;
    cfg.textureVisibility = -8;
    cfg.validate();
    check(
        cfg.menuScale == .7 && cfg.scanRadius == 128 && cfg.textureVisibility == 0,
        "corrupt numeric settings bounded");
    cfg.scanBudget = 1024;
    cfg.validate();
    check(cfg.scanBudget >= 4096, "accepted scan budget can process a complete section");
    cfg.targets.add("invalid block");
    cfg.targets.add("other:custom_ore");
    cfg.validate();
    check(
        !cfg.targets.contains("invalid block") && cfg.targets.contains("other:custom_ore"),
        "arbitrary valid registry IDs accepted");
    check(
        Math.abs(Settings.textureEdge(25) - (1 - Math.sqrt(.75)) / 2) < 1e-9,
        "face area slider maps to texture frame");
    check(
        Settings.textureEdge(0) > 0 && Settings.textureEdge(100) == .5,
        "minimum thin edge maximum ordinary face");

    var targets = new TargetIndex(2);
    targets.begin("a");
    targets.put("a", 1, "minecraft:diamond_ore");
    targets.put("a", 2, "minecraft:gold_ore");
    targets.put("a", 3, "minecraft:coal_ore");
    check(targets.size() == 2 && targets.partial(), "target budget explicit");
    targets.remove(1);
    check(targets.size() == 1, "block update removal");
    targets.begin("b");
    targets.put("a", 4, "minecraft:diamond_ore");
    check(targets.size() == 0, "world change clears and stale work rejected");
    targets.put("b", 5, "minecraft:diamond_ore");
    targets.clear();
    check(targets.size() == 0 && !targets.partial(), "disconnect releases cache");
    var hud = new java.util.ArrayList<HudLayout.Panel>();
    for (int i = 0; i < 17; i++) hud.add(new HudLayout.Panel("text" + i, 160, 20));
    hud.add(new HudLayout.Panel("minimap", 96, 96));
    hud.add(new HudLayout.Panel("enabled_mods", 140, 40));
    for (int[] size : new int[][] {{320, 180}, {640, 360}, {1280, 720}}) {
      var layout = HudLayout.fit(hud, size[0], size[1], 1);
      check(layout.fits() && layout.panels().size() == 19, "all 19 default panels fit " + size[0]);
      var rectangles = new java.util.ArrayList<>(layout.panels().values());
      for (var panel : hud)
        check(
            layout.panels().get(panel.id()).width() >= Math.ceil(panel.width() * layout.scale()),
            "default fit preserves full readout width instead of truncating it");
      for (int i = 0; i < rectangles.size(); i++) {
        var rect = rectangles.get(i);
        check(
            rect.x() >= 0
                && rect.y() >= 0
                && rect.x() + rect.width() <= size[0]
                && rect.y() + rect.height() <= size[1] - 36,
            "HUD stays inside viewport and above hotbar");
        for (int j = i + 1; j < rectangles.size(); j++)
          check(!rect.overlaps(rectangles.get(j)), "default HUD panels do not overlap");
      }
    }
    try {
      var directory = java.nio.file.Files.createTempDirectory("rusty-settings-test-");
      var file = directory.resolve("settings.json");
      AtomicSettings.write(file, "first");
      AtomicSettings.write(file, "replacement");
      check(
          java.nio.file.Files.readString(file).equals("replacement"),
          "settings replace actual existing file");
      try (var files = java.nio.file.Files.list(directory)) {
        check(files.count() == 1, "settings leave no temporary files");
      }
      java.nio.file.Files.delete(file);
      java.nio.file.Files.delete(directory);
    } catch (java.io.IOException e) {
      throw new AssertionError("actual settings write", e);
    }
    var fast = new Activation();
    check(
        fast.sampleQueued(false, false, true, 1),
        "brief press retained after release before game tick");
    check(!fast.sampleQueued(false, false, true, 1), "second brief press toggles back off");
    check(!fast.sampleQueued(false, false, true, 2), "two queued taps preserve parity");
    check(
        fast.sampleQueued(true, false, true, 1) && fast.sampleQueued(true, false, true, 0),
        "queued held press toggles exactly once");
    check(!fast.sampleQueued(false, true, true, 1), "released hold tap never sticks on");
    System.out.println("Rusty shared core: " + checks + " checks passed");
  }
}
