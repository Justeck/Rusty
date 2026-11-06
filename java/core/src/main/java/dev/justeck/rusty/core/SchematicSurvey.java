package dev.justeck.rusty.core;

import java.util.*;

/** Target-column invalidation, bounded retries, and explicit incomplete observations. */
public final class SchematicSurvey {
  public static final int MAX_RESTARTS = 8;
  private final Set<Long> columns;
  private long revision;

  public SchematicSurvey(Collection<Schematic.Pos> cells) {
    var keys = new HashSet<Long>();
    for (var p : cells) keys.add(key(p.x() >> 4, p.z() >> 4));
    columns = Set.copyOf(keys);
  }

  private static long key(int x, int z) {
    return ((long) x << 32) ^ (z & 0xffffffffL);
  }

  public long revision() {
    return revision;
  }

  public void column(int x, int z) {
    if (columns.contains(key(x, z))) revision++;
  }

  public void all() {
    revision++;
  }

  public static final class Pass {
    private long revision = Long.MIN_VALUE;
    private int restarts;
    public boolean exhausted;

    public boolean begin(long now) {
      if (revision == now) return false;
      if (revision != Long.MIN_VALUE && ++restarts > MAX_RESTARTS) exhausted = true;
      revision = now;
      return true;
    }

    public boolean coherent(long now) {
      return !exhausted && revision == now;
    }
  }

  public static final class Counts {
    public int scanned, matching, occupied, missing, manual, inaccessible, unknown, completed;

    public void observe(
        int kind, boolean manualState, boolean unreachable, boolean newlyCompleted) {
      scanned++;
      switch (kind) {
        case 0 -> unknown++;
        case 1 -> {
          matching++;
          if (newlyCompleted) completed++;
        }
        case 2 -> {
          missing++;
          if (manualState) manual++;
          if (unreachable) inaccessible++;
        }
        case 3 -> occupied++;
        default -> throw new IllegalArgumentException("Invalid observation");
      }
    }
  }
}
