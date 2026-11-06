package dev.justeck.rusty.core;

import java.util.*;

/** Original bounded planner. Unknown terrain never implies empty space or safe support. */
public final class Mining {
  private Mining() {}

  public static final String[] RESOURCES = {
    "Coal",
    "Iron",
    "Copper",
    "Gold",
    "Redstone",
    "Lapis",
    "Diamond",
    "Emerald",
    "Quartz",
    "Nether gold",
    "Ancient debris",
    "Custom"
  };
  private static final String[] IDS = {
    "coal_ore",
    "iron_ore",
    "copper_ore",
    "gold_ore",
    "redstone_ore",
    "lapis_ore",
    "diamond_ore",
    "emerald_ore",
    "nether_quartz_ore",
    "nether_gold_ore",
    "ancient_debris"
  };

  public static boolean matches(Settings s, String id) {
    if (s.mineResource == 11) return id.equals(s.mineCustom);
    if (s.mineResource < 0 || s.mineResource >= IDS.length) return false;
    return id.equals("minecraft:" + IDS[s.mineResource])
        || (s.mineResource < 8 && id.equals("minecraft:deepslate_" + IDS[s.mineResource]));
  }

  private static final int KNOWN = 1,
      EMPTY = 2,
      SOLID = 4,
      DIG = 8,
      ORE = 16,
      DANGER = 32,
      SUPPORT_GUARD = 64;
  public static final int UNKNOWN = 0,
      AIR = KNOWN | EMPTY,
      WALL = KNOWN | SOLID,
      HAZARD = KNOWN | DANGER;

  public static int terrain(int ticks) {
    return KNOWN | SOLID | DIG | (Math.clamp(ticks, 1, 65535) << 8);
  }

  public static int resource(int ticks) {
    return terrain(ticks) | ORE;
  }

  public static boolean air(int c) {
    return (c & EMPTY) != 0;
  }

  public static int resourceShape(int ticks, boolean full) {
    return full ? resource(ticks) : resource(ticks) & ~SOLID;
  }

  public static int passive(boolean full) {
    return KNOWN | (full ? SOLID : SUPPORT_GUARD);
  }

  public record Region(Pos min, Pos max) {
    public Region {
      var a = min;
      var b = max;
      min = new Pos(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.min(a.z, b.z));
      max = new Pos(Math.max(a.x, b.x), Math.max(a.y, b.y), Math.max(a.z, b.z));
      if ((long) max.x - min.x >= 32
          || (long) max.y - min.y >= 32
          || (long) max.z - min.z >= 32
          || min.x < -29999984
          || min.z < -29999984
          || max.x > 29999983
          || max.z > 29999983
          || min.y < -2048
          || max.y > 2047)
        throw new IllegalArgumentException(
            "Selection must fit 32 blocks per axis inside supported world bounds");
    }

    public int volume() {
      return (max.x - min.x + 1) * (max.y - min.y + 1) * (max.z - min.z + 1);
    }

    public boolean contains(Pos p) {
      return p.x >= min.x
          && p.x <= max.x
          && p.y >= min.y
          && p.y <= max.y
          && p.z >= min.z
          && p.z <= max.z;
    }

    public Pos position(int i) {
      if (i < 0 || i >= volume()) return null;
      int w = max.x - min.x + 1, d = max.z - min.z + 1;
      return min.add(i % w, i / (w * d), i / w % d);
    }

    public int constrain(int cell, Pos p, int layer) {
      if (contains(p) && p.y == layer) return cell;
      int restricted = cell & ~(DIG | ORE);
      if (!contains(p) && (cell & KNOWN) != 0 && (cell & (EMPTY | SOLID)) == 0)
        restricted |= SUPPORT_GUARD;
      return restricted;
    }

    public boolean withinBorder(double minX, double minZ, double maxX, double maxZ) {
      return Double.isFinite(minX)
          && Double.isFinite(minZ)
          && Double.isFinite(maxX)
          && Double.isFinite(maxZ)
          && min.x >= minX
          && min.z >= minZ
          && max.x + 1.0 <= maxX
          && max.z + 1.0 <= maxZ;
    }

