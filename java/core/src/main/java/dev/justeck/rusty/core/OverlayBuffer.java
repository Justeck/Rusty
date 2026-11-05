package dev.justeck.rusty.core;

import java.util.ArrayList;
import java.util.List;

/** Bounded geometry collected before publishing an immutable render snapshot. */
public final class OverlayBuffer<T> {
  public static final int MAIN_LIMIT = 768, ENTITY_TRACER_LIMIT = 128;
  private final List<T> entries = new ArrayList<>();
  private final List<T> entityTracers = new ArrayList<>();

  public void add(T value) {
    if (entries.size() < MAIN_LIMIT) entries.add(value);
  }

  public void addEntityTracer(T value) {
    if (entityTracers.size() < ENTITY_TRACER_LIMIT) entityTracers.add(value);
  }

  public List<T> snapshot() {
    var combined = new ArrayList<T>(entries.size() + entityTracers.size());
    combined.addAll(entries);
    combined.addAll(entityTracers);
    return List.copyOf(combined);
  }
}
