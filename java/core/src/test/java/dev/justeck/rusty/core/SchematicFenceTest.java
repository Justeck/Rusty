package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class SchematicFenceTest {
  @Test
  void staleStatsScreenResponseCannotSatisfyNewInventoryFence() {
    var f = new SchematicFence();
    long older = f.sent(), transfer = f.sent();
    assertNotEquals(older, transfer);
    f.received();
    assertFalse(f.complete(transfer));
    f.received();
    assertTrue(f.complete(transfer));
  }

  @Test
  void predictionAndOutgoingRequestNeverAcknowledgeProcessing() {
    var f = new SchematicFence();
    long token = f.sent();
    assertFalse(f.complete(token));
    assertFalse(f.complete(0));
  }

  @Test
  void unsolicitedResponseTaintsConnectionInsteadOfAuthorizingFutureSwaps() {
    var f = new SchematicFence();
    f.received();
    long token = f.sent();
    f.received();
    assertTrue(f.tainted());
    assertFalse(f.complete(token));
  }

  @Test
  void freshConnectionCannotConsumeOldConnectionAcknowledgment() {
    var old = new SchematicFence();
    long token = old.sent();
    old.received();
    var fresh = new SchematicFence();
    assertFalse(fresh.complete(token));
  }
}