    public int constrainBorder(
        int cell, Pos p, int layer, double minX, double minZ, double maxX, double maxZ) {
      if (air(cell) && (p.x < minX || p.z < minZ || p.x + 1.0 > maxX || p.z + 1.0 > maxZ))
        return WALL;
      return constrain(cell, p, layer);
    }

    public List<Pos[]> edges() {
      var out = new ArrayList<Pos[]>();
      int[] lo = {min.x, min.y, min.z}, hi = {max.x + 1, max.y + 1, max.z + 1};
      for (int axis = 0; axis < 3; axis++)
        for (int a = 0; a < 2; a++)
          for (int b = 0; b < 2; b++) {
            int[] from = lo.clone();
            from[(axis + 1) % 3] = a == 0 ? lo[(axis + 1) % 3] : hi[(axis + 1) % 3];
            from[(axis + 2) % 3] = b == 0 ? lo[(axis + 2) % 3] : hi[(axis + 2) % 3];
            int[] to = from.clone();
            to[axis] = hi[axis];
            out.add(new Pos[] {new Pos(from[0], from[1], from[2]), new Pos(to[0], to[1], to[2])});
          }
      return out;
    }
  }

  public static final class ClearSurvey {
    public int remaining, protectedCount, unknown, dangerous, highest = Integer.MIN_VALUE;

    public void observe(Pos p, int cell) {
      if ((cell & KNOWN) == 0) unknown++;
      else if (!air(cell)) {
        remaining++;
        if ((cell & DANGER) != 0) dangerous++;
        if ((cell & DIG) != 0) highest = Math.max(highest, p.y);
        else protectedCount++;
      }
    }

    public boolean complete() {
      return remaining == 0 && unknown == 0;
    }
  }

  public record Pos(int x, int y, int z) {
    public Pos add(int dx, int dy, int dz) {
      return new Pos(x + dx, y + dy, z + dz);
    }
  }

  private static final int[][]
      DIRS = {{1, 0, 0}, {-1, 0, 0}, {0, 1, 0}, {0, -1, 0}, {0, 0, 1}, {0, 0, -1}},
      SIDES = {{1, 0, 0}, {-1, 0, 0}, {0, 0, 1}, {0, 0, -1}};

  public record Action(boolean mine, Pos pos, boolean resource) {}

  public static boolean collectAfter(ArrayDeque<Action> actions, Pos to, boolean quota) {
    if (quota) actions.clear();
    if (to != null) {
      var walk = new Action(false, to, false);
      if (actions.isEmpty()) {
        actions.addFirst(walk);
        return true;
      }
      if (walk.equals(actions.peekFirst())) return true;
    }
    return false;
  }

  public record Plan(Pos target, List<Action> actions, int ticks, int expanded) {}

  public enum State {
    WORKING,
    FOUND,
    NO_PATH,
    LIMITED
  }

  public record Result(State state, Plan plan) {}

  private record Edge(int cost, List<Action> actions) {}

  public static final class Grid {
    final Pos origin;
    final int width, height, depth;
    final int[] cells;
    final char[] distance;
    int prepared, resources;

    public Grid(Pos origin, int width, int height, int depth) {
      long size = (long) width * height * depth;
      if (width < 1
          || width > 65
          || depth < 1
          || depth > 65
          || height < 1
          || height > 512
          || size > 2_000_000)
        throw new IllegalArgumentException("Terrain grid exceeds safety bounds");
      this.origin = origin;
      this.width = width;
      this.height = height;
      this.depth = depth;
      cells = new int[(int) size];
      distance = new char[(int) size];
      Arrays.fill(distance, Character.MAX_VALUE);
    }

    public int size() {
      return cells.length;
    }

    public Pos position(int i) {
      return origin.add(i % width, i / (width * depth), i / width % depth);
    }

    int index(Pos p) {
      long x = (long) p.x - origin.x, y = (long) p.y - origin.y, z = (long) p.z - origin.z;
      return x < 0 || y < 0 || z < 0 || x >= width || y >= height || z >= depth
          ? -1
          : (int) (x + width * (z + depth * y));
    }

