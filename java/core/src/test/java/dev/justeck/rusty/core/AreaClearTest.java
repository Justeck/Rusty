package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class AreaClearTest {
  @Test
  void borderRouteWallsPreserveOutsideDependenciesAndUnknownTerrain() {
    var region = new Mining.Region(new Mining.Pos(2, 1, 2), new Mining.Pos(2, 1, 2));
    var g = new Mining.Grid(new Mining.Pos(0, 0, 0), 6, 5, 5);
    for (int i = 0; i < g.size(); i++) g.set(g.position(i), Mining.AIR);
    var target = new Mining.Pos(2, 1, 2);
    var outside = new Mining.Pos(3, 1, 2);
    g.set(target, region.constrainBorder(Mining.resource(1), target, 1, 0, 0, 3, 5));
    g.set(outside, region.constrainBorder(Mining.passive(false), outside, 1, 0, 0, 3, 5));
    assertFalse(g.canMine(target, false));
    g.set(outside, region.constrainBorder(Mining.UNKNOWN, outside, 1, 0, 0, 3, 5));
    assertFalse(g.canMine(target, false));
    g.set(outside, region.constrainBorder(Mining.AIR, outside, 1, 0, 0, 3, 5));
    assertTrue(g.canMine(target, false));
    assertFalse(Mining.air(region.constrainBorder(Mining.AIR, outside, 1, 0, 0, 3, 5)));
  }

  @Test
  void liveBorderMustContainTheFullBlockVolume() {
    var inside = new Mining.Region(new Mining.Pos(-4, 1, -4), new Mining.Pos(3, 2, 3));
    var crossing = new Mining.Region(new Mining.Pos(0, 1, 3), new Mining.Pos(1, 2, 4));
    assertTrue(inside.withinBorder(-4, -4, 4, 4));
    assertFalse(crossing.withinBorder(-4, -4, 4, 4));
    assertTrue(crossing.withinBorder(-8, -8, 8, 8));
    assertFalse(inside.withinBorder(Double.NaN, -4, 4, 4));
  }

  @Test
  void protectedPartialObjectsInsideSelectionKeepTheirSupport() {
    var r = new Mining.Region(new Mining.Pos(2, 1, 2), new Mining.Pos(3, 1, 2));
    var g = new Mining.Grid(new Mining.Pos(0, 0, 0), 6, 5, 5);
    for (int i = 0; i < g.size(); i++) {
      var p = g.position(i);
      g.set(p, p.y() == 0 ? Mining.WALL : Mining.AIR);
    }
    g.set(new Mining.Pos(2, 1, 2), r.constrain(Mining.resource(1), new Mining.Pos(2, 1, 2), 1));
    g.set(new Mining.Pos(3, 1, 2), r.constrain(Mining.passive(false), new Mining.Pos(3, 1, 2), 1));
    assertFalse(g.canMine(new Mining.Pos(2, 1, 2), false));
    assertTrue(g.standable(new Mining.Pos(3, 1, 1)));
  }

  @Test
  void reversedInclusiveCornersAndLimits() {
    var r = new Mining.Region(new Mining.Pos(2, 4, -3), new Mining.Pos(-1, 2, -5));
    assertEquals(new Mining.Pos(-1, 2, -5), r.min());
    assertEquals(new Mining.Pos(2, 4, -3), r.max());
    assertEquals(36, r.volume());
    assertEquals(new Mining.Pos(2, 4, -3), r.position(35));
    assertNull(r.position(36));
    assertEquals(12, r.edges().size());
    assertThrows(
        IllegalArgumentException.class,
        () -> new Mining.Region(new Mining.Pos(0, 0, 0), new Mining.Pos(32, 0, 0)));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            new Mining.Region(
                new Mining.Pos(Integer.MIN_VALUE, 0, 0), new Mining.Pos(Integer.MAX_VALUE, 0, 0)));
  }

  @Test
  void unknownAndProtectedCellsCannotBeReportedAsCompletion() {
    var survey = new Mining.ClearSurvey();
    survey.observe(new Mining.Pos(0, 2, 0), Mining.resource(7));
    survey.observe(new Mining.Pos(0, 5, 0), Mining.resource(4));
    survey.observe(new Mining.Pos(0, 7, 0), Mining.WALL);
    survey.observe(new Mining.Pos(0, 9, 0), Mining.UNKNOWN);
    assertEquals(5, survey.highest);
    assertEquals(3, survey.remaining);
    assertEquals(1, survey.protectedCount);
    assertEquals(1, survey.unknown);
    assertFalse(survey.complete());
    survey.observe(new Mining.Pos(8, 1, 8), Mining.HAZARD);
    assertEquals(1, survey.dangerous);
  }

  @Test
  void identicalStatesOutsideTheSelectionAndLowerLayersAreNotMineable() {
    var r = new Mining.Region(new Mining.Pos(2, 1, 2), new Mining.Pos(3, 3, 3));
    var g = new Mining.Grid(new Mining.Pos(0, 0, 0), 6, 6, 6);
    for (int i = 0; i < g.size(); i++) g.set(g.position(i), Mining.AIR);
    var outside = new Mining.Pos(1, 3, 2);
    var lower = new Mining.Pos(2, 2, 2);
    g.set(outside, r.constrain(Mining.resource(4), outside, 3));
    g.set(lower, r.constrain(Mining.resource(4), lower, 3));
    assertFalse(g.canMine(outside, true));
    assertFalse(g.canMine(lower, true));
    g.set(new Mining.Pos(2, 0, 2), Mining.resourceShape(1, false));
    assertFalse(g.standable(new Mining.Pos(2, 1, 2)));
  }

  @Test
  void outsidePartialBlocksProtectTheirNeighboringSupport() {
    var r = new Mining.Region(new Mining.Pos(2, 1, 2), new Mining.Pos(2, 1, 2));
    var g = new Mining.Grid(new Mining.Pos(0, 0, 0), 5, 5, 5);
    for (int i = 0; i < g.size(); i++) g.set(g.position(i), Mining.AIR);
    g.set(new Mining.Pos(2, 1, 2), r.constrain(Mining.resource(1), new Mining.Pos(2, 1, 2), 1));
    g.set(
        new Mining.Pos(2, 2, 2),
        r.constrain(Mining.resourceShape(1, false), new Mining.Pos(2, 2, 2), 1));
    assertFalse(g.canMine(new Mining.Pos(2, 1, 2), false));
  }

  @Test
  void nullEnabledSettingsStillRecoverWithoutRestoringAutomation() {
    var cfg = new Settings();
    cfg.enabled = null;
    cfg.validate();
    assertNotNull(cfg.enabled);
    assertFalse(cfg.enabled.contains("area_clear"));
  }

  @Test
  void clearActivationIsTransientAndRequiresAllExistingServerGrants() {
    var cfg = new Settings();
    cfg.enabled.add("area_clear");
    cfg.validate();
    assertFalse(cfg.enabled.contains("area_clear"));
    var policy = new SessionPolicy();
    policy.connect("server", false);
    policy.grant("server", java.util.Set.of("movement", "assistance"));
    assertFalse(policy.allows("area_clear"));
    policy.grant("server", java.util.Set.of("view", "movement", "assistance"));
    assertTrue(policy.allows("area_clear"));
  }
}
