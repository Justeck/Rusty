package dev.justeck.rusty.core;

import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.BooleanSupplier;

/** One request owns one nonblocking final-commit decision. */
public final class SchematicCommit {
  private static final int PENDING = 0, CANCELLED = 1, COMMITTING = 2;
  private final AtomicInteger state = new AtomicInteger(PENDING);

  public boolean cancel() {
    return state.compareAndSet(PENDING, CANCELLED) || state.get() == CANCELLED;
  }

  public boolean claim(BooleanSupplier current) {
    if (!current.getAsBoolean()) {
      cancel();
      return false;
    }
    return state.compareAndSet(PENDING, COMMITTING);
  }

  public boolean committing() {
    return state.get() == COMMITTING;
  }
}