    public void set(Pos p, int c) {
      int i = index(p);
      if (i >= 0) {
        resources += ((c & ORE) != 0 ? 1 : 0) - ((cells[i] & ORE) != 0 ? 1 : 0);
        cells[i] = c;
        distance[i] = (c & ORE) != 0 ? 0 : Character.MAX_VALUE;
      }
    }

    public int get(Pos p) {
      int i = index(p);
      return i < 0 ? UNKNOWN : cells[i];
    }

    public boolean prepare(int budget) {
      int end = (int) Math.min((long) prepared + Math.max(0, budget), size() * 2L);
      while (prepared < end) {
        boolean forward = prepared < size();
        int i = forward ? prepared : 2 * size() - 1 - prepared;
        int x = i % width, z = i / width % depth, y = i / (width * depth);
        if (forward) {
          if (x > 0) relax(i, i - 1);
          if (z > 0) relax(i, i - width);
          if (y > 0) relax(i, i - width * depth);
        } else {
          if (x + 1 < width) relax(i, i + 1);
          if (z + 1 < depth) relax(i, i + width);
          if (y + 1 < height) relax(i, i + width * depth);
        }
        prepared++;
      }
      return prepared == size() * 2;
    }

    private void relax(int i, int j) {
      distance[i] = (char) Math.min(distance[i], Math.min(65535, distance[j] + 1));
    }

    public boolean canMine(Pos p, boolean tunnels) {
      int c = get(p);
      if ((c & DIG) == 0 || (!tunnels && (c & ORE) == 0)) return false;
      for (var d : DIRS) {
        int n = get(p.add(d[0], d[1], d[2]));
        if ((n & KNOWN) == 0 || (n & (DANGER | SUPPORT_GUARD)) != 0) return false;
      }
      return true;
    }

    boolean support(Pos p) {
      return (get(p) & (KNOWN | SOLID | DANGER)) == (KNOWN | SOLID);
    }

    boolean sides(Pos p) {
      for (var d : SIDES)
        for (int y = 0; y < 2; y++) {
          int c = get(p.add(d[0], y, d[2]));
          if ((c & KNOWN) == 0 || (c & DANGER) != 0) return false;
        }
      return true;
    }

    public boolean standable(Pos p) {
      return air(get(p)) && air(get(p.add(0, 1, 0))) && support(p.add(0, -1, 0)) && sides(p);
    }

    public Pos collectionStep(Pos from, Pos mined) {
      var to = new Pos(mined.x, Math.min(from.y, mined.y), mined.z);
      return canWalk(from, to) ? to : null;
    }

    public boolean canWalk(Pos from, Pos to) {
      var edge = edge(from, to, false);
      return Math.abs(from.x - to.x) + Math.abs(from.z - to.z) == 1
          && Math.abs(from.y - to.y) <= 1
          && standable(from)
          && standable(to)
          && edge != null
          && edge.actions.size() == 1;
    }

    Edge edge(Pos from, Pos to, boolean tunnels) {
      if (!support(to.add(0, -1, 0))
          || !sides(to)
          || (to.y > from.y && !air(get(from.add(0, 2, 0))))) return null;
      int cost = to.y == from.y ? 5 : 8;
      var actions = new ArrayList<Action>();
      for (int y = Math.max(from.y + 1, to.y + 1); y >= to.y; y--) {
        var p = new Pos(to.x, y, to.z);
        int c = get(p);
        if (air(c)) continue;
        if (!canMine(p, tunnels)) return null;
        cost += Math.max(1, c >>> 8);
        actions.add(new Action(true, p, (c & ORE) != 0));
      }
      actions.add(new Action(false, to, false));
      return new Edge(cost, actions);
    }
  }

  public static final class Search {
    private record Entry(int estimate, int cost, int index) {}

    private record Goal(int cost, int index, Pos target, List<Action> actions) {}

