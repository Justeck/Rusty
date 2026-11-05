package dev.justeck.rusty.core;

public final class Activation {
  private boolean previous, active;

  public boolean sampleQueued(boolean down, boolean hold, boolean focused, int presses) {
    if (hold || !focused) return sample(down, hold, focused);
    if (presses > 0) {
      if ((presses & 1) != 0) active = !active;
      previous = down;
      return active;
    }
    return sample(down, false, true);
  }

  public boolean sample(boolean down, boolean hold, boolean focused) {
    if (!focused) {
      previous = down;
      if (hold) active = false;
      return active;
    }
    if (hold) active = down;
    else if (down && !previous) active = !active;
    previous = down;
    return active;
  }

  public void set(boolean active) {
    this.active = active;
  }

  public void contextChanged(boolean requested, boolean down) {
    active = requested;
    previous = down;
  }

  public void reset() {
    active = false;
    previous = false;
  }
}
