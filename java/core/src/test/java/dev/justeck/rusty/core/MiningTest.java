package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class MiningTest {
  Mining.Grid room() {
    var g = new Mining.Grid(new Mining.Pos(0, 0, 0), 9, 5, 7);
    for (int i = 0; i < g.size(); i++) {
      var p = g.position(i);
      g.set(p, p.y() == 0 ? Mining.WALL : Mining.AIR);
    }
    return g;
  }

  Mining.Result solve(Mining.Grid g, boolean dig, int limit) {
    while (!g.prepare(17)) {}
    var s = new Mining.Search(g, new Mining.Pos(1, 1, 3), dig, limit);
    Mining.Result r;
    do {
      r = s.advance(11);
    } while (r.state() == Mining.State.WORKING);
    return r;
  }

  @Test
  void resourceSettingsAreTransientAndValidated() {
    var s = new Settings();
    assertTrue(Mining.matches(s, "minecraft:deepslate_diamond_ore"));
    assertFalse(Mining.matches(s, "minecraft:gold_ore"));
    s.enabled.add("auto_mine");
    s.validate();
    assertFalse(s.enabled.contains("auto_mine"));
    assertFalse(new SessionPolicy().allows("auto_mine"));
    var policy = new SessionPolicy();
    policy.connect("server", false);
    policy.grant("server", java.util.Set.of("movement", "assistance"));
    assertFalse(policy.allows("auto_mine"));
    policy.grant("server", java.util.Set.of("movement", "assistance", "view"));
    assertTrue(policy.allows("auto_mine"));
  }

  @Test
  void routesUseWalkingAndMiningCosts() {
    var g = room();
    g.set(new Mining.Pos(6, 1, 3), Mining.resource(40));
    g.set(new Mining.Pos(3, 1, 3), Mining.terrain(300));
    g.set(new Mining.Pos(3, 2, 3), Mining.terrain(300));
    var p = solve(g, true, 32768).plan();
    assertNotNull(p);
    assertTrue(
        p.actions().stream().noneMatch(a -> a.mine() && a.pos().x() == 3 && a.pos().z() == 3));
    g = room();
    g.set(new Mining.Pos(3, 1, 3), Mining.resource(500));
    g.set(new Mining.Pos(6, 1, 3), Mining.resource(20));
    assertEquals(new Mining.Pos(6, 1, 3), solve(g, true, 32768).plan().target());
  }

  @Test
  void tunnellingAndHazardsAreConservative() {
    var g = room();
    g.set(new Mining.Pos(6, 1, 3), Mining.resource(40));
    var p = solve(g, false, 32768).plan();
    assertNotNull(p);
    assertTrue(p.actions().stream().noneMatch(a -> a.mine() && !a.resource()));
    for (int hazard : new int[] {Mining.HAZARD, Mining.UNKNOWN}) {
      g = room();
      g.set(new Mining.Pos(5, 1, 3), Mining.resource(20));
      g.set(new Mining.Pos(5, 2, 3), hazard);
      assertNull(solve(g, true, 32768).plan());
    }
    g = room();
    g.set(new Mining.Pos(7, 1, 5), Mining.resource(30));
    assertEquals(Mining.State.LIMITED, solve(g, true, 1).state());
    assertThrows(
        IllegalArgumentException.class,
        () -> new Mining.Grid(new Mining.Pos(0, 0, 0), 65, 512, 65));
  }

  @Test
  void collectionNeverInvalidatesDescendingClearance() {
    var below = new Mining.Pos(2, 0, 3);
    var to = new Mining.Pos(2, 1, 3);
    var q =
        new java.util.ArrayDeque<Mining.Action>(
            java.util.List.of(
                new Mining.Action(true, below, false), new Mining.Action(false, below, false)));
    var original = new java.util.ArrayList<>(q);
    assertFalse(Mining.collectAfter(q, to, false));
    assertEquals(original, new java.util.ArrayList<>(q));
    assertTrue(Mining.collectAfter(q, to, true));
    assertEquals(
        java.util.List.of(new Mining.Action(false, to, false)), new java.util.ArrayList<>(q));
    assertTrue(Mining.collectAfter(q, to, false));
    assertEquals(1, q.size());
  }
}
