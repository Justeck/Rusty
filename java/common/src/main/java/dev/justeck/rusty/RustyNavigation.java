package dev.justeck.rusty;

import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.level.storage.LevelResource;
import net.minecraft.world.phys.Vec3;

/** Local marker storage and explicit, bounded walking through received collision data. */
public final class RustyNavigation {
  public record Marker(String name, double x, double y, double z, boolean death) {
    public Vec3 position() {
      return new Vec3(x, y, z);
    }
  }

  public static final class Store {
    public Map<String, List<Marker>> worlds = new LinkedHashMap<>();
    public Map<String, Integer> selected = new LinkedHashMap<>();
  }

  private static Store store = new Store();
  private static boolean loaded, wasAlive;
  private static String scope = "";
  private static final ArrayDeque<BlockPos> route = new ArrayDeque<>();
  private static int timeout;
  private static float lastYaw;

  public static void reset() {
    route.clear();
    scope = "";
    wasAlive = false;
  }

  private static Path path() {
    return RustyClient.configPath().resolveSibling("rusty-waypoints.json");
  }

  public static void tick(boolean focused) {
    var mc = Minecraft.getInstance();
    if (mc.player == null || mc.level == null) {
      cancelWalk();
      return;
    }
    if (!loaded) {
      loaded = true;
      try {
        if (Files.exists(path()) && Files.size(path()) <= 1024 * 1024) {
          var next = RustyClient.GSON.fromJson(Files.readString(path()), Store.class);
          if (next != null && next.worlds != null && next.selected != null) {
            next.worlds
                .entrySet()
                .removeIf(
                    e -> e.getKey() == null || e.getKey().length() > 256 || e.getValue() == null);
            next.worlds.replaceAll((key, list) ->
                new ArrayList<>(
                    list.stream()
                        .filter(m ->
                            m != null
                                && m.name() != null
                                && m.name().length() <= 64
                                && Double.isFinite(m.x())
                                && Double.isFinite(m.y())
                                && Double.isFinite(m.z())
                                && Math.abs(m.x()) <= 30000000
                                && Math.abs(m.z()) <= 30000000
                                && Math.abs(m.y()) <= 4096)
                        .limit(64)
                        .toList()));
            next.selected.entrySet().removeIf(e -> e.getKey() == null || e.getValue() == null);
            store = next;
          }
        }
      } catch (Exception e) {
        RustyClient.LOG.warn("Markers could not be read", e);
      }
    }
    String next = worldKey();
    if (!scope.equals(next)) {
      scope = next;
      cancelWalk();
      wasAlive = !mc.player.isDeadOrDying();
    }
    boolean alive = !mc.player.isDeadOrDying();
    if (wasAlive && !alive && RustyClient.active("death_markers")) {
      var list = current();
      var selected = selected();
      list.removeIf(Marker::death);
      var death =
          new Marker("Last death", mc.player.getX(), mc.player.getY(), mc.player.getZ(), true);
      if (list.size() >= 64) list.removeFirst();
      list.add(death);
      store.selected.put(
          scope,
          selected != null && !selected.death() && list.contains(selected)
              ? list.indexOf(selected)
              : list.size() - 1);
      save();
    }
    wasAlive = alive;
    if (!walking()) return;
    if (!focused
        || !alive
        || RustyClient.active("freecam")
        || !RustyClient.POLICY.allows("waypoint_walk")
        || ++timeout > 2400
        || Math.abs(net.minecraft.util.Mth.wrapDegrees(mc.player.getYRot() - lastYaw)) > 3) {
      cancelWalk();
      return;
    }
    BlockPos target = route.peekFirst();
    if (mc.player.position().distanceToSqr(Vec3.atBottomCenterOf(target)) < .5) {
      route.removeFirst();
      if (route.isEmpty()) {
        RustyClient.notice = "Waypoint reached.";
        return;
      }
      target = route.peekFirst();
    }
    if (!standable(target)) {
      cancelWalk();
      RustyClient.notice = "Walking stopped: route changed or unloaded.";
      return;
    }
    Vec3 delta = Vec3.atBottomCenterOf(target).subtract(mc.player.position());
    lastYaw = (float) Math.toDegrees(Math.atan2(-delta.x, delta.z));
    mc.player.setYRot(lastYaw);
  }

