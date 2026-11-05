package dev.justeck.rusty;

import dev.justeck.rusty.core.HudLayout;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.core.BlockPos;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.level.LightLayer;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;

public final class RustyHud {
  public static final List<String> IDS =
      List.of(
          "enabled_mods",
          "status_hud",
          "coordinates",
          "finder",
          "compass",
          "waypoints",
          "equipment_hud",
          "environment_hud",
          "container_preview",
          "build_planner",
          "fps_hud",
          "clicks_hud",
          "portal_coordinates",
          "clock_hud",
          "speed_hud",
          "target_info",
          "light_overlay",
          "trajectories",
          "minimap");
  public static final Map<String, int[]> bounds = new LinkedHashMap<>();
  private static final ArrayDeque<Long> clicks = new ArrayDeque<>();
  private static volatile Map<String, List<String>> panels = Map.of();
  private static volatile int[] map = new int[24 * 24];
  private static int mapCursor;

  public static void reset() {
    panels = Map.of();
    map = new int[24 * 24];
    Arrays.fill(map, 0xff333333);
    mapCursor = 0;
    clicks.clear();
    bounds.clear();
  }

  public static void click() {
    long now = System.nanoTime();
    clicks.addLast(now);
    while (clicks.size() > 256) clicks.removeFirst();
  }

