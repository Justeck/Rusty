package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class SchematicSafetyTest {
  @Test
  void predictedInventorySwapIsNotAReceipt() {
    var transfer = new SchematicSafety.Transfer<>(11, 36, "stone64", "pickaxe");
    assertFalse(transfer.confirmed());
    transfer.received(11, "pickaxe");
    assertFalse(transfer.confirmed());
    transfer.received(36, "stone64");
    assertTrue(transfer.confirmed());
  }

  @Test
  void unrelatedOrRejectedInventoryCannotConfirm() {
    var transfer = new SchematicSafety.Transfer<>(11, 36, "stone64", "pickaxe");
    transfer.received(4, "stone64");
    assertFalse(transfer.confirmed());
    transfer.received(11, "stone64");
    assertTrue(transfer.rejected());
  }

  @Test
  void onlyMonotonicMaterialAccumulationIsAllowed() {
    assertTrue(
        SchematicSafety.progress(
            Schematic.State.parse("minecraft:stone_slab[type=double,waterlogged=false]"),
            Schematic.State.parse("minecraft:stone_slab[type=bottom,waterlogged=false]")));
    assertTrue(
        SchematicSafety.progress(
            Schematic.State.parse("minecraft:snow[layers=4]"),
            Schematic.State.parse("minecraft:snow[layers=2]")));
    assertFalse(
        SchematicSafety.progress(
            Schematic.State.parse("minecraft:snow[layers=2]"),
            Schematic.State.parse("minecraft:snow[layers=4]")));
    assertFalse(
        SchematicSafety.progress(
            Schematic.State.parse("minecraft:oak_stairs[facing=north]"),
            Schematic.State.parse("minecraft:oak_stairs[facing=south]")));
  }

  @Test
  void progressRejectsUntrustedCounts() {
    for (String bad : new String[] {"wat", "-1", "9", "2147483648"})
      assertFalse(
          SchematicSafety.progress(
              Schematic.State.parse("minecraft:snow[layers=" + bad + "]"),
              Schematic.State.parse("minecraft:snow[layers=1]")));
  }

  @Test
  void constructionSafetyDefaultsAreResettableAndClamped() {
    var settings = new Settings();
    assertEquals(12, settings.plannerHealth);
    assertEquals(10, settings.plannerFood);
    settings.plannerHealth = 99;
    settings.plannerFood = -3;
    settings.validate();
    assertEquals(20, settings.plannerHealth);
    assertEquals(0, settings.plannerFood);
  }

  @Test
  void safetyStopsAreExplicitAndNeverRearm() {
    assertEquals(
        "Health below 12/20", SchematicSafety.stopReason(false, true, true, 11, 20, 20, 0));
    assertEquals("Damage received", SchematicSafety.stopReason(false, true, true, 19, 20, 20, 0));
    assertEquals("Food below 10/20", SchematicSafety.stopReason(false, true, true, 20, 20, 9, 0));
    assertEquals("Manual input", SchematicSafety.stopReason(true, true, true, 20, 20, 20, 0));
    assertEquals(
        "Permission unavailable", SchematicSafety.stopReason(false, false, true, 20, 20, 20, 0));
    assertEquals(
        "Menu or focus changed", SchematicSafety.stopReason(false, true, false, 20, 20, 20, 0));
    assertEquals(
        "30-minute deadline", SchematicSafety.stopReason(false, true, true, 20, 20, 20, 36001));
    assertNull(SchematicSafety.stopReason(false, true, true, 20, 20, 20, 1));
  }

  @Test
  void configuredHealthAndFoodFloorsRemainIndependentOfDamage() {
    assertEquals(
        "Health below 14/20", SchematicSafety.stopReason(false, true, true, 13, 13, 20, 0, 14, 10));
    assertEquals(
        "Food below 15/20", SchematicSafety.stopReason(false, true, true, 20, 20, 14, 0, 12, 15));
    assertEquals(
        "Damage received", SchematicSafety.stopReason(false, true, true, 19, 20, 20, 0, 0, 0));
    assertNull(SchematicSafety.stopReason(false, true, true, 14, 14, 15, 0, 14, 15));
  }

  @Test
  void processingFenceStillRequiresBothExactStacks() {
    var t = new SchematicSafety.Transfer<>(11, 36, "stone64", "pickaxe");
    assertTrue(t.matches("pickaxe", "stone64"));
    assertFalse(t.matches("stone64", "pickaxe"));
    assertFalse(t.matches("air", "stone64"));
    t.received(36, "air");
    assertFalse(t.matches("pickaxe", "stone64"));
  }

  @Test
  void plannedCoupledSecondariesArePendingRatherThanManualOrFreeStanding() {
    var upper =
        new Schematic.Cell(
            new Schematic.Pos(0, 1, 0),
            Schematic.State.parse(
                "minecraft:oak_door[facing=east,half=upper,hinge=left,open=false,powered=false]"));
    var lower =
        Schematic.State.parse(
            "minecraft:oak_door[facing=east,half=lower,hinge=left,open=false,powered=false]");
    assertTrue(
        SchematicSafety.coupledCovered(
            upper, p -> p.equals(new Schematic.Pos(0, 0, 0)) ? lower : null));
    assertFalse(SchematicSafety.coupledCovered(upper, p -> null));
    assertFalse(
        SchematicSafety.coupledCovered(
            upper,
            p ->
                Schematic.State.parse(
                    "minecraft:oak_door[facing=west,half=lower,hinge=left,open=false,powered=false]")));
    var head =
        new Schematic.Cell(
            new Schematic.Pos(2, 0, 7),
            Schematic.State.parse("minecraft:red_bed[facing=south,occupied=false,part=head]"));
    assertTrue(
        SchematicSafety.coupledCovered(
            head,
            p ->
                p.equals(new Schematic.Pos(2, 0, 6))
                    ? Schematic.State.parse(
                        "minecraft:red_bed[facing=south,occupied=false,part=foot]")
                    : null));
    assertFalse(SchematicSafety.coupledCovered(head, p -> null));
  }
}