  private static String worldKey() {
    var mc = Minecraft.getInstance();
    String world;
    if (mc.hasSingleplayerServer())
      world =
          "local:"
              + mc.getSingleplayerServer()
                  .getWorldPath(LevelResource.ROOT)
                  .toAbsolutePath()
                  .normalize();
    else
      world =
          "server:"
              + (mc.getCurrentServer() == null
                  ? "unknown-session:" + RustyClient.scope()
                  : mc.getCurrentServer().ip.toLowerCase(Locale.ROOT));
    try {
      world =
          HexFormat.of()
              .formatHex(
                  MessageDigest.getInstance("SHA-256")
                      .digest(world.getBytes(java.nio.charset.StandardCharsets.UTF_8)));
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
    return world + "/" + mc.level.dimension().identifier();
  }

  private static List<Marker> current() {
    return store.worlds.computeIfAbsent(scope, k -> new ArrayList<>());
  }

  public static List<Marker> markers() {
    return List.copyOf(current());
  }

  public static Marker selected() {
    var list = current();
    if (list.isEmpty()) return null;
    return list.get(Math.floorMod(store.selected.getOrDefault(scope, 0), list.size()));
  }

  public static void saveMarker() {
    var p = Minecraft.getInstance().player;
    if (p == null) return;
    if (current().size() >= 64) {
      RustyClient.notice = "Maximum 64 markers in this world/dimension.";
      return;
    }
    current()
        .add(new Marker("Waypoint " + (current().size() + 1), p.getX(), p.getY(), p.getZ(), false));
    store.selected.put(scope, current().size() - 1);
    save();
    RustyClient.notice = "Waypoint saved locally.";
  }

  public static void nextMarker() {
    if (!current().isEmpty())
      store.selected.put(scope, (store.selected.getOrDefault(scope, 0) + 1) % current().size());
    save();
  }

  public static void removeMarker() {
    var m = selected();
    if (m != null) {
      var list = current();
      int index = list.indexOf(m);
      list.remove(m);
      store.selected.put(scope, Math.min(index, Math.max(0, list.size() - 1)));
    }
    save();
  }

  private static void save() {
    try {
      Files.createDirectories(path().getParent());
      Files.writeString(path(), RustyClient.GSON.toJson(store));
    } catch (Exception e) {
      RustyClient.notice = "Markers were not saved.";
      RustyClient.LOG.warn(RustyClient.notice, e);
    }
  }

  public static boolean walking() {
    return !route.isEmpty();
  }

  public static void cancelWalk() {
    route.clear();
  }

  private static boolean loaded(BlockPos p) {
    var l = Minecraft.getInstance().level;
    return l != null
        && l.getChunkSource().getChunk(p.getX() >> 4, p.getZ() >> 4, ChunkStatus.FULL, false)
            != null;
  }

  private static boolean standable(BlockPos pos) {
    var mc = Minecraft.getInstance();
    if (!loaded(pos) || !loaded(pos.below())) return false;
    Vec3 offset = Vec3.atBottomCenterOf(pos).subtract(mc.player.position());
    var box = mc.player.getBoundingBox().move(offset);
    return mc.level.noCollision(mc.player, box)
        && !mc.level.noCollision(mc.player, box.move(0, -.15, 0));
  }

  public static void startWalk() {
    cancelWalk();
    var mc = Minecraft.getInstance();
    var marker = selected();
    if (marker == null || mc.player == null || !RustyClient.POLICY.allows("waypoint_walk")) {
      RustyClient.notice = "Select a marker and obtain server movement permission first.";
      return;
    }
    BlockPos start = mc.player.blockPosition(), goal = BlockPos.containing(marker.position());
    if (start.distManhattan(goal) > 128) {
      RustyClient.notice = "Waypoint is beyond the 128-block route budget.";
      return;
    }
    record Node(BlockPos pos, int cost, int score) {}
    var open = new PriorityQueue<Node>(Comparator.comparingInt(Node::score));
    var costs = new HashMap<BlockPos, Integer>();
    var parent = new HashMap<BlockPos, BlockPos>();
    open.add(new Node(start, 0, start.distManhattan(goal)));
    costs.put(start, 0);
    int work = 0;
    BlockPos found = null;
    while (!open.isEmpty() && work++ < 4096) {
      var node = open.poll();
      if (node.cost != costs.getOrDefault(node.pos, Integer.MAX_VALUE)) continue;
      if (node.pos.distManhattan(goal) <= 1) {
        found = node.pos;
        break;
      }
      for (int[] d : new int[][] {{1, 0}, {-1, 0}, {0, 1}, {0, -1}})
        for (int dy : new int[] {0, 1, -1}) {
          BlockPos next = node.pos.offset(d[0], dy, d[1]);
          if (!standable(next)) continue;
          int cost = node.cost + 10 + Math.abs(dy) * 4;
          if (cost < costs.getOrDefault(next, Integer.MAX_VALUE)) {
            costs.put(next, cost);
            parent.put(next, node.pos);
            open.add(new Node(next, cost, cost + next.distManhattan(goal) * 10));
          }
          break;
        }
    }
    if (found == null) {
      RustyClient.notice = "No safe loaded route within the work budget.";
      return;
    }
    while (!found.equals(start)) {
      route.addFirst(found);
      found = parent.get(found);
      if (found == null) {
        cancelWalk();
        return;
      }
    }
    timeout = 0;
    lastYaw = mc.player.getYRot();
    RustyClient.notice = "Walking. Move, turn or open a menu to cancel.";
  }
}