    final Grid grid;
    final int start, limit;
    final boolean tunnels;
    final int[] cost, parent;
    final PriorityQueue<Entry> open =
        new PriorityQueue<>(
            Comparator.comparingInt(Entry::estimate)
                .thenComparingInt(Entry::cost)
                .thenComparingInt(Entry::index));
    int expanded;
    Goal best;
    boolean done;

    public Search(Grid grid, Pos start, boolean tunnels, int limit) {
      if (grid.prepared != grid.size() * 2 || !grid.standable(start))
        throw new IllegalArgumentException("Player is not in prepared safe terrain");
      this.grid = grid;
      this.start = grid.index(start);
      this.tunnels = tunnels;
      this.limit = limit;
      cost = new int[grid.size()];
      parent = new int[grid.size()];
      Arrays.fill(cost, Integer.MAX_VALUE);
      Arrays.fill(parent, -1);
      cost[this.start] = 0;
      if (grid.resources > 0) open.add(new Entry(heuristic(this.start), 0, this.start));
    }

    int heuristic(int i) {
      return ((Math.max(0, grid.distance[i] - 2) + 1) / 2) * 5;
    }

    Result finish() {
      done = true;
      if (best == null) return new Result(State.NO_PATH, null);
      int i = best.index;
      var edges = new ArrayList<List<Action>>();
      while (i != start) {
        int previous = parent[i];
        if (previous < 0) return new Result(State.NO_PATH, null);
        var e = grid.edge(grid.position(previous), grid.position(i), tunnels);
        if (e == null) return new Result(State.NO_PATH, null);
        edges.add(e.actions);
        i = previous;
      }
      var actions = new ArrayList<Action>();
      for (int j = edges.size() - 1; j >= 0; j--) actions.addAll(edges.get(j));
      actions.addAll(best.actions);
      var removed = new HashSet<Pos>();
      var stand = grid.position(start);
      for (var a : actions) {
        if (a.mine) {
          if (a.pos.x == stand.x && a.pos.z == stand.z) return new Result(State.NO_PATH, null);
          removed.add(a.pos);
        } else {
          if (removed.contains(a.pos.add(0, -1, 0))) return new Result(State.NO_PATH, null);
          stand = a.pos;
        }
      }
      return new Result(
          State.FOUND, new Plan(best.target, List.copyOf(actions), best.cost, expanded));
    }

    public Result advance(int budget) {
      if (done) return new Result(State.NO_PATH, null);
      for (int n = 0; n < budget; n++) {
        var entry = open.poll();
        if (entry == null) return finish();
        int i = entry.index, c = entry.cost;
        if (c != cost[i]) continue;
        if (best != null && entry.estimate >= best.cost) return finish();
        if (expanded >= limit) {
          done = true;
          return new Result(State.LIMITED, null);
        }
        expanded++;
        var p = grid.position(i);
        for (var d : SIDES)
          for (int dy = -1; dy <= 1; dy++) {
            var target = p.add(d[0], dy, d[2]);
            if ((grid.get(target) & ORE) == 0 || !grid.canMine(target, tunnels)) continue;
            var terminal = new ArrayList<Action>();
            int total = c;
            boolean allowed = true;
            for (int y = Math.max(target.y, p.y + 1); y >= target.y; y--) {
              var q = new Pos(target.x, y, target.z);
              int cell = grid.get(q);
              if (air(cell)) continue;
              if (!grid.canMine(q, tunnels)) {
                allowed = false;
                break;
              }
              total += Math.max(1, cell >>> 8);
              terminal.add(new Action(true, q, (cell & ORE) != 0));
            }
            if (allowed && (best == null || total < best.cost))
              best = new Goal(total, i, target, terminal);
          }
        for (var d : SIDES)
          for (int dy = -1; dy <= 1; dy++) {
            var q = p.add(d[0], dy, d[2]);
            int j = grid.index(q);
            if (j < 0) continue;
            var edge = grid.edge(p, q, tunnels);
            if (edge == null) continue;
            int next = c + edge.cost;
            if (next < cost[j]) {
              cost[j] = next;
              parent[j] = i;
              open.add(new Entry(next + heuristic(j), next, j));
            }
          }
      }
      return new Result(State.WORKING, null);
    }
  }
}
