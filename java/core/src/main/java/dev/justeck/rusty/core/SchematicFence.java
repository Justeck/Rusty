package dev.justeck.rusty.core;

/**
 * Every normal stats request is counted, including older requests from the player's Stats screen.
 */
public final class SchematicFence {
  private long sent, received;
  private boolean tainted;

  public synchronized long sent() {
    return ++sent;
  }

  public synchronized void received() {
    if (received >= sent) tainted = true;
    else received++;
  }

  public synchronized boolean complete(long token) {
    return !tainted && token > 0 && token <= sent && received >= token;
  }

  public synchronized boolean tainted() {
    return tainted;
  }

  public synchronized long latest() {
    return sent;
  }
}
