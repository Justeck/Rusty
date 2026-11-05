package dev.justeck.rusty.core;

import java.util.LinkedHashMap;
import java.util.Map;

public final class TargetIndex {
  private final int limit;
  private String epoch;
  private boolean partial;
  private final Map<Long, String> positions = new LinkedHashMap<>();

  public TargetIndex(int limit) {
    this.limit = Math.clamp(limit, 1, 4096);
  }

  public void begin(String epoch) {
    clear();
    this.epoch = epoch;
  }

  public void put(String epoch, long position, String id) {
    if (this.epoch == null || !this.epoch.equals(epoch)) return;
    if (positions.containsKey(position) || positions.size() < limit) positions.put(position, id);
    else partial = true;
  }

  public void remove(long position) {
    positions.remove(position);
  }

  public void clear() {
    positions.clear();
    partial = false;
    epoch = null;
  }

  public int size() {
    return positions.size();
  }

  public boolean partial() {
    return partial;
  }

  public Map<Long, String> snapshot() {
    return Map.copyOf(positions);
  }
}
