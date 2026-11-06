package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class SchematicLookTest {
  static SchematicLook.Context context() {
    return new SchematicLook.Context(new Object(), new Object(), new Object(), "owned/overworld");
  }

  @Test
  void staleNorthCannotAuthorizeSouthBedOrItsCoupledHead() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(1.5, 200, 5.5);
    f.observe(c, 180, 20);
    var aim = f.aim(c, p, 0, 20, 10);
    assertEquals(SchematicLook.Gate.WAITING, f.check(aim, c, p, 10));
    assertEquals(SchematicLook.Gate.WAITING, f.check(aim, c, p, 11));
    f.observe(c, 0, 20);
    assertEquals(SchematicLook.Gate.READY, f.check(aim, c, p, 11));
  }

  @Test
  void freshConnectionWorldAndPlayerCannotReusePreviousLook() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    f.observe(c, 0, 20);
    for (var next :
        new SchematicLook.Context[] {
          new SchematicLook.Context(new Object(), c.connection(), c.world(), c.scope()),
          new SchematicLook.Context(c.player(), new Object(), c.world(), c.scope()),
          new SchematicLook.Context(c.player(), c.connection(), new Object(), c.scope()),
          new SchematicLook.Context(c.player(), c.connection(), c.world(), "owned/nether")
        }) {
      var aim = f.aim(next, p, 0, 20, 0);
      assertEquals(SchematicLook.Gate.WAITING, f.check(aim, next, p, 1));
    }
  }

  @Test
  void movedPoseOrChangedScopeInvalidatesStagedAim() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    var aim = f.aim(c, p, 0, 20, 0);
    f.observe(c, 0, 20);
    assertEquals(SchematicLook.Gate.STALE, f.check(aim, c, new SchematicLook.Pose(.01, 200, 0), 1));
    assertEquals(SchematicLook.Gate.STALE, f.check(aim, context(), p, 1));
  }

  @Test
  void timeoutAndManualCancellationNeverResume() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    var aim = f.aim(c, p, 0, 20, 0);
    assertEquals(SchematicLook.Gate.TIMED_OUT, f.check(aim, c, p, 101));
    aim.cancel();
    f.observe(c, 0, 20);
    assertEquals(SchematicLook.Gate.CANCELLED, f.check(aim, c, p, 102));
  }

  @Test
  void yawWrapAndFiniteValuesAreExplicit() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    f.observe(c, 360, 20);
    var aim = f.aim(c, p, 0, 20, 0);
    assertEquals(SchematicLook.Gate.READY, f.check(aim, c, p, 1));
    assertThrows(IllegalArgumentException.class, () -> f.aim(c, p, Float.NaN, 20, 0));
    assertThrows(IllegalArgumentException.class, () -> f.aim(c, p, 0, 91, 0));
    f.observe(c, Float.POSITIVE_INFINITY, 20);
    assertEquals(SchematicLook.Gate.WAITING, f.check(aim, c, p, 2));
  }

  @Test
  void freshProbeMustMatchStagedPoseAndAngles() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    var aim = f.aim(c, p, 0, 20, 0);
    assertTrue(aim.same(c, p, 360, 20));
    assertFalse(aim.same(c, p, 1, 20));
    assertFalse(aim.same(c, new SchematicLook.Pose(.01, 200, 0), 0, 20));
  }

  @Test
  void serverCorrectionOrDifferentAirUseCannotLeaveOldPublicationValid() {
    var f = new SchematicLook();
    var c = context();
    var p = new SchematicLook.Pose(0, 200, 0);
    f.observe(c, 180, 20);
    var aim = f.aim(c, p, 180, 20, 0);
    f.superseded(c, 0, 20);
    assertEquals(SchematicLook.Gate.WAITING, f.check(aim, c, p, 1));
    f.observe(c, 180, 20);
    f.superseded(c, -180, 20);
    assertEquals(SchematicLook.Gate.READY, f.check(aim, c, p, 2));
    f.clear();
    assertEquals(SchematicLook.Gate.WAITING, f.check(aim, c, p, 3));
  }
}