  public static void capture() {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    var l = mc.level;
    if (p == null || l == null) {
      panels = Map.of();
      return;
    }
    var next = new LinkedHashMap<String, List<String>>();
    var activeNames = dev.justeck.rusty.core.Catalog.enabledNames(RustyClient::active);
    next.put("enabled_mods", activeNames.isEmpty() ? List.of("None enabled") : activeNames);
    long count = RustyClient.config.enabled.stream().filter(RustyClient.POLICY::allows).count();
    next.put(
        "status_hud",
        List.of(
            "Rusty  |  " + count + " enabled",
            RustyClient.targets.size() + " targets" + (RustyClient.partial ? "  [partial]" : "")));
    next.put(
        "coordinates",
        List.of(
            String.format(Locale.ROOT, "XYZ  %.1f / %.1f / %.1f", p.getX(), p.getY(), p.getZ()),
            l.dimension().identifier().getPath()));
    float yaw = net.minecraft.util.Mth.wrapDegrees(p.getYRot());
    String[] cardinal = {"S", "W", "N", "E"};
    next.put(
        "compass",
        List.of(
            cardinal[Math.floorMod(Math.round(yaw / 90), 4)]
                + String.format(Locale.ROOT, "  %.0f degrees", yaw)));
    next.put(
        "finder",
        List.of(
            "Finder  " + RustyClient.targets.size() + (RustyClient.partial ? "+" : ""),
            RustyClient.targets.isEmpty()
                ? "Scanning received sections..."
                : "Selected blocks in loaded chunks"));
    var marker = RustyNavigation.selected();
    next.put(
        "waypoints",
        marker == null
            ? List.of("No selected waypoint")
            : List.of(
                marker.name(),
                String.format(
                    Locale.ROOT,
                    "%.1f m  |  %.0f / %.0f / %.0f",
                    p.position().distanceTo(marker.position()),
                    marker.x(),
                    marker.y(),
                    marker.z())));
    var gear = new ArrayList<String>();
    for (var slot :
        List.of(
            EquipmentSlot.MAINHAND,
            EquipmentSlot.HEAD,
            EquipmentSlot.CHEST,
            EquipmentSlot.LEGS,
            EquipmentSlot.FEET)) {
      var stack = p.getItemBySlot(slot);
      if (stack.isEmpty()) continue;
      gear.add(
          stack.getHoverName().getString()
              + (stack.isDamageableItem()
                  ? "  "
                      + (stack.getMaxDamage() - stack.getDamageValue())
                      + "/"
                      + stack.getMaxDamage()
                  : ""));
    }
    next.put("equipment_hud", gear.isEmpty() ? List.of("No held or worn equipment") : gear);
    boolean lightReady =
        l.getChunkSource()
            .getLightEngine()
            .lightOnInColumn(
                net.minecraft.world.level.ChunkPos.pack(p.getBlockX() >> 4, p.getBlockZ() >> 4));
    next.put(
        "environment_hud",
        List.of(
            l.getBiome(p.blockPosition())
                .unwrapKey()
                .map(k -> k.identifier().getPath())
                .orElse("Unknown biome"),
            lightReady
                ? "Light  B "
                    + l.getBrightness(LightLayer.BLOCK, p.blockPosition())
                    + " / S "
                    + l.getBrightness(LightLayer.SKY, p.blockPosition())
                : "Light unavailable"));
    var container = p.getMainHandItem().get(DataComponents.CONTAINER);
    var contents = new ArrayList<String>();
    if (container != null)
      container
          .nonEmptyItemCopyStream()
          .limit(6)
          .forEach(s -> contents.add(s.getCount() + "x " + s.getHoverName().getString()));
    next.put(
        "container_preview",
        contents.isEmpty() ? List.of("No received embedded contents") : contents);
    next.put(
        "build_planner",
        List.of(
            "Plan: " + RustyPlanner.cells.size() + " visible cells",
            "Anchor and Place one from the menu"));
    next.put("fps_hud", List.of(mc.getFps() + " FPS"));
    long now = System.nanoTime();
    while (!clicks.isEmpty() && now - clicks.peekFirst() > 1_000_000_000L) clicks.removeFirst();
    next.put("clicks_hud", List.of(clicks.size() + " mouse presses / second"));
    boolean nether = l.dimension().identifier().getPath().equals("the_nether");
    double factor = nether ? 8 : .125;
    next.put(
        "portal_coordinates",
        List.of(
            (nether ? "Overworld" : "Nether")
                + String.format(
                    Locale.ROOT, "  %.0f / %.0f", p.getX() * factor, p.getZ() * factor)));
    long minutes = Math.floorMod(l.getOverworldClockTime() + 6000, 24000) * 60 / 1000;
    next.put(
        "clock_hud",
        List.of(String.format(Locale.ROOT, "World time  %02d:%02d", minutes / 60, minutes % 60)));
    next.put(
        "speed_hud",
        List.of(String.format(Locale.ROOT, "Body speed  %.2f m/s", RustyClient.speed)));
    if (mc.hitResult instanceof BlockHitResult hit) {
      var state = l.getBlockState(hit.getBlockPos());
      next.put(
          "target_info",
          List.of(
              BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString(),
              fontTrim(
                  state
                      .getValues()
                      .map(Object::toString)
                      .collect(java.util.stream.Collectors.joining(", ")),
                  56)));
    } else next.put("target_info", List.of("No normally reachable block"));
    next.put(
        "light_overlay",
        List.of(
            "Light Risk  |  observed floor samples", "Darkness is not a full spawn prediction"));
    next.put("trajectories", List.of(RustyOverlay.trajectoryStatus));
    panels = Map.copyOf(next);
    if (RustyClient.active("minimap")) updateMap();
  }

  private static String fontTrim(String s, int length) {
    return s.length() > length ? s.substring(0, length - 3) + "..." : s;
  }

  private static void updateMap() {
    var mc = Minecraft.getInstance();
    int[] next = map.clone();
    double angle = RustyClient.config.mapRotate ? Math.toRadians(mc.player.getYRot() + 180) : 0;
    for (int work = 0; work < 32; work++) {
      int index = mapCursor++ % (24 * 24), u = index % 24 - 12, v = index / 24 - 12;
      int x = (int) Math.floor(mc.player.getX() + (Math.cos(angle) * u - Math.sin(angle) * v) * 3),
          z = (int) Math.floor(mc.player.getZ() + (Math.sin(angle) * u + Math.cos(angle) * v) * 3);
      var chunk = mc.level.getChunkSource().getChunk(x >> 4, z >> 4, ChunkStatus.FULL, false);
      if (chunk == null) {
        next[index] = 0xff333333;
        continue;
      }
      int y = chunk.getHeight(Heightmap.Types.WORLD_SURFACE, x & 15, z & 15) - 1;
      var pos = new BlockPos(x, y, z);
      next[index] = 0xff000000 | mc.level.getBlockState(pos).getMapColor(mc.level, pos).col;
    }
    map = next;
  }

  public static void draw(GuiGraphicsExtractor g, boolean editor) {
    var mc = Minecraft.getInstance();
    if (!editor && (mc.player == null || mc.gui.hud.isHidden())) return;
    bounds.clear();
    var measures = new ArrayList<HudLayout.Panel>();
    var extraScales = new HashMap<String, Double>();
    for (String id : IDS) {
      if (!editor && !RustyClient.active(id)) continue;
      var lines = panels.getOrDefault(id, List.of(id.replace('_', ' ')));
      double extra = RustyClient.config.overlayScales.getOrDefault(id, 1.0);
      int iw = lines.stream().mapToInt(mc.font::width).max().orElse(80) + 12,
          ih = lines.size() * 12 + 8;
      if (id.equals("minimap")) iw = ih = RustyClient.config.mapSize;
      extra = Math.min(extra, Math.max(.5, (g.guiHeight() - 44) * 2.0 / ih));
      extraScales.put(id, extra);
      measures.add(
          new HudLayout.Panel(id, (int) Math.ceil(iw * extra), (int) Math.ceil(ih * extra)));
    }
    var layout =
        HudLayout.fit(
            measures, g.guiWidth(), g.guiHeight() - (editor ? 24 : 0), RustyClient.config.hudScale);
    if (!layout.fits()) {
      g.text(mc.font, "Rusty: reduce HUD size or enable fewer panels.", 4, 4, 0xffd6b990);
      return;
    }
    for (String id : IDS) {
      var rect = layout.panels().get(id);
      if (rect == null) continue;
      List<String> lines = panels.getOrDefault(id, List.of(id.replace('_', ' ')));
      double s = layout.scale() * extraScales.get(id);
      int bw = rect.width(), bh = rect.height(), x = rect.x(), y = rect.y();
      if (id.equals("minimap")) s = Math.min(s, (double) bw / RustyClient.config.mapSize);
      double[] custom = RustyClient.config.overlays.get(id);
      if (custom != null) {
        x = (int) (custom[0] * g.guiWidth());
        y = (int) (custom[1] * g.guiHeight());
      }
      x = Math.clamp(x, 0, Math.max(0, g.guiWidth() - bw));
      y = Math.clamp(y, 0, Math.max(0, g.guiHeight() - bh));
      bounds.put(id, new int[] {x, y, bw, bh});
      g.fill(x, y, x + bw, y + bh, 0xb51c1916);
      if (editor) g.outline(x, y, bw, bh, 0xffc57b40);
      g.pose().pushMatrix();
      g.pose().translate(x, y);
      g.pose().scale((float) s);
      if (id.equals("minimap")) {
        int size = RustyClient.config.mapSize;
        int[] colors = map;
        for (int row = 0; row < 24; row++)
          for (int col = 0; col < 24; col++) {
            if (RustyClient.config.mapRound && Math.hypot(col - 11.5, row - 11.5) > 11.5) continue;
            g.fill(
                col * size / 24,
                row * size / 24,
                (col + 1) * size / 24,
                (row + 1) * size / 24,
                colors[row * 24 + col]);
          }
        g.fill(size / 2 - 1, size / 2 - 2, size / 2 + 2, size / 2 + 2, 0xfff0c27b);
        g.text(mc.font, RustyClient.config.mapRotate ? "^" : "N", size / 2 - 3, 2, 0xffeeeeee);
      } else {
        int ly = 4;
        for (String line : lines) {
          g.text(
              mc.font,
              mc.font.plainSubstrByWidth(line, Math.max(8, (int) (bw / s) - 12)),
              6,
              ly,
              0xffeee7de);
          ly += 12;
        }
      }
      g.pose().popMatrix();
    }
  }
}
